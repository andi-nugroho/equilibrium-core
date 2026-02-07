//! Mathematical models for the Equilibrium AMM

// Remove unused import
use std::cmp;

// Constants for fee calculation
pub const BASE_FEE: u64 = 1; // 0.1% = 1/1000
pub const MAX_FEE: u64 = 5; // 0.5% = 5/1000
pub const FEE_MULTIPLIER: u64 = 1; // 0.1% = 1/1000 per unit of deviation
pub const FEE_DENOMINATOR: u64 = 1000; // Fees are expressed as x/1000

// Constants for liquidity concentration
pub const MIN_PRICE: u64 = 995; // 0.995
pub const MAX_PRICE: u64 = 1005; // 1.005
pub const PRICE_DENOMINATOR: u64 = 1000; // Prices are expressed as x/1000

/// Calculate dynamic swap fee based on weight deviations
///
/// Takes current_weights and target_weights (both in basis points where 10000 = 100%)
/// # Returns
/// * Fee in parts per 1000 (e.g., 1 = 0.1%)
pub fn calculate_dynamic_fee(current_weights: &[u64], target_weights: &[u64]) -> u64 {
    let mut total_deviation = 0;

    // Calculate total absolute deviation from target weights
    for (current, target) in current_weights.iter().zip(target_weights.iter()) {
        let deviation = if current > target {
            current - target
        } else {
            target - current
        };
        total_deviation += deviation;
    }

    // Convert basis points to percentage points for fee calculation
    // 10000 basis points = 100%, so divide by 100 to get deviation as percentage points
    let deviation_percentage = total_deviation / 100;

    // Calculate fee: BASE_FEE + deviation * FEE_MULTIPLIER, capped at MAX_FEE
    let fee = BASE_FEE + (deviation_percentage * FEE_MULTIPLIER) / 10;
    cmp::min(fee, MAX_FEE)
}

/// StableSwap invariant calculator
/// Based on the formula: An^n * sum(x_i) + D = An^n * D + D^(n+1) / (n^n * prod(x_i))
/// Simplified for stablecoins near parity
///
/// # Arguments
/// * `amounts` - Token amounts in the pool
/// * `amplification` - Amplification coefficient (higher = closer to constant sum, lower = closer to constant product)
///
/// # Returns
/// * The invariant D
pub fn calculate_invariant(amounts: &[u64], amplification: u64) -> Option<u64> {
    if amounts.is_empty() {
        return None;
    }

    let n = amounts.len() as u64;
    let mut sum = 0;
    let mut _product = 1; // Mark as intentionally unused with underscore
    let mut has_zero = false;

    for &amount in amounts {
        if amount == 0 {
            has_zero = true;
            break;
        }
        sum += amount;
        _product *= amount; // Not used but kept for algorithm clarity
    }

    if has_zero || sum == 0 {
        return None;
    }

    // D cannot be less than the sum in the worst case (constant sum)
    let mut d = sum;

    // A * n^n
    let ann = amplification * n.pow(n as u32);

    // Newton's method to approximate D
    for _ in 0..255 {
        let d_prev = d; // Remove mut as it's not modified

        // D / (A * n^n)
        let d_p = d;

        // Calculate f(D) using the formula
        let mut d_product = d;
        for &amount in amounts {
            d_product = d_product * d / (amount * n);
        }

        // Newton iteration: D = (A * n^n * sum + D_P * n) * D / ((A * n^n - 1) * D + (n + 1) * D_P)
        d = (ann * sum + d_p * n) * d / ((ann - 1) * d + (n + 1) * d_p);

        // Check for convergence with precision of 1
        if d > d_prev {
            if d - d_prev <= 1 {
                break;
            }
        } else if d_prev - d <= 1 {
            break;
        }
    }

    Some(d)
}

/// Calculate output amount for a swap
///
/// # Arguments
/// * `x_amount` - Input token amount
/// * `x_reserve` - Input token reserve
/// * `y_reserve` - Output token reserve  
/// * `fee` - Fee in parts per 1000
/// * `amplification` - Amplification coefficient
///
/// # Returns
/// * Output amount after fees
pub fn calculate_output_amount(
    x_amount: u64,
    x_reserve: u64,
    y_reserve: u64,
    fee: u64,
    amplification: u64,
) -> Option<u64> {
    if x_reserve == 0 || y_reserve == 0 {
        return None;
    }

    // Calculate invariant before swap
    let amounts = vec![x_reserve, y_reserve];
    let d = calculate_invariant(&amounts, amplification)?;

    // Apply fee to input amount
    let fee_amount = (x_amount * fee) / FEE_DENOMINATOR;
    let x_amount_after_fee = x_amount - fee_amount;

    // New input reserve after swap
    let new_x_reserve = x_reserve + x_amount_after_fee;

    // Find new_y_reserve such that invariant is preserved
    // Solve for: A * n^n * (new_x_reserve + new_y_reserve) + D = A * n^n * D + D^(n+1) / (n^n * new_x_reserve * new_y_reserve)

    // Simplified for 2 tokens (n=2)
    let ann = amplification * 4; // A * n^n for n=2

    // Fix the negative u64 issue by rearranging the formula
    // Instead of: let c = -(d.pow(3)) / (4 * ann * new_x_reserve);
    let c_positive = d.pow(3) / (4 * ann * new_x_reserve);

    // Quadratic formula parameters: a * y^2 + b * y - c = 0
    let a = 1;
    let b = ann * d / (ann * new_x_reserve);

    // Calculate discriminant
    let discriminant = b * b + 4 * a * c_positive; // Changed to + for the rearranged equation
    if discriminant < 0 {
        return None; // This should never happen with our rearrangement
    }

    // Use quadratic formula, taking the smaller root
    let sqrt_discriminant = (discriminant as f64).sqrt() as u64;
    let new_y_reserve = (b - sqrt_discriminant) / (2 * a);

    // Calculate output amount
    let y_amount = y_reserve - new_y_reserve;

    Some(y_amount)
}

/// Calculate current weights of tokens in the pool
///
/// # Arguments
/// * `reserves` - Current token reserves
///
/// # Returns
/// * Weights in basis points (sum = 10000)
pub fn calculate_weights(reserves: &[u64]) -> Vec<u64> {
    let total: u64 = reserves.iter().sum();
    if total == 0 {
        return vec![0; reserves.len()];
    }

    reserves
        .iter()
        .map(|&reserve| (reserve * 10000) / total)
        .collect()
}

/// Calculate position bounds based on concentration factor
///
/// # Arguments
/// * `center_price` - Center price in price_denominator units (typically 1000)
/// * `concentration` - Number of 0.005 increments to use
///
/// # Returns
/// * (min_price, max_price) in price_denominator units
pub fn calculate_position_bounds(center_price: u64, concentration: u64) -> (u64, u64) {
    let increment = 5; // 0.005 * PRICE_DENOMINATOR
    let half_range = concentration * increment;

    let min_price = center_price.saturating_sub(half_range);
    let max_price = center_price + half_range;

    (min_price, max_price)
}
