use anchor_lang::prelude::*;

#[error_code]
pub enum ErrorCode {
    #[msg("InvalidFee")]
    InvalidFee,
    #[msg("InvalidMintOrder")]
    InvalidMintOrder,
    #[msg("InsufficientAmount")]
    InsufficientAmount,
    #[msg("InsufficientReserves")]
    InsufficientReserves,
    #[msg("InsufficientLiquidityMinted")]
    InsufficientLiquidityMinted,
    #[msg("InsufficientLiquidityBurned")]
    InsufficientLiquidityBurned,
}
