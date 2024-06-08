use crate::state::config::Config;
use crate::state::Pool;
use anchor_lang::prelude::*;
use anchor_spl::token::{mint_to, MintTo};
use fixed::types::U128F0;

#[derive(Accounts)]
pub struct SetFeeTo<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,
    #[account(mut, seeds = [b"config"], bump = config.bump, has_one = owner)]
    pub config: Account<'info, Config>,
    pub system_program: Program<'info, System>,
}

pub fn set_fee_to(ctx: Context<SetFeeTo>, new_fee_to: Pubkey) -> Result<()> {
    let config = &mut ctx.accounts.config;
    Ok(config.set_fee_to(new_fee_to)?)
}

#[derive(Accounts)]
pub struct SetFee<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,
    #[account(mut, seeds = [b"config"], bump = config.bump, has_one = owner)]
    pub config: Account<'info, Config>,
    pub system_program: Program<'info, System>,
}

pub fn set_fee(ctx: Context<SetFee>, new_fee: u64) -> Result<()> {
    let config = &mut ctx.accounts.config;
    Ok(config.set_fee(new_fee)?)
}

pub fn mint_fee<'info>(
    _config: &Config,
    pool: &Pool,
    reserve0: u64,
    reserve1: u64,
    lp_supply: u64,
    mint_ctx: CpiContext<'_, '_, '_, 'info, MintTo<'info>>,
) -> Result<()> {
    let k_last = pool.k_last;

    if k_last != 0 {
        let root_k: u128 = U128F0::from_num((reserve0 as u128) * (reserve1 as u128))
            .sqrt()
            .to_num::<u128>();
        let root_k_last = U128F0::from_num(k_last).sqrt().to_num::<u128>();
        if root_k > root_k_last {
            let numerator: u128 = (lp_supply as u128) * (root_k - root_k_last);
            let denominator: u128 = root_k * 5 + root_k_last;
            let liquidity: u64 = (numerator / denominator) as u64;
            if liquidity > 0 {
                mint_to(mint_ctx, liquidity)?;
            }
        }
    }

    Ok(())
}
