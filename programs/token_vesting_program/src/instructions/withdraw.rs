use anchor_lang::prelude::*;
use anchor_spl::token::{TokenAccount, Token, Transfer, transfer};
use vestinglib::GetReleasableAmountParams;

use crate::account_data::Grant;
use crate::utils::{get_vesting_instance, GrantStateParams};

#[derive(Accounts)]
pub struct WithdrawGrant<'info> {
    // external accounts section
    // 👇 👇 👇 👇 👇
    employee: Signer<'info>,
    /// CHECK: account is not mutable and does not contain state
    employer: AccountInfo<'info>,
    #[account(mut, token::mint=grant.mint, token::authority=employee)]
    employee_account: Account<'info, TokenAccount>,

    // PDAs section
    // 👇 👇 👇 👇 👇
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
    #[account(
        token::mint=grant.mint,
        token::authority=escrow_authority,
        seeds = [b"tokens", grant.key().as_ref()],
        bump = grant.bumps.escrow_token_account
    )]
    escrow_token_account: Account<'info, TokenAccount>,

    // Programs
    token_program: Program<'info, Token>,
}

/// Is called by the employee whenever they want to withdraw the amount of SOL
/// that is vested.
/// This instruction loads the grant from the stored configuration and transfers
/// all freeable (vested) SOL to the employee.
impl<'info> WithdrawGrant<'info> {
    fn token_program_context<T: ToAccountMetas + ToAccountInfos<'info>>(
        &self,
        data: T,
    ) -> CpiContext<'_, '_, '_, 'info, T> {
        CpiContext::new(self.token_program.to_account_info(), data)
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
            transfer(
                self.token_program_context(Transfer {
                    from: self.escrow_token_account.to_account_info(),
                    to: self.employee_account.to_account_info(),
                    authority: self.escrow_authority.to_account_info(),
                })
                .with_signer(&[&[
                    b"grant_custody",
                    self.employer.key().as_ref(),
                    self.employee.key().as_ref(),
                    &[self.grant.bumps.escrow_authority],
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
