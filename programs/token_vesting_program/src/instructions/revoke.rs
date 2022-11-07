use anchor_lang::prelude::*;
use anchor_spl::token::{transfer, Token, TokenAccount, Transfer};
use vestinglib::GetReleasableAmountParams;

use crate::account_data::Grant;
use crate::utils::{get_vesting_instance, GrantStateParams};

#[derive(Accounts)]
pub struct RevokeGrant<'info> {
    // External accounts section
    // 👇 👇 👇 👇 👇
    #[account(address = grant.employer)]
    employer: Signer<'info>,
    #[account(address = grant.employee)]
    employee: SystemAccount<'info>,
    #[account(mut, token::mint=grant.mint, token::authority=employer)]
    employer_account: Account<'info, TokenAccount>,
    #[account(mut, token::mint=grant.mint, token::authority=employee)]
    employee_account: Account<'info, TokenAccount>,

    // PDAs section
    // 👇 👇 👇 👇 👇
    #[account(
        mut,
        seeds = [b"grant".as_ref(), employer.key().as_ref(), employee.key().as_ref()],
        bump = grant.bumps.grant,
        constraint = grant.initialized == true,
        constraint = grant.revoked == false,
    )]
    // Grant account allows Vesting program to read/write state
    grant: Account<'info, Grant>,
    #[account(
        seeds = [b"authority".as_ref(), grant.key().as_ref()],
        bump = grant.bumps.escrow_authority
    )]
    // Escrow token account authority a system account PDA
    // Only this program can sign using that account
    escrow_authority: SystemAccount<'info>,
    #[account(
        mut,
        token::mint=grant.mint,
        token::authority=escrow_authority,
        seeds = [b"tokens".as_ref(), grant.key().as_ref()],
        bump = grant.bumps.escrow_token_account
    )]
    escrow_token_account: Account<'info, TokenAccount>,

    // Programs section
    // 👇 👇 👇 👇 👇
    token_program: Program<'info, Token>,
}

impl<'info> RevokeGrant<'info> {
    fn token_program_context<T: ToAccountMetas + ToAccountInfos<'info>>(
        &self,
        data: T,
    ) -> CpiContext<'_, '_, '_, 'info, T> {
        CpiContext::new(self.token_program.to_account_info(), data)
    }
}

/// This instruction is called by the employer when an employee leaves the company (and the grant is revoked).
///
/// Goal of instruction
/// 1) Pay out all releasable amount to the employee
/// 2) Refund the rest back to th employer
/// 3) Update state
pub fn revoke(ctx: Context<RevokeGrant>) -> Result<()> {
    // This first step is similar to "withdraw" - compute releasable amount and transfer it to the employee.
    let vesting = get_vesting_instance(
        &ctx.accounts.grant.params,
        GrantStateParams {
            revoked: ctx.accounts.grant.revoked,
            already_issued_token_amount: ctx.accounts.grant.already_issued_token_amount,
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
            from: ctx.accounts.escrow_token_account.to_account_info(),
            to: ctx.accounts.employee_account.to_account_info(),
            authority: ctx.accounts.escrow_authority.to_account_info(),
        };
        transfer(
            ctx.accounts
                .token_program_context(release_to_employee)
                .with_signer(&[&[
                    b"authority",
                    ctx.accounts.grant.key().as_ref(),
                    &[ctx.accounts.grant.bumps.escrow_authority],
                ]]),
            releasable_amount,
        )?;
        let data = &mut ctx.accounts.grant;
        data.already_issued_token_amount += releasable_amount;
    }

    // Compute how much of the remaining grant is still in the escrow
    // account (grant custody).
    // Send all the remaining amount back to employer
    ctx.accounts.escrow_token_account.reload()?;
    let amount_to_send_back = ctx.accounts.escrow_token_account.amount;
    msg!("Sending back {} to employer", amount_to_send_back);
    let send_back_to_employer = Transfer {
        from: ctx.accounts.escrow_token_account.to_account_info(),
        to: ctx.accounts.employer_account.to_account_info(),
        authority: ctx.accounts.escrow_authority.to_account_info(),
    };
    transfer(
        ctx.accounts
            .token_program_context(send_back_to_employer)
            .with_signer(&[&[
                b"authority",
                ctx.accounts.grant.key().as_ref(),
                &[ctx.accounts.grant.bumps.escrow_authority],
            ]]),
        amount_to_send_back,
    )?;

    // Mark grant as revoked, so this function cannot be called again.
    let data = &mut ctx.accounts.grant;
    data.revoked = true;
    Ok(())
}
