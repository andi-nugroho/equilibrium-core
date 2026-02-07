use crate::errors::ErrorCode;
use crate::state::*;
use anchor_lang::prelude::*;
use anchor_spl::token::{self, Burn, Mint, Token, TokenAccount, Transfer};

#[derive(Accounts)]
#[instruction(lp_amount: u64, min_amounts: Vec<u64>)]
pub struct Withdraw<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(mut)]
    pub pool: Account<'info, Pool>,

    // LP token mint
    #[account(
        mut,
        constraint = lp_mint.key() == pool.lp_mint
    )]
    pub lp_mint: Account<'info, Mint>,

    // User's LP token account
    #[account(
        mut,
        token::authority = user,
        token::mint = lp_mint,
    )]
    pub user_lp_token: Account<'info, TokenAccount>,

    // Token accounts for receiving withdrawn assets
    #[account(
        mut,
        token::authority = user,
        token::mint = token_mint_a,
    )]
    pub user_token_a: Account<'info, TokenAccount>,

    #[account(
        mut,
        token::authority = user,
        token::mint = token_mint_b,
    )]
    pub user_token_b: Account<'info, TokenAccount>,

    #[account(
        mut,
        token::authority = user,
        token::mint = token_mint_c,
    )]
    pub user_token_c: Option<Account<'info, TokenAccount>>,

    // Token mints - must match the order in pool.token_mints
    pub token_mint_a: Account<'info, Mint>,
    pub token_mint_b: Account<'info, Mint>,
    pub token_mint_c: Option<Account<'info, Mint>>,

    // Pool token accounts
    #[account(
        mut,
        token::authority = pool,
        token::mint = token_mint_a,
    )]
    pub pool_token_a: Account<'info, TokenAccount>,

    #[account(
        mut,
        token::authority = pool,
        token::mint = token_mint_b,
    )]
    pub pool_token_b: Account<'info, TokenAccount>,

    #[account(
        mut,
        token::authority = pool,
        token::mint = token_mint_c,
    )]
    pub pool_token_c: Option<Account<'info, TokenAccount>>,

    // User position
    #[account(
        mut,
        seeds = [&b"user-position"[..], user.key().as_ref(), pool.key().as_ref()],
        bump = user_position.bump,
        constraint = user_position.owner == user.key() @ ErrorCode::Unauthorized,
        constraint = user_position.pool == pool.key() @ ErrorCode::InvalidPoolType,
        constraint = user_position.is_active @ ErrorCode::PositionNotActive,
    )]
    pub user_position: Account<'info, UserPosition>,

    pub token_program: Program<'info, Token>,
}

