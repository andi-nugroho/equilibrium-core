use crate::errors::ErrorCode;
use crate::state::*;
use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, Token, TokenAccount, Transfer};

#[derive(Accounts)]
#[instruction(amounts: Vec<u64>, min_lp_amount: u64, concentration: u64)]
pub struct Deposit<'info> {
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

    // Token accounts owned by the user - we'll handle different pool types
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
        owner = user.key(),
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

    // User position for concentrated liquidity
    #[account(
        init_if_needed,
        payer = user,
        space = UserPosition::space(),
        seeds = [&b"user-position"[..], user.key().as_ref(), pool.key().as_ref()],
        bump
    )]
    pub user_position: Account<'info, UserPosition>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

pub fn handler(
    ctx: Context<Deposit>,
    amounts: Vec<u64>,
    min_lp_amount: u64,
    concentration: u64,
) -> Result<()> {
    // Get the pool key first to avoid borrow conflicts later
    let pool_key = ctx.accounts.pool.key();

    // Now mutably borrow pool
    let pool = &mut ctx.accounts.pool;

    // Extract all the data we need from the pool to avoid borrow conflicts later
    let pool_type = pool.pool_type;
    let token_mints = pool.token_mints.clone();
    let old_reserves = pool.reserves.clone();
    let amplification = pool.amplification;
    let pool_bump = pool.bump;

    match pool_type {
        PoolType::Seed => {
            require!(amounts.len() == 3, ErrorCode::InvalidInputLength);
            require!(
                ctx.accounts.token_mint_c.is_some(),
                ErrorCode::InvalidTokenMint
            );
            require!(
                ctx.accounts.user_token_c.is_some(),
                ErrorCode::InvalidTokenMint
            );
            require!(
                ctx.accounts.pool_token_c.is_some(),
                ErrorCode::InvalidTokenMint
            );
        }
        PoolType::Growth => {
            require!(amounts.len() == 2, ErrorCode::InvalidInputLength);
        }
    }

    // Verify token mints match pool configuration
    require!(
        ctx.accounts.token_mint_a.key() == token_mints[0],
        ErrorCode::InvalidTokenMint
    );

    require!(
        ctx.accounts.token_mint_b.key() == token_mints[1],
        ErrorCode::InvalidTokenMint
    );

    if pool_type == PoolType::Seed {
        require!(
            ctx.accounts.token_mint_c.as_ref().unwrap().key() == token_mints[2],
            ErrorCode::InvalidTokenMint
        );
    }

    // Transfer tokens from user to pool
    let total_old_reserves = old_reserves.iter().sum::<u64>();

    // Handle different pool types
    if pool_type == PoolType::Seed {
        // For Seed Pool, handle 3 tokens
        let token_accounts = [
            (
                &ctx.accounts.user_token_a,
                &ctx.accounts.pool_token_a,
                amounts[0],
            ),
            (
                &ctx.accounts.user_token_b,
                &ctx.accounts.pool_token_b,
                amounts[1],
            ),
            (
                ctx.accounts.user_token_c.as_ref().unwrap(),
                ctx.accounts.pool_token_c.as_ref().unwrap(),
                amounts[2],
            ),
        ];

        // Process each token transfer
        for (i, (from, to, amount)) in token_accounts.iter().enumerate() {
            if *amount > 0 {
                let cpi_accounts = Transfer {
                    from: from.to_account_info(),
                    to: to.to_account_info(),
                    authority: ctx.accounts.user.to_account_info(),
                };
                let cpi_program = ctx.accounts.token_program.to_account_info();
                let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
                token::transfer(cpi_ctx, *amount)?;

                // Update reserves
                pool.reserves[i] += amount;
            }
        }
    } else {
        // For Growth Pool, handle 2 tokens
        let token_accounts = [
            (
                &ctx.accounts.user_token_a,
                &ctx.accounts.pool_token_a,
                amounts[0],
            ),
            (
                &ctx.accounts.user_token_b,
                &ctx.accounts.pool_token_b,
                amounts[1],
            ),
        ];

        // Process each token transfer
        for (i, (from, to, amount)) in token_accounts.iter().enumerate() {
            if *amount > 0 {
                let cpi_accounts = Transfer {
                    from: from.to_account_info(),
                    to: to.to_account_info(),
                    authority: ctx.accounts.user.to_account_info(),
                };
                let cpi_program = ctx.accounts.token_program.to_account_info();
                let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
                token::transfer(cpi_ctx, *amount)?;

                // Update reserves
                pool.reserves[i] += amount;
            }
        }
    }

    // Calculate LP tokens to mint based on the invariant increase
    let lp_amount: u64;

    if total_old_reserves == 0 {
        // Initial deposit - for simplicity, use the sum
        lp_amount = amounts.iter().sum();
    } else {
        // Calculate based on invariant
        let old_d = crate::state::math::calculate_invariant(&old_reserves, amplification)
            .ok_or(ErrorCode::MathOverflow)?;

        let new_d = crate::state::math::calculate_invariant(&pool.reserves, amplification)
            .ok_or(ErrorCode::MathOverflow)?;

        // LP tokens minted proportional to invariant growth
        let lp_supply = ctx.accounts.lp_mint.supply;
        lp_amount = (lp_supply as u128 * (new_d - old_d) as u128 / old_d as u128) as u64;
    }

    // Check minimum LP amount
    require!(lp_amount >= min_lp_amount, ErrorCode::SlippageExceeded);

    // Now prepare the seeds for the CPI call
    let partner_token_mint_ref = if pool_type == PoolType::Growth {
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

    // Clone the pool account info to avoid borrow conflicts
    let pool_account_info = ctx.accounts.pool.to_account_info();

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

    // Mint LP tokens to user
    let cpi_accounts = token::MintTo {
        mint: ctx.accounts.lp_mint.to_account_info(),
        to: ctx.accounts.user_lp_token.to_account_info(),
        authority: pool_account_info,
    };
    let cpi_program = ctx.accounts.token_program.to_account_info();
    let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer);
    token::mint_to(cpi_ctx, lp_amount)?;

    // Initialize user position if it's new
    if ctx.accounts.user_position.owner == Pubkey::default() {
        let user_position = &mut ctx.accounts.user_position;
        user_position.bump = ctx.bumps.user_position;
        user_position.owner = ctx.accounts.user.key();
        user_position.pool = pool_key;
        user_position.created_at = Clock::get()?.unix_timestamp;
    }

    // Update position
    let user_position = &mut ctx.accounts.user_position;
    user_position.lp_amount += lp_amount;
    user_position.min_price = concentration.saturating_sub(1000); // Lower bound = concentration - 10%
    user_position.max_price = concentration.saturating_add(1000); // Upper bound = concentration + 10%
    user_position.is_active = true;
    user_position.last_update = Clock::get()?.unix_timestamp;

    Ok(())
}
