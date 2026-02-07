use anchor_lang::prelude::*;
use anchor_spl::token::TokenAccount;
use crate::state::*;
use crate::errors::ErrorCode;

/// Verify token account belongs to the expected owner and has the expected mint
pub fn verify_token_account(
    token_account: &AccountInfo,
    expected_owner: &Pubkey,
    expected_mint: &Pubkey,
) -> Result<()> {
    let token_account_data = TokenAccount::try_deserialize(&mut &token_account.data.borrow()[..])?;
    require!(
        token_account_data.owner == *expected_owner,
        ErrorCode::Unauthorized
    );
    require!(
        token_account_data.mint == *expected_mint,
        ErrorCode::InvalidTokenMint
    );
    Ok(())
}

/// Get seeds for pool signing
pub fn get_pool_signer_seeds<'a>(
    pool: &'a Pool,
    partner_token_mint: Option<&'a [u8]>,
    bump: &'a [u8],
) -> Vec<&'a [u8]> {
    let mut seeds = vec![
        &b"pool"[..],
        if pool.pool_type == PoolType::Seed { &b"seed"[..] } else { &b"growth"[..] },
    ];
    
    if pool.pool_type == PoolType::Growth && partner_token_mint.is_some() {
        seeds.push(partner_token_mint.unwrap());
    }
    
    seeds.push(bump);
    
    seeds
}

/// Format basis points (10000 = 100%) as a percentage string
pub fn format_basis_points(basis_points: u64) -> String {
    let whole = basis_points / 100;
    let fraction = basis_points % 100;
    format!("{}.{:02}%", whole, fraction)
}

/// Calculate fee in readable format (e.g. 0.1% to 0.5%)
pub fn calculate_fee_percentage(fee: u64) -> String {
    format!("0.{}%", fee / 10)
}

/// Log pool statistics
pub fn log_pool_stats(pool: &Pool) {
    let weights = crate::state::math::calculate_weights(&pool.reserves);
    let fee = crate::state::math::calculate_dynamic_fee(&weights, &pool.target_weights);
    
    msg!("Pool type: {:?}", pool.pool_type);
    msg!("Current reserves: {:?}", pool.reserves);
    msg!("Current weights: {:?}", weights);
    msg!("Target weights: {:?}", pool.target_weights);
    msg!("Dynamic fee: {}", calculate_fee_percentage(fee));
    msg!("Amplification coefficient: {}", pool.amplification);
}

/// Calculate dollar value of a token amount using a price oracle
/// Note: In a real implementation, you would integrate with a price oracle
pub fn calculate_dollar_value(amount: u64, decimals: u8) -> f64 {
    // Simplified calculation assuming 1:1 peg for stablecoins
    (amount as f64) / (10u64.pow(decimals as u32) as f64)
}

/// Calculate the capital efficiency of a position based on bounds
pub fn calculate_capital_efficiency(min_price: u64, max_price: u64) -> f64 {
    // The narrower the range, the higher the capital efficiency
    let range_width = max_price as f64 - min_price as f64;
    let theoretical_max_range = crate::state::math::MAX_PRICE as f64 - crate::state::math::MIN_PRICE as f64;
    
    // Calculate ratio compared to theoretical maximum range
    (theoretical_max_range / range_width) * 100.0
}