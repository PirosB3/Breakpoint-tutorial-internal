use anchor_lang::prelude::*;
use anchor_lang::system_program::Transfer;
use vestinglib::GetReleasableAmountParams;

use crate::account_data::Grant;
use crate::utils::{get_vesting_instance, GrantStateParams};

#[derive(Accounts)]
pub struct WithdrawGrant<'info> {
    #[account(mut)]
    employee: Signer<'info>,
    /// CHECK: The account should just be the public key of whoever we want to give the grant to
    employer: AccountInfo<'info>,

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

/// Is called by the employee whenever they want to withdraw the amount of SOL
/// that is vested.
/// This instruction loads the grant from the stored configuration and transfers
/// all freeable (vested) SOL to the employee.
impl<'info> WithdrawGrant<'info> {
    fn system_program_context<T: ToAccountMetas + ToAccountInfos<'info>>(
        &self,
        data: T,
    ) -> CpiContext<'_, '_, '_, 'info, T> {
        CpiContext::new(self.system_program.to_account_info(), data)
    }

    pub fn handle(&mut self) -> Result<()> {
        // Load vesting instance from the internal state.
        let vesting = get_vesting_instance(
            &self.grant.params,
            GrantStateParams {
                revoked: self.grant.revoked,
                already_issued_token_amount: self.grant.already_issued_token_amount,
            },
        )?;

        // Compute the current releasable amount
        //
        // Example: Total grant is 4000 SOL
        // 1/4 is vested
        //
        let clock = Clock::get()?;
        let releasable_amount = vesting
            .get_releasable_amount(&GetReleasableAmountParams {
                current_time_unix: clock.unix_timestamp as u64,
            })
            .unwrap();
        msg!("Releasable amount: {}", releasable_amount);
        // Before a grant is permanently terminated, we force the employee
        // to withdraw all the remaining freable (vested) tokens - if any exist.
        if releasable_amount > 0 {
            // In this transfer, we must pass in Signer Seeds - because funds are going from the Grat Custody
            // To the employee - and Grant Custody is a PDA.
            //
            // Grant Custody -> 1000 SOL -> Employee
            anchor_lang::system_program::transfer(
                self.system_program_context(Transfer {
                    from: self.grant_custody.to_account_info(),
                    to: self.employee.to_account_info(),
                })
                .with_signer(&[&[
                    b"grant_custody",
                    self.employer.key().as_ref(),
                    self.employee.key().as_ref(),
                    &[self.grant.grant_custody_bump],
                ]]),
                releasable_amount,
            )?;

            // Transfer was successful, update persistent state to account for the funds already released.
            let data = &mut self.grant;
            data.already_issued_token_amount += releasable_amount;
        }
        Ok(())
    }
}
