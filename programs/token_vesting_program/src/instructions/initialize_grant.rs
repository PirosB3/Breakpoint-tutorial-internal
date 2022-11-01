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
        space = Grant::MAX_SIZE,
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

/// Goal of instruction
/// 1) Validate the vesting input parameters (exit if error)
/// 2) Transfer the total grant size to an escrow token account
/// 3) Initialize state
impl<'info> InitializeNewGrant<'info> {
    pub fn token_program_context<T: ToAccountMetas + ToAccountInfos<'info>>(
        &self,
        data: T,
    ) -> CpiContext<'_, '_, '_, 'info, T> {
        CpiContext::new(self.token_program.to_account_info(), data)
    }

    pub fn handle(&mut self, params: GrantInputParams, bumps: Bumps) -> Result<()> {
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
        let context = self.token_program_context(Transfer {
            from: self.employer_account.to_account_info(),
            to: self.escrow_token_account.to_account_info(),
            authority: self.employer.to_account_info(),
        });
        transfer(context, amount_to_transfer)?;

        // Transfer was successful - let's store the grant terms (and other important info we want to persist) in to the memory of grant_account.
        self.grant.set_inner(Grant {
            params,
            already_issued_token_amount: 0,
            revoked: false,
            initialized: true,
            employer: self.employer.key(),
            employee: self.employee.key(),
            mint: self.mint.key(),
            bumps,
        });
        Ok(())
    }
}
