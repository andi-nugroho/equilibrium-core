use crate::errors::ErrorCode;
use crate::state::*;
use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, Token, TokenAccount, Transfer};

#[derive(Accounts)]
#[instruction(amount_in: u64, min_amount_out: u64)]
pub struct Swap<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(mut)]
    pub pool: Account<'info, Pool>,

    // Token being sent to the pool
    pub token_mint_in: Account<'info, Mint>,

    // Token being received from the pool
    pub token_mint_out: Account<'info, Mint>,

    // User's token accounts
    #[account(
        mut,
        token::authority = user,
        token::mint = token_mint_in,
    )]
    pub user_token_in: Account<'info, TokenAccount>,

    #[account(
        mut,
        token::authority = user,
        token::mint = token_mint_out,
    )]
    pub user_token_out: Account<'info, TokenAccount>,

    // Pool's token accounts
    #[account(
        mut,
        token::authority = pool,
        token::mint = token_mint_in,
    )]
    pub pool_token_in: Account<'info, TokenAccount>,

    #[account(
        mut,
        token::authority = pool,
        token::mint = token_mint_out,
    )]
    pub pool_token_out: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
}

pub fn handler(ctx: Context<Swap>, amount_in: u64, min_amount_out: u64) -> Result<()> {
    // Extract pool information first to avoid borrow conflicts
    let pool_key = ctx.accounts.pool.key();
    let pool_account_info = ctx.accounts.pool.to_account_info();

    // Now use mutable borrow
    let pool = &mut ctx.accounts.pool;

    // Find the token indices
    let token_in_idx = pool
        .token_mints
        .iter()
        .position(|mint| mint == &ctx.accounts.token_mint_in.key())
        .ok_or(ErrorCode::InvalidTokenMint)?;

    let token_out_idx = pool
        .token_mints
        .iter()
        .position(|mint| mint == &ctx.accounts.token_mint_out.key())
        .ok_or(ErrorCode::InvalidTokenMint)?;

    // Capture values we'll need later
    let pool_type = pool.pool_type;
    let pool_reserves = pool.reserves.clone();
    let pool_amplification = pool.amplification;
    let pool_bump = pool.bump;
    let token_mints = pool.token_mints.clone();

    // Get current reserves
    let in_reserve = pool_reserves[token_in_idx];
    let out_reserve = pool_reserves[token_out_idx];

    // Calculate current weights
    let current_weights = crate::state::math::calculate_weights(&pool_reserves);

    // Calculate dynamic fee based on weight deviation
    let fee = crate::state::math::calculate_dynamic_fee(&current_weights, &pool.target_weights);

    // Calculate output amount
    let amount_out = crate::state::math::calculate_output_amount(
        amount_in,
        in_reserve,
        out_reserve,
        fee,
        pool_amplification,
    )
    .ok_or(ErrorCode::InvalidSwap)?;

    // Check minimum output amount
    require!(amount_out >= min_amount_out, ErrorCode::SlippageExceeded);

    // Transfer tokens from user to pool
    let cpi_accounts_in = Transfer {
        from: ctx.accounts.user_token_in.to_account_info(),
        to: ctx.accounts.pool_token_in.to_account_info(),
        authority: ctx.accounts.user.to_account_info(),
    };
    let cpi_program = ctx.accounts.token_program.to_account_info();
    let cpi_ctx_in = CpiContext::new(cpi_program.clone(), cpi_accounts_in);
    token::transfer(cpi_ctx_in, amount_in)?;

    // Transfer tokens from pool to user - fixed seed array handling
    let partner_token_mint_ref = if pool_type == PoolType::Growth {
        // For Growth pools, we need the partner token mint as part of the seeds
        let partner_idx = if token_in_idx == 0 { 1 } else { 0 };
        Some(token_mints[partner_idx].as_ref())
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

    let cpi_accounts_out = Transfer {
        from: ctx.accounts.pool_token_out.to_account_info(),
        to: ctx.accounts.user_token_out.to_account_info(),
        authority: pool_account_info,
    };
    let cpi_ctx_out = CpiContext::new_with_signer(cpi_program, cpi_accounts_out, signer);
    token::transfer(cpi_ctx_out, amount_out)?;

    // Update pool reserves
    pool.reserves[token_in_idx] += amount_in;
    pool.reserves[token_out_idx] = pool.reserves[token_out_idx].saturating_sub(amount_out);

    // Update pool last update timestamp
    pool.last_update = Clock::get()?.unix_timestamp;

    Ok(())
}
