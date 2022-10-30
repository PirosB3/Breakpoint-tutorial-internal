use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, TokenAccount, Token, Transfer, transfer};

use crate::account_data::{Grant, Bumps};
use crate::utils::{get_vesting_instance, GrantInputParams, GrantStateParams};

#[derive(Accounts)]
pub struct InitializeNewGrant<'info> {
    // external accounts section
    // 👇 👇 👇 👇 👇
    #[account(mut)]
    employer: Signer<'info>,
    /// CHECK: The account should just be the public key of whoever we want to give the grant to
    employee: AccountInfo<'info>,
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
        seeds = [b"grant", employer.key().as_ref(), employee.key().as_ref()], bump
    )]
    grant: Account<'info, Grant>,
    #[account(
        seeds = [b"authority", grant.key().as_ref()], bump
    )]
    /// CHECK: The account is a PDA and does not read/write data
    escrow_authority: AccountInfo<'info>,
    #[account(
        init,
        payer = employer,
        token::mint=mint,
        token::authority=escrow_authority,
        seeds = [b"tokens", grant.key().as_ref()], bump
    )]
    escrow_token_account: Account<'info, TokenAccount>,

    // Programs section
    // 👇 👇 👇 👇 👇
    token_program: Program<'info, Token>,
    system_program: Program<'info, System>,
    rent: Sysvar<'info, Rent>,
}


/// This is the first instruction called by the employer when they want to set up a new vesting grant.
/// This instruction will set up the vesting schedule, validate the parameters, and transfer the total
/// grant size to an escrow account (called grant_custody).
impl<'info> InitializeNewGrant<'info> {
    pub fn system_program_context<T: ToAccountMetas + ToAccountInfos<'info>>(
        &self,
        data: T,
    ) -> CpiContext<'_, '_, '_, 'info, T> {
        CpiContext::new(self.system_program.to_account_info(), data)
    }

    pub fn handle(&mut self, params: GrantInputParams, bumps: Bumps) -> Result<()> {
        // Load vesting instance from params passed in to the instruction.
        // If there is an error in the parameters, an error will be returned and
        // program will exit.
        let _ = get_vesting_instance(
            &params,
            GrantStateParams {
                revoked: false,
                already_issued_token_amount: 0,
            },
        )?;

        // Transfer SOL from the employer to the escrow account (specifically for this employee) based on the input parameters
        //
        // Example: Total grant is 7000 SOL
        // Employer -> 7000 SOL -> Grant Custody
        //
        // NOTE: Writing to a new account requires us to pay rent for that account (even if no bytes are written).
        let amount_to_transfer = params.grant_token_amount;
        let context = self.system_program_context(Transfer {
            from: self.employer_account.to_account_info(),
            to: self.escrow_token_account.to_account_info(),
            authority: self.employer.to_account_info(),
        });
        transfer(context, amount_to_transfer)?;

        // Transfer was successful - let's store the grant terms (and other important info we want to persist) in to the memory of grant_account.
        let grant_account = &mut self.grant;
        grant_account.params = params;
        grant_account.already_issued_token_amount = 0;
        grant_account.revoked = false;
        grant_account.initialized = true;
        grant_account.employee = self.employee.key();
        grant_account.employer = self.employer.key();
        grant_account.mint = self.mint.key();
        grant_account.bumps = bumps;
        Ok(())
    }
}
