use anchor_lang::prelude::*;

use crate::utils::GrantInputParams;

#[account]
pub struct Grant {
    pub params: GrantInputParams,
    pub already_issued_token_amount: u64,
    pub revoked: bool,

    pub initialized: bool,
    pub bump: u8,
    pub employer: Pubkey,
    pub employee: Pubkey,
    pub grant_custody: Pubkey,
    pub grant_custody_bump: u8,
}

impl Grant {
    pub const MAX_SIZE: usize = {
        let discriminant = 8;
        let grant_input_params = 5 * 8;
        discriminant + grant_input_params + 8 + 1 + 1 + 1 + (32 * 3) + 1
    };
}
