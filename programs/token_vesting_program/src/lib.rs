use anchor_lang::prelude::*;

mod account_data;
mod instructions;
mod utils;

use instructions::initialize_grant::*;
use instructions::revoke_grant::*;
use instructions::withdraw::*;
use utils::GrantInputParams;

declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

#[error_code]
pub enum TokenVestingError {
    #[msg("Grant input parameters invalid")]
    ParamsInvalid,
    #[msg("Employer put a 0 token grant! call the union!")]
    EmployerNGMI,
}

#[program]
pub mod token_vesting_program {
    use crate::account_data::Bumps;

    use super::*;

    pub fn initialize(ctx: Context<InitializeNewGrant>, params: GrantInputParams) -> Result<()> {
        let grant_bump = *ctx.bumps.get("grant").unwrap();
        let escrow_authority_bump = *ctx.bumps.get("escrow_authority").unwrap();
        let escrow_token_account_bump = *ctx.bumps.get("escrow_token_account").unwrap();
        ctx.accounts.handle(
            params,
            Bumps {
                grant: grant_bump,
                escrow_authority: escrow_authority_bump,
                escrow_token_account: escrow_token_account_bump,
            },
        )
    }

    pub fn revoke(ctx: Context<RevokeGrant>) -> Result<()> {
        ctx.accounts.handle()
    }

    pub fn withdraw(ctx: Context<WithdrawGrant>) -> Result<()> {
        ctx.accounts.handle()
    }
}
