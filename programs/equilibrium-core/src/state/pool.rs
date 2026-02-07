use anchor_lang::prelude::*;

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub enum PoolType {
    Seed,
    Growth,
}

#[account]
pub struct Pool {
    /// Bump seed for PDA
    pub bump: u8,
    
    /// Pool type
    pub pool_type: PoolType,
    
    /// AMM Config this pool belongs to
    pub amm_config: Pubkey,
    
    /// Token mints in the pool
    pub token_mints: Vec<Pubkey>,
    
    /// Token accounts holding reserves
    pub token_accounts: Vec<Pubkey>,
    
    /// Current token reserves
    pub reserves: Vec<u64>,
    
    /// LP token mint
    pub lp_mint: Pubkey,
    
    /// Target weights in basis points (sum = 10000)
    pub target_weights: Vec<u64>,
    
    /// Amplification coefficient
    pub amplification: u64,
    
    /// Total swap fee collected (in LP tokens)
    pub total_fees: u64,
    
    /// Last update timestamp
    pub last_update: i64,
    
    /// If this is a Growth Pool, the Seed Pool it's connected to
    pub seed_pool: Option<Pubkey>,
}

impl Pool {
    pub fn space(num_tokens: usize) -> usize {
        8 + // discriminator
        1 + // bump
        1 + // pool_type
        32 + // amm_config
        4 + (32 * num_tokens) + // token_mints
        4 + (32 * num_tokens) + // token_accounts
        4 + (8 * num_tokens) + // reserves
        32 + // lp_mint
        4 + (8 * num_tokens) + // target_weights
        8 + // amplification
        8 + // total_fees
        8 + // last_update
        1 + 32 // optional seed_pool
    }
}