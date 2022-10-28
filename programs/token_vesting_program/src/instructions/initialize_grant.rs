use anchor_lang::prelude::*;
use anchor_lang::system_program::Transfer;
use std::collections::BTreeMap;

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
    grant_account: Account<'info, Grant>,

    #[account(
        mut,
        seeds = [b"grant_custody", employer.key().as_ref(), employee.key().as_ref()], bump
    )]
    /// CHECK: The account is a PDA
    grant_custody: AccountInfo<'info>,
    system_program: Program<'info, System>,
    rent: Sysvar<'info, Rent>,
}

impl<'info> InitializeNewGrant<'info> {
    pub fn system_program_context<T: ToAccountMetas + ToAccountInfos<'info>>(
        &self,
        data: T,
    ) -> CpiContext<'_, '_, '_, 'info, T> {
        CpiContext::new(self.system_program.to_account_info(), data)
    }

    pub fn handle(&mut self, params: GrantInputParams, bumps: &BTreeMap<String, u8>) -> Result<()> {
        let _ = get_vesting_instance(
            &params,
            GrantStateParams {
                revoked: self.grant_account.revoked,
                already_issued_token_amount: self.grant_account.already_issued_token_amount,
            },
        )?;

        let context = self.system_program_context(Transfer {
            from: self.employer.to_account_info(),
            to: self.grant_custody.to_account_info(),
        });
        let min_rent = self.rent.minimum_balance(0);
        anchor_lang::system_program::transfer(context, params.grant_token_amount + min_rent)?;

        let bump = *bumps.get("grant_account").unwrap();
        let grant_custody_bump = *bumps.get("grant_custody").unwrap();
        {
            let grant_account = &mut self.grant_account;
            grant_account.params = params;
            grant_account.already_issued_token_amount = 0;
            grant_account.revoked = false;

            grant_account.initialized = true;
            grant_account.bump = bump;
            grant_account.employee = self.employee.key();
            grant_account.employer = self.employer.key();
            grant_account.grant_custody = self.grant_custody.key();
            grant_account.grant_custody_bump = grant_custody_bump;
        }
        Ok(())
    }
}
