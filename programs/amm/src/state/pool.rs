use crate::error::ErrorCode;
use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)] // automatically calculate the space required for the struct
pub struct Pool {
    pub token0: Pubkey,
    pub token1: Pubkey,
    pub reserve0: u64,
    pub reserve1: u64,
    pub k_last: u128,
}

impl Pool {
    pub fn initialize(&mut self, token0: Pubkey, token1: Pubkey) -> Result<()> {
        require!(token0 < token1, ErrorCode::InvalidMintOrder);
        self.token0 = token0;
        self.token1 = token1;
        self.reserve0 = 0;
        self.reserve1 = 0;
        self.k_last = 0;
        Ok(())
    }

    pub fn add_liquidity(&mut self, reserve0_added: u64, reserve1_added: u64) -> Result<()> {
        self.reserve0 += reserve0_added;
        self.reserve1 += reserve1_added;

        Ok(())
    }

    pub fn remove_liquidity(&mut self, reserve0_removed: u64, reserve1_removed: u64) -> Result<()> {
        self.reserve0 -= reserve0_removed;
        self.reserve1 -= reserve1_removed;

        Ok(())
    }

    pub fn update_k_last(&mut self) {
        self.k_last = self.reserve0 as u128 * self.reserve1 as u128;
    }
}
