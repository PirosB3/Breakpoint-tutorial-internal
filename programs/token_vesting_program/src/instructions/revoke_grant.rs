use anchor_lang::prelude::*;
use anchor_lang::system_program::Transfer;
use vestinglib::GetReleasableAmountParams;

use crate::account_data::Grant;
use crate::utils::{get_vesting_instance, GrantStateParams};

#[derive(Accounts)]
pub struct RevokeGrant<'info> {
    #[account(mut)]
    employer: Signer<'info>,
    /// CHECK: The account should just be the public key of whoever we want to give the grant to
    #[account(mut)]
    employee: AccountInfo<'info>,

    #[account(
        mut,
        seeds = [b"grant_account", employer.key().as_ref(), employee.key().as_ref()],
        bump = grant.bump,
        constraint = grant.initialized == true,
        constraint = grant.revoked == false,
    )]
    grant: Account<'info, Grant>,

    #[account(
        mut,
        seeds = [b"grant_custody", employer.key().as_ref(), employee.key().as_ref()],
        bump = grant.grant_custody_bump,
    )]
    /// CHECK: The account is a PDA
    grant_custody: AccountInfo<'info>,
    system_program: Program<'info, System>,
}

/// This instruction is called by the employer when they leave the companty (and the grant is complete).
/// When this is called, we pay out all releasable amount to the employee, and refund the rest back to th employer.
impl<'info> RevokeGrant<'info> {
    fn system_program_context<T: ToAccountMetas + ToAccountInfos<'info>>(
        &self,
        data: T,
    ) -> CpiContext<'_, '_, '_, 'info, T> {
        CpiContext::new(self.system_program.to_account_info(), data)
    }

    pub fn handle(&mut self) -> Result<()> {
        // This first step is simular to "withdraw" - compute releasable amount and transfer it to the employee.
        let vesting = get_vesting_instance(
            &self.grant.params,
            GrantStateParams {
                revoked: self.grant.revoked,
                already_issued_token_amount: self.grant.already_issued_token_amount,
            },
        )?;
        let clock = Clock::get()?;
        let releasable_amount = vesting
            .get_releasable_amount(&GetReleasableAmountParams {
                current_time_unix: clock.unix_timestamp as u64,
            })
            .unwrap();
        if releasable_amount > 0 {
            msg!("Sending remaining {} to employee", releasable_amount);
            let release_to_employee = Transfer {
                from: self.grant_custody.to_account_info(),
                to: self.employee.to_account_info(),
            };
            anchor_lang::system_program::transfer(
                self.system_program_context(release_to_employee)
                    .with_signer(&[&[
                        b"grant_custody",
                        self.employer.key().as_ref(),
                        self.employee.key().as_ref(),
                        &[self.grant.grant_custody_bump],
                    ]]),
                releasable_amount,
            )?;
            let data = &mut self.grant;
            data.already_issued_token_amount += releasable_amount;
        }

        // Compute how much of the remaining grant is stil in the escrow
        // account (grant custody).
        // Send all the remaining amount back to employer
        let amount_to_send_back = self.grant_custody.lamports();
        msg!("Sending back {} to employer", amount_to_send_back);
        let send_back_to_employer = Transfer {
            from: self.grant_custody.to_account_info(),
            to: self.employer.to_account_info(),
        };
        anchor_lang::system_program::transfer(
            self.system_program_context(send_back_to_employer)
                .with_signer(&[&[
                    b"grant_custody",
                    self.employer.key().as_ref(),
                    self.employee.key().as_ref(),
                    &[self.grant.grant_custody_bump],
                ]]),
            amount_to_send_back,
        )?;

        // Mark grant as revoked, so this function cannot be called again.
        let data = &mut self.grant;
        data.revoked = true;
        Ok(())
    }
}
