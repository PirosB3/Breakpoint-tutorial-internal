use anchor_lang::prelude::*;

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
    use anchor_lang::{
        context,
        solana_program::{
            native_token::LAMPORTS_PER_SOL, system_instruction::SystemInstruction, system_program,
        },
    };
    use vestinglib::{CanInitialize, GetReleasableAmountParams, Vesting, VestingInitParams};

    use super::*;

    pub fn revoke(ctx: Context<RevokeGrant>) -> Result<()> {
        let clock = Clock::get()?;
        let vesting = {
            let grant_account = &ctx.accounts.grant_account;
            let GrantInputParams {
                cliff_seconds,
                duration_seconds,
                grant_token_amount,
                seconds_per_slice,
                start_unix,
            } = grant_account.params;
            let val = Vesting::from_init_params(&VestingInitParams {
                cliff_seconds,
                duration_seconds,
                grant_token_amount,
                seconds_per_slice,
                start_unix,
                already_issued_token_amount: grant_account.already_issued_token_amount,
                revoked: grant_account.revoked,
            })
            .unwrap();
            val
        };
        let releasable_amount = vesting
            .get_releasable_amount(&GetReleasableAmountParams {
                current_time_unix: clock.unix_timestamp as u64,
            })
            .unwrap();
        if releasable_amount > 0 {
            let transfer_accounts = anchor_lang::system_program::Transfer {
                from: ctx.accounts.grant_custody.to_account_info(),
                to: ctx.accounts.employee.to_account_info(),
            };
            anchor_lang::system_program::transfer(
                CpiContext::new_with_signer(
                    ctx.accounts.system_program.to_account_info(),
                    transfer_accounts,
                    &[&[
                        b"grant_custody",
                        ctx.accounts.employer.key().as_ref(),
                        ctx.accounts.employee.key().as_ref(),
                        &[ctx.accounts.grant_account.grant_custody_bump],
                    ]],
                ),
                releasable_amount,
            )?;
            let data = &mut ctx.accounts.grant_account;
            data.already_issued_token_amount += releasable_amount;
        }

        // Compute how much the account has
        
        msg!("Sending back {}", ctx.accounts.grant_custody.lamports());
        let transfer_accounts = anchor_lang::system_program::Transfer {
            from: ctx.accounts.grant_custody.to_account_info(),
            to: ctx.accounts.employer.to_account_info(),
        };
        anchor_lang::system_program::transfer(
            CpiContext::new_with_signer(
                ctx.accounts.system_program.to_account_info(),
                transfer_accounts,
                &[&[
                    b"grant_custody",
                    ctx.accounts.employer.key().as_ref(),
                    ctx.accounts.employee.key().as_ref(),
                    &[ctx.accounts.grant_account.grant_custody_bump],
                ]],
            ),
            ctx.accounts.grant_custody.lamports(),
        )?;
        let data = &mut ctx.accounts.grant_account;
        data.revoked = true;
        Ok(())
    }

    pub fn withdraw(ctx: Context<WithdrawGrant>, delta: u8) -> Result<Res> {
        let clock = Clock::get()?;
        let vesting = {
            let grant_account = &ctx.accounts.grant_account;
            let GrantInputParams {
                cliff_seconds,
                duration_seconds,
                grant_token_amount,
                seconds_per_slice,
                start_unix,
            } = grant_account.params;
            let val = Vesting::from_init_params(&VestingInitParams {
                cliff_seconds,
                duration_seconds,
                grant_token_amount,
                seconds_per_slice,
                start_unix,
                already_issued_token_amount: grant_account.already_issued_token_amount,
                revoked: grant_account.revoked,
            })
            .unwrap();
            val
        };
        let releasable_amount = vesting
            .get_releasable_amount(&GetReleasableAmountParams {
                current_time_unix: clock.unix_timestamp as u64,
            })
            .unwrap();
        msg!("Releasable amount: {}", releasable_amount);
        if releasable_amount > 0 {
            let transfer_accounts = anchor_lang::system_program::Transfer {
                from: ctx.accounts.grant_custody.to_account_info(),
                to: ctx.accounts.employee.to_account_info(),
            };
            anchor_lang::system_program::transfer(
                CpiContext::new_with_signer(
                    ctx.accounts.system_program.to_account_info(),
                    transfer_accounts,
                    &[&[
                        b"grant_custody",
                        ctx.accounts.employer.key().as_ref(),
                        ctx.accounts.employee.key().as_ref(),
                        &[ctx.accounts.grant_account.grant_custody_bump],
                    ]],
                ),
                releasable_amount,
            )?;
            let data = &mut ctx.accounts.grant_account;
            data.already_issued_token_amount += releasable_amount;
        }
        // Ok()
        Ok(Res { releasable_amount })
    }

    pub fn initialize(ctx: Context<InitializeNewGrant>, params: GrantInputParams) -> Result<()> {
        let GrantInputParams {
            cliff_seconds,
            duration_seconds,
            grant_token_amount,
            seconds_per_slice,
            start_unix,
        } = params;
        let val = Vesting::from_init_params(&VestingInitParams {
            cliff_seconds,
            duration_seconds,
            grant_token_amount,
            seconds_per_slice,
            start_unix,
            revoked: false,
            already_issued_token_amount: 0,
        });
        if val.is_err() {
            return err!(TokenVestingError::ParamsInvalid);
        }

        let transfer_accounts = anchor_lang::system_program::Transfer {
            from: ctx.accounts.employer.to_account_info(),
            to: ctx.accounts.grant_custody.to_account_info(),
        };
        let context = CpiContext::new(
            ctx.accounts.system_program.to_account_info(),
            transfer_accounts,
        );
        let min_rent = ctx.accounts.rent.minimum_balance(0);
        anchor_lang::system_program::transfer(context, grant_token_amount + min_rent)?;

        let bump = *ctx.bumps.get("grant_account").unwrap();
        let grant_custody_bump = *ctx.bumps.get("grant_custody").unwrap();
        {
            let grant_account = &mut ctx.accounts.grant_account;
            grant_account.params = params;
            grant_account.already_issued_token_amount = 0;
            grant_account.revoked = false;

            grant_account.initialized = true;
            grant_account.bump = bump;
            grant_account.employee = ctx.accounts.employee.key();
            grant_account.employer = ctx.accounts.employer.key();
            grant_account.grant_custody = ctx.accounts.grant_custody.key();
            grant_account.grant_custody_bump = grant_custody_bump;
        }
        Ok(())
    }
}

