use crate::error::ErrorCode;
use crate::instructions::mint_fee;
use crate::state::Config;
use crate::state::Pool;
use anchor_lang::prelude::*;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token::{
    burn, mint_to, transfer, Burn, Mint, MintTo, Token, TokenAccount, Transfer,
};
use fixed::types::U128F0;
use std::cmp::min;

#[derive(Accounts)]
pub struct LiquidityOperation<'info> {
    #[account( seeds = [b"config"], bump = config.bump)]
    pub config: Box<Account<'info, Config>>,

    #[account(mut)]
    pub owner: Signer<'info>,
    #[account(
        mut,
        has_one = owner,
    )]
    pub user_ata0: Box<Account<'info, TokenAccount>>,
    #[account(
        mut,
        has_one = owner,
    )]
    pub user_ata1: Box<Account<'info, TokenAccount>>,
    #[account(
        init_if_needed,
        payer = owner,
        associated_token::mint = lp_mint,
        associated_token::authority = owner
    )]
    pub user_lp_ata: Box<Account<'info, TokenAccount>>,

    #[account(mut)]
    pub pool: Box<Account<'info, Pool>>,
    /// CHECK: authority so 1 acc pass in can derive all other pdas
    #[account(seeds=[b"authority", pool.key().as_ref()], bump)]
    pub pool_authority: AccountInfo<'info>,

    #[account(
        init_if_needed,
        payer = owner,
        associated_token::mint = lp_mint,
        associated_token::authority = pool_authority
    )]
    pub vault_lp: Box<Account<'info, TokenAccount>>,
    #[account(
        mut,
        constraint = vault0.mint == user_ata0.mint,
    )]
    pub vault0: Box<Account<'info, TokenAccount>>,
    #[account(
        mut,
        constraint = vault1.mint == user_ata1.mint,
    )]
    pub vault1: Box<Account<'info, TokenAccount>>,
    #[account(mut, seeds = [b"lp_mint", pool.key().as_ref()], bump)]
    pub lp_mint: Box<Account<'info, Mint>>,

    pub associated_token_program: Program<'info, AssociatedToken>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

pub fn add_liquidity(
    ctx: Context<LiquidityOperation>,
    amount0_desired: u64,
    amount1_desired: u64,
    amount0_min: u64,
    amount1_min: u64,
) -> Result<()> {
    let pool: &Box<Account<Pool>> = &ctx.accounts.pool;

    let (amount0, amount1) = calculate_liquidity_amounts(
        pool.reserve0,
        pool.reserve1,
        amount0_desired,
        amount1_desired,
        amount0_min,
        amount1_min,
    )?;

    let pool_key = pool.key();
    let pool_sign = &[b"authority", pool_key.as_ref(), &[ctx.bumps.pool_authority]];

    let mint_fee_ctx = CpiContext::new(
        ctx.accounts.token_program.to_account_info(),
        MintTo {
            to: ctx.accounts.vault_lp.to_account_info(),
            mint: ctx.accounts.lp_mint.to_account_info(),
            authority: ctx.accounts.pool_authority.to_account_info(),
        },
    );
    mint_fee(
        &ctx.accounts.config,
        &ctx.accounts.pool,
        ctx.accounts.lp_mint.supply,
        mint_fee_ctx.with_signer(&[pool_sign]),
    )?;

    let lp_mint = &ctx.accounts.lp_mint;
    let liquidity: u64;
    if lp_mint.supply == 0 {
        liquidity = U128F0::from_num((amount0 as u128) * (amount1 as u128))
            .sqrt()
            .to_num::<u64>();
    } else {
        liquidity = min(
            amount0 * lp_mint.supply / pool.reserve0,
            amount1 * lp_mint.supply / pool.reserve1,
        )
    }

    require!(liquidity > 0, ErrorCode::InsufficientLiquidityMinted);

    let mint_ctx = CpiContext::new(
        ctx.accounts.token_program.to_account_info(),
        MintTo {
            to: ctx.accounts.user_lp_ata.to_account_info(),
            mint: ctx.accounts.lp_mint.to_account_info(),
            authority: ctx.accounts.pool_authority.to_account_info(),
        },
    );
    mint_to(mint_ctx.with_signer(&[pool_sign]), liquidity)?;

    // transfer tokens from user to vault
    transfer(
        CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.user_ata0.to_account_info(),
                to: ctx.accounts.vault0.to_account_info(),
                authority: ctx.accounts.owner.to_account_info(),
            },
        ),
        amount0,
    )?;

    transfer(
        CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.user_ata1.to_account_info(),
                to: ctx.accounts.vault1.to_account_info(),
                authority: ctx.accounts.owner.to_account_info(),
            },
        ),
        amount1,
    )?;

    let pool: &mut Box<Account<Pool>> = &mut ctx.accounts.pool;
    pool.add_liquidity(amount0, amount1)?;
    pool.update_k_last();

    Ok(())
}

