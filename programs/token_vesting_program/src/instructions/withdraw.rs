use anchor_lang::prelude::*;
use anchor_spl::token::{transfer, Token, TokenAccount, Transfer};
use vestinglib::GetReleasableAmountParams;

use crate::account_data::Grant;
use crate::utils::{get_vesting_instance, GrantStateParams};

#[derive(Accounts)]
pub struct WithdrawGrant<'info> {
    // external accounts section
    // 👇 👇 👇 👇 👇
    #[account(address = grant.employee)]
    employee: Signer<'info>,
    #[account(address = grant.employer)]
    employer: SystemAccount<'info>,
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
    grant: Account<'info, Grant>,
    #[account(
        seeds = [b"authority".as_ref(), grant.key().as_ref()],
        bump = grant.bumps.escrow_authority
    )]
    escrow_authority: SystemAccount<'info>,
    #[account(
        mut,
        token::mint=grant.mint,
        token::authority=escrow_authority,
        seeds = [b"tokens".as_ref(), grant.key().as_ref()],
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
}

/// Goal of instruction
/// 1) Get releasable amount that can be withdrawn by employee
/// 2) Transfer amount to employee-owned token account
/// 3) Update state
pub fn withdraw(ctx: Context<WithdrawGrant>) -> Result<()> {
    // Load vesting instance from the internal state.
    let accounts = &ctx.accounts;
    let vesting = get_vesting_instance(
        &accounts.grant.params,
        GrantStateParams {
            revoked: accounts.grant.revoked,
            already_issued_token_amount: accounts.grant.already_issued_token_amount,
        },
    )?;

    // Before a grant is permanently terminated, we force the employee
    // to withdraw all the remaining (vested) tokens - if any exist.
    let clock = Clock::get()?;
    let releasable_amount = vesting
        .get_releasable_amount(&GetReleasableAmountParams {
            current_time_unix: clock.unix_timestamp as u64,
        })
        .unwrap();
    msg!("Releasable amount: {}", releasable_amount);

    if releasable_amount > 0 {
        // In this transfer, we must pass in Signer Seeds - because funds are going from the Grant Custody
        // To the employee - and Grant Custody is a PDA.
        //
        // Grant Custody -> 1000 SOL -> Employee
        transfer(
            accounts
                .token_program_context(Transfer {
                    from: accounts.escrow_token_account.to_account_info(),
                    to: accounts.employee_account.to_account_info(),
                    authority: accounts.escrow_authority.to_account_info(),
                })
                .with_signer(&[&[
                    b"authority",
                    accounts.grant.key().as_ref(),
                    &[accounts.grant.bumps.escrow_authority],
                ]]),
            releasable_amount,
        )?;

        // Transfer was successful, update persistent state to account for the funds already released.
        // 
        // Prevent arithmetic errors in Solana smart contracts
        // https://medium.com/coinmonks/understanding-arithmetic-overflow-underflows-in-rust-and-solana-smart-contracts-9f3c9802dc45
        let updated_issued_amount = ctx.accounts.grant.already_issued_token_amount.checked_add(releasable_amount).unwrap();
        ctx.accounts.grant.already_issued_token_amount = updated_issued_amount;
        msg!("OUT -> {}", ctx.accounts.grant.already_issued_token_amount);
    }
    Ok(())
}
