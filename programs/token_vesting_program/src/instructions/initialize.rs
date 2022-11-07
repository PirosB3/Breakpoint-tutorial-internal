use anchor_lang::prelude::*;
use anchor_spl::token::{transfer, Mint, Token, TokenAccount, Transfer};

use crate::account_data::{Bumps, Grant};
use crate::utils::{get_vesting_instance, GrantInputParams, GrantStateParams};
use crate::TokenVestingError;

#[derive(Accounts)]
pub struct InitializeNewGrant<'info> {
    // external accounts section
    // 👇 👇 👇 👇 👇
    #[account(mut)]
    employer: Signer<'info>,
    #[account(constraint = employee.key() != employer.key())]
    employee: SystemAccount<'info>,
    #[account(constraint = mint.is_initialized == true)]
    mint: Account<'info, Mint>,
    #[account(mut, token::mint=mint, token::authority=employer)]
    employer_account: Account<'info, TokenAccount>,

    // PDAs section
    // 👇 👇 👇 👇 👇
    #[account(
        init,
        payer = employer,
        space = Grant::LEN,
        seeds = [b"grant".as_ref(), employer.key().as_ref(), employee.key().as_ref()], bump
    )]
    grant: Account<'info, Grant>,
    #[account(
        seeds = [b"authority".as_ref(), grant.key().as_ref()], bump
    )]
    escrow_authority: SystemAccount<'info>,
    #[account(
        init,
        payer = employer,
        token::mint=mint,
        token::authority=escrow_authority,
        seeds = [b"tokens".as_ref(), grant.key().as_ref()], bump
    )]
    escrow_token_account: Account<'info, TokenAccount>,

    // Programs section
    // 👇 👇 👇 👇 👇
    token_program: Program<'info, Token>,
    system_program: Program<'info, System>,
    rent: Sysvar<'info, Rent>,
}

impl<'info> InitializeNewGrant<'info> {
    pub fn token_program_context<T: ToAccountMetas + ToAccountInfos<'info>>(
        &self,
        data: T,
    ) -> CpiContext<'_, '_, '_, 'info, T> {
        CpiContext::new(self.token_program.to_account_info(), data)
    }
}

/// Goal of instruction
/// 1) Validate the vesting input parameters (exit if error)
/// 2) Transfer the total grant size to an escrow token account
/// 3) Initialize state
pub fn initialize(ctx: Context<InitializeNewGrant>, params: GrantInputParams) -> Result<()> {
    // Quick check to ensure grant token amount is greater than 0
    // if params.grant_token_amount == 0 {
    //     return err!(TokenVestingError::EmployerNGMI);
    // }

    // Load vesting instance from params passed in to the instruction.
    // If there is an error in the parameters, an error will be returned and
    // program will exit.
    let vesting_result = get_vesting_instance(
        &params,
        GrantStateParams {
            revoked: false,
            already_issued_token_amount: 0,
        },
    );
    if vesting_result.is_err() {
        return err!(TokenVestingError::ParamsInvalid);
    }

    // Transfer token from the employer to the escrow account (specifically for this grant) based on the input parameters
    //
    // Example: Total grant is 7000 tokens
    // Employer -> 7000 SOL -> Grant Custody
    //
    // NOTE: Writing to a new account requires us to pay rent for that account (even if no bytes are written).
    let amount_to_transfer = params.grant_token_amount;
    let context = ctx.accounts.token_program_context(Transfer {
        from: ctx.accounts.employer_account.to_account_info(),
        to: ctx.accounts.escrow_token_account.to_account_info(),
        authority: ctx.accounts.employer.to_account_info(),
    });
    transfer(context, amount_to_transfer)?;

    // Transfer was successful - let's store the grant terms (and other important info we want to persist) in to the memory of grant_account.
    let grant_bump = *ctx.bumps.get("grant").unwrap();
    let escrow_authority_bump = *ctx.bumps.get("escrow_authority").unwrap();
    let escrow_token_account_bump = *ctx.bumps.get("escrow_token_account").unwrap();
    let bumps = Bumps{
        grant: grant_bump,
        escrow_authority: escrow_authority_bump,
        escrow_token_account: escrow_token_account_bump,
    };
    ctx.accounts.grant.set_inner(Grant {
        params,
        already_issued_token_amount: 0,
        revoked: false,
        initialized: true,
        employer: ctx.accounts.employer.key(),
        employee: ctx.accounts.employee.key(),
        mint: ctx.accounts.mint.key(),
        bumps,
    });
    Ok(())
}
