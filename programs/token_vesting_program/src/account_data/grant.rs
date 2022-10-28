use anchor_lang::prelude::*;

use crate::utils::GrantInputParams;

#[account]
pub struct Grant {
    pub params: GrantInputParams,
    pub already_issued_token_amount: u64,
    pub revoked: bool,

    pub initialized: bool,
    pub employer: Pubkey,
    pub employee: Pubkey,
    pub bump: u8,
    pub grant_custody_bump: u8,
}

impl Grant {
    pub const MAX_SIZE: usize = {
        let discriminant = 8;
        let grant_input_params = 5 * 8;
        discriminant + grant_input_params + 8 + 1 + 1 + (32 * 2) + 1 + 1
    };
}
