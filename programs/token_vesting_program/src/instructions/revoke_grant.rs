use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, TokenAccount, Token, Transfer, transfer};
use vestinglib::GetReleasableAmountParams;

use crate::account_data::Grant;
use crate::utils::{get_vesting_instance, GrantStateParams};

#[derive(Accounts)]
pub struct RevokeGrant<'info> {
    employer: Signer<'info>,
    /// CHECK: account is not mutable and does not contain state
    employee: AccountInfo<'info>,

    #[account(constraint = mint.is_initialized == true)]
    mint: Account<'info, Mint>,
    #[account(mut, token::mint=mint, token::authority=employer)]
    employer_account: Account<'info, TokenAccount>,
    #[account(mut, token::mint=mint, token::authority=employee)]
    employee_account: Account<'info, TokenAccount>,

    // State accounts (created and owned by this program)
    #[account(
        seeds = [b"grant", employer.key().as_ref(), employee.key().as_ref()],
        bump = grant.bumps.grant,
        constraint = grant.initialized == true,
        constraint = grant.revoked == false,
    )]
    grant: Account<'info, Grant>,
    #[account(
        seeds = [b"authority", grant.key().as_ref()],
        bump = grant.bumps.escrow_authority
    )]
    /// CHECK: The account is a PDA and does not read/write data
    escrow_authority: AccountInfo<'info>,

    // Token accounts
    #[account(
        token::mint=mint,
        token::authority=escrow_authority,
        seeds = [b"tokens", grant.key().as_ref()],
        bump = grant.bumps.escrow_token_account
    )]
    escrow_token_account: Account<'info, TokenAccount>,

    // Programs
    token_program: Program<'info, Token>,
}

/// This instruction is called by the employer when an employee leaves the companty (and the grant is revoked).
/// When this is called, we pay out all releasable amount to the employee, and refund the rest back to th employer.
impl<'info> RevokeGrant<'info> {
    fn token_program_context<T: ToAccountMetas + ToAccountInfos<'info>>(
        &self,
        data: T,
    ) -> CpiContext<'_, '_, '_, 'info, T> {
        CpiContext::new(self.token_program.to_account_info(), data)
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
                from: self.escrow_token_account.to_account_info(),
                to: self.employee_account.to_account_info(),
                authority: self.escrow_authority.to_account_info(),
            };
            transfer(
                self.token_program_context(release_to_employee)
                    .with_signer(&[&[
                        b"authority",
                        self.grant.key().as_ref(),
                        &[self.grant.bumps.escrow_authority],
                    ]]),
                releasable_amount,
            )?;
            let data = &mut self.grant;
            data.already_issued_token_amount += releasable_amount;
        }

        // Compute how much of the remaining grant is stil in the escrow
        // account (grant custody).
        // Send all the remaining amount back to employer
        self.escrow_token_account.reload()?;
        let amount_to_send_back = self.escrow_token_account.amount;
        msg!("Sending back {} to employer", amount_to_send_back);
        let send_back_to_employer = Transfer {
            from: self.escrow_token_account.to_account_info(),
            to: self.employer_account.to_account_info(),
            authority: self.escrow_authority.to_account_info(),
        };
        transfer(
            self.token_program_context(send_back_to_employer)
                .with_signer(&[&[
                b"authority",
                self.grant.key().as_ref(),
                &[self.grant.bumps.escrow_authority],
            ]]),
            amount_to_send_back,
        )?;

        // Mark grant as revoked, so this function cannot be called again.
        let data = &mut self.grant;
        data.revoked = true;
        Ok(())
    }
}
