use anchor_lang::prelude::*;

use crate::utils::GrantInputParams;

#[derive(AnchorDeserialize, AnchorSerialize, Debug, Clone)]
pub struct Bumps {
    pub grant: u8,
    pub escrow_authority: u8,
    pub escrow_token_account: u8,
}

/// Stores data about the vesting schedule
#[account]
pub struct Grant {
    // Input parameters - will never change
    pub params: GrantInputParams,
    
    // State parameters - will change
    pub already_issued_token_amount: u64,
    pub revoked: bool,
    pub initialized: bool,

    // Public keys used for safety checks in "withdraw" + "revoke"
    pub employer: Pubkey,
    pub employee: Pubkey,
    pub mint: Pubkey,
    pub bumps: Bumps,
}

impl Grant {
    pub const LEN: usize = {
        let discriminant = 8;
        let grant_input_params = 5 * 8;
        let grant_bumps = 3 * 1;
        discriminant + grant_input_params + 8 + 1 + 1 + (32 * 3) + grant_bumps
    };
}
