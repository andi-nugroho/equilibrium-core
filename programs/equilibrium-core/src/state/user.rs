use anchor_lang::prelude::*;

#[account]
pub struct UserPosition {
    /// Bump seed for PDA
    pub bump: u8,
    
    /// User wallet
    pub owner: Pubkey,
    
    /// Pool this position belongs to
    pub pool: Pubkey,
    
    /// LP token amount
    pub lp_amount: u64,
    
    /// Min price boundary (in price_denominator units)
    pub min_price: u64,
    
    /// Max price boundary (in price_denominator units)
    pub max_price: u64,
    
    /// If position is currently collecting fees
    pub is_active: bool,
    
    /// Creation timestamp
    pub created_at: i64,
    
    /// Last update timestamp
    pub last_update: i64,
}

impl UserPosition {
    pub fn space() -> usize {
        8 + // discriminator
        1 + // bump
        32 + // owner
        32 + // pool
        8 + // lp_amount
        8 + // min_price
        8 + // max_price
        1 + // is_active
        8 + // created_at
        8 // last_update
    }
}