fn calculate_liquidity_amounts(
    reserve0: u64,
    reserve1: u64,
    amount0_desired: u64,
    amount1_desired: u64,
    amount0_min: u64,
    amount1_min: u64,
) -> Result<(u64, u64)> {
    let amount0: u64;
    let amount1: u64;
    if reserve0 == 0 && reserve1 == 0 {
        (amount0, amount1) = (amount0_desired, amount1_desired);
    } else {
        let amount1_optimal = quote(amount0_desired, reserve0, reserve1)?;
        if amount1_optimal <= amount1_desired {
            require!(
                amount1_optimal >= amount1_min,
                ErrorCode::InsufficientAmount
            );
            (amount0, amount1) = (amount0_desired, amount1_optimal);
        } else {
            let amount0_optimal = quote(amount1_desired, reserve1, reserve0)?;
            require!(
                amount0_optimal <= amount0_desired,
                ErrorCode::InsufficientAmount
            );
            require!(
                amount0_optimal >= amount0_min,
                ErrorCode::InsufficientAmount
            );
            (amount0, amount1) = (amount0_optimal, amount1_desired);
        }
    }

    Ok((amount0, amount1))
}

// given some amount of an asset and pair reserves, returns an equivalent amount of the other asset
fn quote(amount0: u64, reserve0: u64, reserve1: u64) -> Result<u64> {
    require!(amount0 > 0, ErrorCode::InsufficientAmount);
    require!(
        reserve0 > 0 && reserve1 > 0,
        ErrorCode::InsufficientReserves
    );

    Ok(amount0 * reserve1 / reserve0)
}

pub fn remove_liquidity(
    ctx: Context<LiquidityOperation>,
    liquidity: u64,
    amount0_min: u64,
    amount1_min: u64,
) -> Result<()> {
    let pool: &Box<Account<Pool>> = &ctx.accounts.pool;

    let pool_key = pool.key();
    let pool_sign = &[b"authority", pool_key.as_ref(), &[ctx.bumps.pool_authority]];

    let mint_fee_ctx = CpiContext::new(
        ctx.accounts.token_program.to_account_info(),
        MintTo {
            to: ctx.accounts.vault_lp.to_account_info(),
            mint: ctx.accounts.lp_mint.to_account_info(),
            authority: ctx.accounts.pool_authority.to_account_info(),
        },
    );
    mint_fee(
        &ctx.accounts.config,
        &ctx.accounts.pool,
        ctx.accounts.lp_mint.supply,
        mint_fee_ctx.with_signer(&[pool_sign]),
    )?;

    let (amount0, amount1) = calculate_removed_amounts(
        liquidity,
        ctx.accounts.lp_mint.supply,
        pool.reserve0,
        pool.reserve1,
        amount0_min,
        amount1_min,
    )?;

    // burn liquidity tokens
    let burn_ctx = CpiContext::new(
        ctx.accounts.token_program.to_account_info(),
        Burn {
            from: ctx.accounts.user_lp_ata.to_account_info(),
            mint: ctx.accounts.lp_mint.to_account_info(),
            authority: ctx.accounts.owner.to_account_info(),
        },
    );
    burn(burn_ctx.with_signer(&[pool_sign]), liquidity)?;

    // transfer tokens from vault to user
    transfer(
        CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.vault0.to_account_info(),
                to: ctx.accounts.user_ata0.to_account_info(),
                authority: ctx.accounts.pool_authority.to_account_info(),
            },
        )
        .with_signer(&[pool_sign]),
        amount0,
    )?;

    transfer(
        CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.vault1.to_account_info(),
                to: ctx.accounts.user_ata1.to_account_info(),
                authority: ctx.accounts.pool_authority.to_account_info(),
            },
        )
        .with_signer(&[pool_sign]),
        amount1,
    )?;

    let pool: &mut Box<Account<Pool>> = &mut ctx.accounts.pool;
    pool.remove_liquidity(amount0, amount1)?;
    pool.update_k_last();

    Ok(())
}

fn calculate_removed_amounts(
    liquidity: u64,
    lp_supply: u64,
    reserve0: u64,
    reserve1: u64,
    amount0_min: u64,
    amount1_min: u64,
) -> Result<(u64, u64)> {
    let amount0: u64 = (liquidity as u128 * reserve0 as u128 / lp_supply as u128) as u64;
    let amount1: u64 = (liquidity as u128 * reserve1 as u128 / lp_supply as u128) as u64;

    require!(
        amount0 >= amount0_min && amount1 >= amount1_min,
        ErrorCode::InsufficientAmount
    );

    require!(
        amount0 != 0 && amount1 != 0,
        ErrorCode::InsufficientLiquidityBurned
    );

    Ok((amount0, amount1))
}
