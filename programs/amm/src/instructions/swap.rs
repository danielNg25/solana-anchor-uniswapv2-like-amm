use crate::constant::BASIS_POINTS;
use crate::error::ErrorCode;
use crate::state::Config;
use crate::state::Pool;
use anchor_lang::prelude::*;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token::{transfer, Mint, Token, TokenAccount, Transfer};

#[derive(Accounts)]
pub struct Swap<'info> {
    #[account( seeds = [b"config"], bump = config.bump)]
    pub config: Box<Account<'info, Config>>,

    #[account(mut)]
    pub owner: Signer<'info>,
    #[account(mut, has_one = owner)]
    pub user_ata_src: Box<Account<'info, TokenAccount>>,
    #[account(mut, has_one = owner)]
    pub user_ata_des: Box<Account<'info, TokenAccount>>,

    #[account(mut)]
    pub pool: Box<Account<'info, Pool>>,
    /// CHECK: authority so 1 acc pass in can derive all other pdas
    #[account(seeds=[b"authority", pool.key().as_ref()], bump)]
    pub pool_authority: AccountInfo<'info>,

    #[account(mut, constraint = user_ata_src.mint == vault_src.mint)]
    pub vault_src: Box<Account<'info, TokenAccount>>,
    #[account(mut, constraint = user_ata_des.mint == vault_des.mint)]
    pub vault_des: Box<Account<'info, TokenAccount>>,
    #[account(mut, seeds = [b"lp_mint", pool.key().as_ref()], bump)]
    pub lp_mint: Box<Account<'info, Mint>>,

    pub associated_token_program: Program<'info, AssociatedToken>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

pub fn swap_exact_input(
    ctx: Context<Swap>,
    input_amount: u64,
    min_output_amount: u64,
) -> Result<()> {
    let amount_out = get_amount_out(
        &ctx.accounts.config,
        input_amount,
        ctx.accounts.vault_src.amount,
        ctx.accounts.vault_des.amount,
    )?;
    require!(
        amount_out >= min_output_amount,
        ErrorCode::InsufficientOutputAmount,
    );

    swap(ctx, input_amount, amount_out)?;

    Ok(())
}

pub fn swap_exact_output(
    ctx: Context<Swap>,
    output_amount: u64,
    max_input_amount: u64,
) -> Result<()> {
    let amount_in = get_amount_in(
        &ctx.accounts.config,
        output_amount,
        ctx.accounts.vault_src.amount,
        ctx.accounts.vault_des.amount,
    )?;

    require!(
        amount_in <= max_input_amount,
        ErrorCode::InsufficientInputAmount,
    );

    swap(ctx, amount_in, output_amount)?;

    Ok(())
}

fn get_amount_out(
    config: &Config,
    amount_in: u64,
    reserve_in: u64,
    reserve_out: u64,
) -> Result<u64> {
    require!(
        reserve_in > 0 && reserve_out > 0,
        ErrorCode::InsufficientLiquidity,
    );
    let amount_in_with_fee = amount_in as u128 * (BASIS_POINTS - config.fee) as u128;
    let numerator = amount_in_with_fee * reserve_out as u128;
    let denominator = reserve_in as u128 * BASIS_POINTS as u128 + amount_in_with_fee;
    Ok((numerator / denominator) as u64)
}

fn get_amount_in(
    config: &Config,
    amount_out: u64,
    reserve_in: u64,
    reserve_out: u64,
) -> Result<u64> {
    require!(
        reserve_in > 0 && reserve_out > 0,
        ErrorCode::InsufficientLiquidity,
    );
    let numerator = reserve_in as u128 * amount_out as u128 * BASIS_POINTS as u128;
    let denominator =
        (reserve_out as u128 - amount_out as u128) * (BASIS_POINTS - config.fee) as u128;
    Ok((numerator / denominator + 1) as u64)
}

fn swap(ctx: Context<Swap>, input_amount: u64, output_amount: u64) -> Result<()> {
    require!(output_amount > 0, ErrorCode::InsufficientOutputAmount,);
    require!(input_amount > 0, ErrorCode::InsufficientInputAmount,);
    require!(
        output_amount < ctx.accounts.vault_des.amount,
        ErrorCode::InsufficientLiquidity,
    );
    require!(
        input_amount < ctx.accounts.user_ata_src.amount,
        ErrorCode::InsufficientUserBalance,
    );

    let pool: &Box<Account<Pool>> = &ctx.accounts.pool;
    let pool_key = pool.key();
    let pool_sign = &[b"authority", pool_key.as_ref(), &[ctx.bumps.pool_authority]];

    // transfer tokens from user to vault
    transfer(
        CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.user_ata_src.to_account_info(),
                to: ctx.accounts.vault_src.to_account_info(),
                authority: ctx.accounts.owner.to_account_info(),
            },
        ),
        input_amount,
    )?;

    // transfer tokens from vault to user
    transfer(
        CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.vault_des.to_account_info(),
                to: ctx.accounts.user_ata_des.to_account_info(),
                authority: ctx.accounts.pool_authority.to_account_info(),
            },
        )
        .with_signer(&[pool_sign]),
        output_amount,
    )?;

    Ok(())
}
