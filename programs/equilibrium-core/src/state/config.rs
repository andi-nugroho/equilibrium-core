use anchor_lang::prelude::*;

#[account]
#[derive(Default)]
pub struct AmmConfig {
    /// Bump seed for PDA
    pub bump: u8,
    
    /// Authority that can update the config
    pub authority: Pubkey,
    
    /// Fees recipient
    pub fee_recipient: Pubkey,
    
    /// Default amplification coefficient (higher = closer to constant sum)
    pub default_amplification: u64,
    
    /// Default target weights for the Seed Pool (in basis points, sum must be 10000)
    /// [USDC weight, USDT weight, PYUSD weight]
    pub default_target_weights: [u64; 3],
}