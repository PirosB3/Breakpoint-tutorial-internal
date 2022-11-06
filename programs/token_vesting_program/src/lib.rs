use anchor_lang::prelude::*;

mod account_data;
mod instructions;
mod utils;

use instructions::initialize::*;
use instructions::revoke::*;
use instructions::withdraw::*;
use utils::GrantInputParams;

declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

#[error_code]
pub enum TokenVestingError {
    #[msg("Grant input parameters invalid")]
    ParamsInvalid,
    // #[msg("Employer put a 0 token grant! call the union!")]
    // EmployerNGMI,
}

#[program]
pub mod token_vesting_program {
    use super::*;

    pub fn initialize(ctx: Context<InitializeNewGrant>, params: GrantInputParams) -> Result<()> {
        instructions::initialize::initialize(ctx, params)
    }

    pub fn revoke(ctx: Context<RevokeGrant>) -> Result<()> {
        instructions::revoke::revoke(ctx)
    }

    pub fn withdraw(ctx: Context<WithdrawGrant>) -> Result<()> {
        instructions::withdraw::withdraw(ctx)
    }
}
