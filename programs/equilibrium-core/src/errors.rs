use anchor_lang::prelude::*;

#[error_code]
pub enum ErrorCode {
    #[msg("Invalid instruction data")]
    InvalidInstructionData,
    
    #[msg("Math overflow")]
    MathOverflow,
    
    #[msg("Slippage tolerance exceeded")]
    SlippageExceeded,
    
    #[msg("Invalid token mint")]
    InvalidTokenMint,
    
    #[msg("Invalid weights, must sum to 10000")]
    InvalidWeights,
    
    #[msg("Invalid input length")]
    InvalidInputLength,
    
    #[msg("Invalid pool type")]
    InvalidPoolType,
    
    #[msg("Invalid swap")]
    InvalidSwap,
    
    #[msg("Invalid position bounds")]
    InvalidPositionBounds,
    
    #[msg("Insufficient liquidity")]
    InsufficientLiquidity,
    
    #[msg("Invalid amplification coefficient")]
    InvalidAmplification,
    
    #[msg("Position not active")]
    PositionNotActive,
    
    #[msg("Unauthorized")]
    Unauthorized,
}