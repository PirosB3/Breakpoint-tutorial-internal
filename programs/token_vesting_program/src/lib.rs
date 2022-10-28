use anchor_lang::prelude::*;
use anchor_lang::system_program::Transfer;

mod account_data;
mod instructions;
mod pda;
mod utils;

use account_data::Grant;
use instructions::initialize_grant::*;
use instructions::withdraw::*;
use utils::{get_vesting_instance, GrantInputParams, GrantStateParams};
use vestinglib::GetReleasableAmountParams;

declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

#[error_code]
pub enum TokenVestingError {
    #[msg("Grant input parameters invalid")]
    ParamsInvalid,
}

#[derive(Clone, Copy, Debug, AnchorDeserialize, AnchorSerialize)]
pub struct Res {
    pub releasable_amount: u64,
}

#[program]
pub mod token_vesting_program {
    use super::*;

    pub fn revoke(ctx: Context<RevokeGrant>) -> Result<()> {
        let vesting = get_vesting_instance(
            &ctx.accounts.grant_account.params,
            GrantStateParams {
                revoked: ctx.accounts.grant_account.revoked,
                already_issued_token_amount: ctx.accounts.grant_account.already_issued_token_amount,
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
                from: ctx.accounts.grant_custody.to_account_info(),
                to: ctx.accounts.employee.to_account_info(),
            };
            anchor_lang::system_program::transfer(
                ctx.accounts
                    .system_program_context(release_to_employee)
                    .with_signer(grant_custody_seeds!(ctx)),
                releasable_amount,
            )?;
            let data = &mut ctx.accounts.grant_account;
            data.already_issued_token_amount += releasable_amount;
        }

        // Compute how much the account has
        let amount_to_send_back = ctx.accounts.grant_custody.lamports();
        msg!("Sending back {} to employer", amount_to_send_back);
        let send_back_to_employer = Transfer {
            from: ctx.accounts.grant_custody.to_account_info(),
            to: ctx.accounts.employer.to_account_info(),
        };
        anchor_lang::system_program::transfer(
            ctx.accounts
                .system_program_context(send_back_to_employer)
                .with_signer(grant_custody_seeds!(ctx)),
            amount_to_send_back,
        )?;
        let data = &mut ctx.accounts.grant_account;
        data.revoked = true;
        Ok(())
    }

    pub fn initialize(ctx: Context<InitializeNewGrant>, params: GrantInputParams) -> Result<()> {
        ctx.accounts.handle(params, &ctx.bumps)?;
        Ok(())
    }

    pub fn withdraw(ctx: Context<WithdrawGrant>, delta: u8) -> Result<()> {
        ctx.accounts.handle()
    }
}

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
        bump = grant_account.bump,
        constraint = grant_custody.key() == grant_account.grant_custody,
        constraint = grant_account.initialized == true,
        constraint = grant_account.revoked == false,
    )]
    grant_account: Account<'info, Grant>,

    #[account(
        mut,
        seeds = [b"grant_custody", employer.key().as_ref(), employee.key().as_ref()], bump
    )]
    /// CHECK: The account is a PDA
    grant_custody: AccountInfo<'info>,
    system_program: Program<'info, System>,
}

impl<'info> RevokeGrant<'info> {
    pub fn system_program_context<T: ToAccountMetas + ToAccountInfos<'info>>(
        &self,
        data: T,
    ) -> CpiContext<'_, '_, '_, 'info, T> {
        CpiContext::new(self.system_program.to_account_info(), data)
    }
}
