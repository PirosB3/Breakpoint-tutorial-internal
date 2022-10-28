use anchor_lang::prelude::*;
use anchor_lang::system_program::Transfer;

mod account_data;
mod instructions;
mod pda;
mod utils;

use account_data::Grant;
use instructions::initialize_grant::*;
use instructions::revoke_grant::*;
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

    pub fn initialize(ctx: Context<InitializeNewGrant>, params: GrantInputParams) -> Result<()> {
        let grant_bump = *ctx.bumps.get("grant").unwrap();
        let grant_custody_bump = *ctx.bumps.get("grant_custody").unwrap();
        ctx.accounts.handle(
            params,
            Bumps {
                grant_bump,
                grant_custody_bump,
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
