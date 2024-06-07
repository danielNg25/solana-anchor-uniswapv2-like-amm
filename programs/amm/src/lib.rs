use anchor_lang::prelude::*;
use instructions::*;

pub mod constant;
pub mod error;
pub mod instructions;
pub mod state;

declare_id!("4tPXqXq5WiLpHPaJSRhpA1we5GhCpQrK3wpdRZFNoFQS");

#[program]
pub mod amm {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>, fee_to: Pubkey, fee: u64) -> Result<()> {
        instructions::initialize(ctx, fee_to, fee)
    }

    pub fn set_fee_to(ctx: Context<SetFeeTo>, new_fee_to: Pubkey) -> Result<()> {
        instructions::set_fee_to(ctx, new_fee_to)
    }

    pub fn set_fee(ctx: Context<SetFee>, new_fee: u64) -> Result<()> {
        instructions::set_fee(ctx, new_fee)
    }

    pub fn create_pool(ctx: Context<CreatePool>) -> Result<()> {
        instructions::create_pool(ctx)
    }

    pub fn add_liquidity(
        ctx: Context<LiquidityOperation>,
        amount0_desired: u64,
        amount1_desired: u64,
        amount0_min: u64,
        amount1_min: u64,
    ) -> Result<()> {
        instructions::add_liquidity(
            ctx,
            amount0_desired,
            amount1_desired,
            amount0_min,
            amount1_min,
        )
    }

    pub fn remove_liquidity(
        ctx: Context<LiquidityOperation>,
        liquidity: u64,
        amount0_min: u64,
        amount1_min: u64,
    ) -> Result<()> {
        instructions::remove_liquidity(ctx, liquidity, amount0_min, amount1_min)
    }
}
