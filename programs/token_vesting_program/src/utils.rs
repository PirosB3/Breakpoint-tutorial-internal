use anchor_lang::prelude::*;
use anchor_lang::{AnchorDeserialize, AnchorSerialize};
use vestinglib::{CanInitialize, Vesting, VestingInitParams};

use crate::TokenVestingError;

#[derive(AnchorDeserialize, AnchorSerialize, Debug, Clone)]
pub struct GrantInputParams {
    pub cliff_seconds: u64,
    pub duration_seconds: u64,
    pub seconds_per_slice: u64,
    pub start_unix: u64,
    pub grant_token_amount: u64,
}

pub struct GrantStateParams {
    pub revoked: bool,
    pub already_issued_token_amount: u64,
}

pub fn get_vesting_instance(params: &GrantInputParams, state: GrantStateParams) -> Result<Vesting> {
    let GrantInputParams {
        cliff_seconds,
        duration_seconds,
        grant_token_amount,
        seconds_per_slice,
        start_unix,
    } = *params;
    let GrantStateParams {
        already_issued_token_amount,
        revoked,
    } = state;
    let vesting = Vesting::from_init_params(&VestingInitParams {
        cliff_seconds,
        duration_seconds,
        grant_token_amount,
        seconds_per_slice,
        start_unix,
        revoked,
        already_issued_token_amount,
    })
    .or(Err(TokenVestingError::ParamsInvalid))?;
    Ok(vesting)
}