pub fn handler(ctx: Context<Withdraw>, lp_amount: u64, min_amounts: Vec<u64>) -> Result<()> {
    // Extract pool information first to avoid borrow conflicts
    let pool_key = ctx.accounts.pool.key();
    let pool_account_info = ctx.accounts.pool.to_account_info();

    // Now use mutable borrow
    let pool = &mut ctx.accounts.pool;

    // Validate inputs based on pool type
    match pool.pool_type {
        PoolType::Seed => {
            require!(min_amounts.len() == 3, ErrorCode::InvalidInputLength);
        }
        PoolType::Growth => {
            require!(min_amounts.len() == 2, ErrorCode::InvalidInputLength);
        }
    }

    // Get data needed for calculations
    let pool_type = pool.pool_type;
    let pool_reserves = pool.reserves.clone();
    let pool_amplification = pool.amplification;
    let pool_bump = pool.bump;
    let token_mints = pool.token_mints.clone();
    let total_lp_supply = ctx.accounts.lp_mint.supply;

    // Validate user has enough LP tokens
    require!(
        ctx.accounts.user_position.lp_amount >= lp_amount,
        ErrorCode::InsufficientLiquidity
    );

    // Calculate withdrawal amounts
    let withdraw_amounts =
        calculate_withdrawal_amounts(&pool_reserves, lp_amount, total_lp_supply, &min_amounts)?;

    // Burn LP tokens
    let cpi_accounts = Burn {
        mint: ctx.accounts.lp_mint.to_account_info(),
        from: ctx.accounts.user_lp_token.to_account_info(),
        authority: ctx.accounts.user.to_account_info(),
    };
    let cpi_program = ctx.accounts.token_program.to_account_info();
    let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
    token::burn(cpi_ctx, lp_amount)?;

    // Transfer tokens from pool to user - fixed seed array handling
    // Prepare the seeds for token transfers
    let partner_token_mint_ref = if pool_type == PoolType::Growth {
        // For Growth pools, we need the partner token mint as part of the seeds
        Some(token_mints[1].as_ref())
    } else {
        None
    };

    let seed_type = if pool_type == PoolType::Seed {
        &b"seed"[..]
    } else {
        &b"growth"[..]
    };

    // Store the seeds in longer-lived variables
    let seed_pool = &b"pool"[..];

    let seeds_with_partner = [
        seed_pool,
        seed_type,
        partner_token_mint_ref.unwrap_or(&[]),
        &[pool_bump],
    ];
    let seeds_without_partner = [seed_pool, seed_type, &[pool_bump]];

    let seeds = match partner_token_mint_ref {
        Some(_) => &seeds_with_partner[..],
        None => &seeds_without_partner[..],
    };

    let signer = &[seeds];

    // Track reserve updates
    let mut updated_reserves = pool_reserves.clone();

    if pool_type == PoolType::Seed {
        // For Seed Pool, handle 3 tokens
        let token_accounts = [
            (
                &ctx.accounts.pool_token_a,
                &ctx.accounts.user_token_a,
                withdraw_amounts[0],
            ),
            (
                &ctx.accounts.pool_token_b,
                &ctx.accounts.user_token_b,
                withdraw_amounts[1],
            ),
            (
                ctx.accounts.pool_token_c.as_ref().unwrap(),
                ctx.accounts.user_token_c.as_ref().unwrap(),
                withdraw_amounts[2],
            ),
        ];

        // Process each token transfer and update reserves
        for (i, (from, to, amount)) in token_accounts.iter().enumerate() {
            if *amount > 0 {
                let cpi_accounts = Transfer {
                    from: from.to_account_info(),
                    to: to.to_account_info(),
                    authority: pool_account_info.clone(),
                };
                let cpi_program = ctx.accounts.token_program.to_account_info();
                let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer);
                token::transfer(cpi_ctx, *amount)?;

                // Update the tracking array
                updated_reserves[i] = updated_reserves[i].saturating_sub(*amount);
            }
        }
    } else {
        // For Growth Pool, handle 2 tokens
        let token_accounts = [
            (
                &ctx.accounts.pool_token_a,
                &ctx.accounts.user_token_a,
                withdraw_amounts[0],
            ),
            (
                &ctx.accounts.pool_token_b,
                &ctx.accounts.user_token_b,
                withdraw_amounts[1],
            ),
        ];

        // Process each token transfer and update reserves
        for (i, (from, to, amount)) in token_accounts.iter().enumerate() {
            if *amount > 0 {
                let cpi_accounts = Transfer {
                    from: from.to_account_info(),
                    to: to.to_account_info(),
                    authority: pool_account_info.clone(),
                };
                let cpi_program = ctx.accounts.token_program.to_account_info();
                let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer);
                token::transfer(cpi_ctx, *amount)?;

                // Update the tracking array
                updated_reserves[i] = updated_reserves[i].saturating_sub(*amount);
            }
        }
    }

    // Now update the pool reserves
    pool.reserves = updated_reserves;

    // Update user position
    let user_position = &mut ctx.accounts.user_position;
    user_position.lp_amount = user_position.lp_amount.saturating_sub(lp_amount);
    user_position.last_update = Clock::get()?.unix_timestamp;

    // If lp_amount is 0, mark position as inactive
    if user_position.lp_amount == 0 {
        user_position.is_active = false;
    }

    // Update pool last update timestamp
    pool.last_update = Clock::get()?.unix_timestamp;

    Ok(())
}

// Helper function to calculate withdrawal amounts
fn calculate_withdrawal_amounts(
    reserves: &[u64],
    lp_amount: u64,
    total_lp_supply: u64,
    min_amounts: &[u64],
) -> Result<Vec<u64>> {
    // Calculate token amounts to withdraw based on share of pool
    let withdraw_ratio = (lp_amount as u128 * 10000) as u128 / total_lp_supply as u128;

    let mut withdraw_amounts = Vec::new();
    for (i, &reserve) in reserves.iter().enumerate() {
        let amount = (reserve as u128 * withdraw_ratio / 10000) as u64;
        withdraw_amounts.push(amount);

        // Check minimum amounts
        if i < min_amounts.len() {
            require!(amount >= min_amounts[i], ErrorCode::SlippageExceeded);
        }
    }

    Ok(withdraw_amounts)
}