#[derive(AnchorDeserialize, AnchorSerialize, Debug, Clone)]
pub struct GrantInputParams {
    pub cliff_seconds: u64,
    pub duration_seconds: u64,
    pub seconds_per_slice: u64,
    pub start_unix: u64,
    pub grant_token_amount: u64,
}

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

#[derive(Accounts)]
pub struct WithdrawGrant<'info> {
    #[account(mut)]
    employee: Signer<'info>,
    /// CHECK: The account should just be the public key of whoever we want to give the grant to
    employer: AccountInfo<'info>,

    #[account(
        mut,
        seeds = [b"grant_account", employer.key().as_ref(), employee.key().as_ref()],
        bump = grant_account.bump,
        constraint = grant_account.initialized == true,
        constraint = grant_account.revoked == false,
    )]
    grant_account: Account<'info, Grant>,

    #[account(
        mut,
        seeds = [b"grant_custody", employer.key().as_ref(), employee.key().as_ref()],
        bump = grant_account.grant_custody_bump,
        constraint = grant_custody.key() == grant_account.grant_custody,
    )]
    /// CHECK: The account is a PDA
    grant_custody: AccountInfo<'info>,
    system_program: Program<'info, System>,
}

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


#[derive(Accounts)]
pub struct RevokeGrant<'info> {
    #[account(mut)]
    employer: Signer<'info>,
    /// CHECK: The account should just be the public key of whoever we want to give the grant to
    #[account(mut)]
    employee: AccountInfo<'info>,

    #[account(
        mut,
        seeds = [b"grant_account", employer.key().as_ref(), employee.key().as_ref()],
        bump = grant_account.bump,
        constraint = grant_custody.key() == grant_account.grant_custody,
        constraint = grant_account.initialized == true,
        constraint = grant_account.revoked == false,
    )]
    grant_account: Account<'info, Grant>,

    #[account(
        mut,
        seeds = [b"grant_custody", employer.key().as_ref(), employee.key().as_ref()], bump
    )]
    /// CHECK: The account is a PDA
    grant_custody: AccountInfo<'info>,
    system_program: Program<'info, System>,
}