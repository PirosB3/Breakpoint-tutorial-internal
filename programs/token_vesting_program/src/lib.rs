use anchor_lang::prelude::*;
use anchor_lang::system_program::Transfer;
use vestinglib::{Vesting, VestingInitParams, CanInitialize};
mod pda;


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

macro_rules! grant_custody_seeds {
    ($ctx:expr) => {
        &[&[
            b"grant_custody",
            $ctx.accounts.employer.key().as_ref(),
            $ctx.accounts.employee.key().as_ref(),
            &[$ctx.accounts.grant_account.grant_custody_bump],
        ]] 
    };
}
fn get_vesting_instance(params: &GrantInputParams, state: GrantStateParams) -> Result<Vesting> {
    let GrantInputParams {
        cliff_seconds,
        duration_seconds,
        grant_token_amount,
        seconds_per_slice,
        start_unix,
    } = *params;
    let GrantStateParams{
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
    }).or(Err(TokenVestingError::ParamsInvalid))?;
    Ok(vesting)
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
        let vesting = get_vesting_instance(
            &ctx.accounts.grant_account.params,
            GrantStateParams {
                revoked: ctx.accounts.grant_account.revoked,
                already_issued_token_amount: ctx.accounts.grant_account.already_issued_token_amount,
            }
        )?;
        let clock = Clock::get()?;
        let releasable_amount = vesting
            .get_releasable_amount(&GetReleasableAmountParams {
                current_time_unix: clock.unix_timestamp as u64,
            })
            .unwrap();
        if releasable_amount > 0 {
            msg!("Sending remaining {} to employee", releasable_amount);
            let release_to_employee = Transfer {
                from: ctx.accounts.grant_custody.to_account_info(),
                to: ctx.accounts.employee.to_account_info(),
            };
            anchor_lang::system_program::transfer(
                ctx.accounts.system_program_context(release_to_employee)
                .with_signer(grant_custody_seeds!(ctx)),
                releasable_amount
            )?;
            let data = &mut ctx.accounts.grant_account;
            data.already_issued_token_amount += releasable_amount;
        }

        // Compute how much the account has
        let amount_to_send_back = ctx.accounts.grant_custody.lamports();
        msg!("Sending back {} to employer", amount_to_send_back);
        let send_back_to_employer = Transfer {
            from: ctx.accounts.grant_custody.to_account_info(),
            to: ctx.accounts.employer.to_account_info(),
        };
        anchor_lang::system_program::transfer(
            ctx.accounts
                .system_program_context(send_back_to_employer)
                .with_signer(grant_custody_seeds!(ctx)),
            amount_to_send_back
        )?;
        let data = &mut ctx.accounts.grant_account;
        data.revoked = true;
        Ok(())
    }

    pub fn withdraw(ctx: Context<WithdrawGrant>, delta: u8) -> Result<Res> {
        let vesting = get_vesting_instance(
            &ctx.accounts.grant_account.params,
            GrantStateParams {
                revoked: ctx.accounts.grant_account.revoked,
                already_issued_token_amount: ctx.accounts.grant_account.already_issued_token_amount,
            }
        )?;
        let clock = Clock::get()?;
        let releasable_amount = vesting
            .get_releasable_amount(&GetReleasableAmountParams {
                current_time_unix: clock.unix_timestamp as u64,
            })
            .unwrap();
        msg!("Releasable amount: {}", releasable_amount);
        if releasable_amount > 0 {
            anchor_lang::system_program::transfer(
                ctx.accounts.system_program_context(Transfer {
                    from: ctx.accounts.grant_custody.to_account_info(),
                    to: ctx.accounts.employee.to_account_info(),
                }).with_signer(grant_custody_seeds!(ctx)),
                releasable_amount
            )?;

            let data = &mut ctx.accounts.grant_account;
            data.already_issued_token_amount += releasable_amount;
        }
        Ok(Res { releasable_amount })
    }



    pub fn initialize(ctx: Context<InitializeNewGrant>, params: GrantInputParams) -> Result<()> {
        let vesting = get_vesting_instance(
            &params,
            GrantStateParams {
                revoked: ctx.accounts.grant_account.revoked,
                already_issued_token_amount: ctx.accounts.grant_account.already_issued_token_amount,
            }
        )?;

        let context = ctx.accounts.system_program_context(Transfer {
            from: ctx.accounts.employer.to_account_info(),
            to: ctx.accounts.grant_custody.to_account_info(),
        });
        let min_rent = ctx.accounts.rent.minimum_balance(0);
        anchor_lang::system_program::transfer(context, params.grant_token_amount + min_rent)?;

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

pub struct GrantStateParams {
    revoked: bool,
    already_issued_token_amount: u64,
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

impl<'info> WithdrawGrant<'info> {
    pub fn system_program_context<T: ToAccountMetas + ToAccountInfos<'info>>(
        &self,
        data: T,
    ) -> CpiContext<'_, '_, '_, 'info, T> {
        CpiContext::new(
            self.system_program.to_account_info(),
            data,
        )
    }
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

impl<'info> InitializeNewGrant<'info> {
    pub fn system_program_context<T: ToAccountMetas + ToAccountInfos<'info>>(
        &self,
        data: T,
    ) -> CpiContext<'_, '_, '_, 'info, T> {
        CpiContext::new(
            self.system_program.to_account_info(),
            data,
        )
    }
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

impl<'info> RevokeGrant<'info> {
    pub fn system_program_context<T: ToAccountMetas + ToAccountInfos<'info>>(
        &self,
        data: T,
    ) -> CpiContext<'_, '_, '_, 'info, T> {
        CpiContext::new(
            self.system_program.to_account_info(),
            data,
        )
    }
}
