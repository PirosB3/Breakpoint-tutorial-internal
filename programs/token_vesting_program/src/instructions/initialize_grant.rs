use anchor_lang::prelude::*;
use anchor_lang::system_program::Transfer;

use crate::account_data::Grant;
use crate::utils::{get_vesting_instance, GrantInputParams, GrantStateParams};

#[derive(Accounts)]
pub struct InitializeNewGrant<'info> {
    #[account(mut)]
    employer: Signer<'info>,
    /// CHECK: The account should just be the public key of whoever we want to give the grant to
    employee: AccountInfo<'info>,

    #[account(
        init,
        payer = employer,
        space = Grant::MAX_SIZE,
        seeds = [b"grant_account", employer.key().as_ref(), employee.key().as_ref()], bump
    )]
    grant: Account<'info, Grant>,

    #[account(
        mut,
        seeds = [b"grant_custody", employer.key().as_ref(), employee.key().as_ref()], bump
    )]
    /// CHECK: The account is a PDA
    grant_custody: AccountInfo<'info>,

    system_program: Program<'info, System>,
    rent: Sysvar<'info, Rent>,
}

pub struct Bumps {
    pub grant_bump: u8,
    pub grant_custody_bump: u8,
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

        let amount_to_transfer = {
            // Compute rent for an account of 0 bytes. Rent is still paid for accounts that store SOL.
            let min_rent = self.rent.minimum_balance(0);
            min_rent + params.grant_token_amount
        };

        // Transfer SOL from the employer to the escrow account (specifically for this employee) based on the input parameters
        //
        // Example: Total grant is 7000 SOL
        // Employer -> 7000 SOL -> Grant Custody
        //
        // NOTE: Writing to a new account requires us to pay rent for that account (even if no bytes are written).
        let context = self.system_program_context(Transfer {
            from: self.employer.to_account_info(),
            to: self.grant_custody.to_account_info(),
        });
        anchor_lang::system_program::transfer(context, amount_to_transfer)?;

        // Transfer was successful - let's store the grant terms (and other important info we want to persist) in to the memory of grant_account.
        let grant_account = &mut self.grant;
        grant_account.params = params;
        grant_account.already_issued_token_amount = 0;
        grant_account.revoked = false;
        grant_account.initialized = true;
        grant_account.employee = self.employee.key();
        grant_account.employer = self.employer.key();
        grant_account.bump = bumps.grant_bump;
        grant_account.grant_custody_bump = bumps.grant_custody_bump;
        Ok(())
    }
}
