use anchor_lang::prelude::*;
use anchor_lang::system_program::Transfer;
use vestinglib::GetReleasableAmountParams;

use crate::account_data::Grant;
use crate::utils::{get_vesting_instance, GrantInputParams, GrantStateParams};

#[derive(Accounts)]
pub struct WithdrawGrant<'info> {
    #[account(mut)]
    employee: Signer<'info>,
    /// CHECK: The account should just be the public key of whoever we want to give the grant to
    employer: AccountInfo<'info>,

    #[account(
        mut,
        seeds = [b"grant_account", employer.key().as_ref(), employee.key().as_ref()],
        bump = grant_account.bump,
        constraint = grant_account.initialized == true,
        constraint = grant_account.revoked == false,
    )]
    grant_account: Account<'info, Grant>,

    #[account(
        mut,
        seeds = [b"grant_custody", employer.key().as_ref(), employee.key().as_ref()],
        bump = grant_account.grant_custody_bump,
        constraint = grant_custody.key() == grant_account.grant_custody,
    )]
    /// CHECK: The account is a PDA
    grant_custody: AccountInfo<'info>,
    system_program: Program<'info, System>,
}

impl<'info> WithdrawGrant<'info> {
    pub fn system_program_context<T: ToAccountMetas + ToAccountInfos<'info>>(
        &self,
        data: T,
    ) -> CpiContext<'_, '_, '_, 'info, T> {
        CpiContext::new(self.system_program.to_account_info(), data)
    }

    pub fn handle(&mut self) -> Result<()> {
        let vesting = get_vesting_instance(
            &self.grant_account.params,
            GrantStateParams {
                revoked: self.grant_account.revoked,
                already_issued_token_amount: self.grant_account.already_issued_token_amount,
            },
        )?;
        let clock = Clock::get()?;
        let releasable_amount = vesting
            .get_releasable_amount(&GetReleasableAmountParams {
                current_time_unix: clock.unix_timestamp as u64,
            })
            .unwrap();
        msg!("Releasable amount: {}", releasable_amount);
        if releasable_amount > 0 {
            anchor_lang::system_program::transfer(
                self.system_program_context(Transfer {
                    from: self.grant_custody.to_account_info(),
                    to: self.employee.to_account_info(),
                })
                .with_signer(&[&[
                    b"grant_custody",
                    self.employer.key().as_ref(),
                    self.employee.key().as_ref(),
                    &[self.grant_account.grant_custody_bump],
                ]]),
                releasable_amount,
            )?;

            let data = &mut self.grant_account;
            data.already_issued_token_amount += releasable_amount;
        }
        Ok(())
    }
}
