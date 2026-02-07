use crate::errors::ErrorCode;
use crate::state::*;
use anchor_lang::prelude::*;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token::{self, Mint, Token, TokenAccount, Transfer};

#[derive(Accounts)]
#[instruction(amplification: u64, target_weights: Vec<u64>, initial_amounts: Vec<u64>)]
pub struct CreateSeedPool<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(
        has_one = authority,
    )]
    pub amm_config: Account<'info, AmmConfig>,

    #[account(
        init,
        payer = payer,
        space = Pool::space(3), // Fixed 3 tokens for Seed Pool
        seeds = [&b"pool"[..], &b"seed"[..]],
        bump
    )]
    pub pool: Account<'info, Pool>,

    // We'll need 3 token mints for USDC, USDT, PYUSD
    pub token_mint_a: Account<'info, Mint>,
    pub token_mint_b: Account<'info, Mint>,
    pub token_mint_c: Account<'info, Mint>,

    // Token accounts owned by the user
    #[account(
        mut,
        token::authority = payer,
        token::mint = token_mint_a,
    )]
    pub user_token_a: Account<'info, TokenAccount>,

    #[account(
        mut,
        token::authority = payer,
        token::mint = token_mint_b,
    )]
    pub user_token_b: Account<'info, TokenAccount>,

    #[account(
        mut,
        token::authority = payer,
        token::mint = token_mint_c,
    )]
    pub user_token_c: Account<'info, TokenAccount>,

    // Pool token accounts
    #[account(
        init,
        payer = payer,
        token::mint = token_mint_a,
        token::authority = pool,
        seeds = [&b"pool-token"[..], pool.key().as_ref(), token_mint_a.key().as_ref()],
        bump
    )]
    pub pool_token_a: Account<'info, TokenAccount>,

    #[account(
        init,
        payer = payer,
        token::mint = token_mint_b,
        token::authority = pool,
        seeds = [&b"pool-token"[..], pool.key().as_ref(), token_mint_b.key().as_ref()],
        bump
    )]
    pub pool_token_b: Account<'info, TokenAccount>,

    #[account(
        init,
        payer = payer,
        token::mint = token_mint_c,
        token::authority = pool,
        seeds = [&b"pool-token"[..], pool.key().as_ref(), token_mint_c.key().as_ref()],
        bump
    )]
    pub pool_token_c: Account<'info, TokenAccount>,

    // LP token mint
    #[account(
        init,
        payer = payer,
        mint::authority = pool,
        mint::decimals = 6,
        seeds = [&b"lp-mint"[..], pool.key().as_ref()],
        bump
    )]
    pub lp_mint: Account<'info, Mint>,

    // User's LP token account
    #[account(
        init_if_needed,
        payer = payer,
        associated_token::mint = lp_mint,
        associated_token::authority = payer,
    )]
    pub user_lp_token: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,

    /// CHECK: This is the authority on the AMM config
    pub authority: AccountInfo<'info>,
}

pub fn create_seed_pool(
    ctx: Context<CreateSeedPool>,
    amplification: u64,
    target_weights: Vec<u64>,
    initial_amounts: Vec<u64>,
) -> Result<()> {
    // Validate inputs
    require!(target_weights.len() == 3, ErrorCode::InvalidInputLength);
    require!(initial_amounts.len() == 3, ErrorCode::InvalidInputLength);

    // Validate target weights sum to 10000 (100%)
    let sum: u64 = target_weights.iter().sum();
    require!(sum == 10000, ErrorCode::InvalidWeights);

    // Set up pool state
    let pool = &mut ctx.accounts.pool;
    pool.bump = ctx.bumps.pool;
    pool.pool_type = PoolType::Seed;
    pool.amm_config = ctx.accounts.amm_config.key();

    // Set token mints
    pool.token_mints = vec![
        ctx.accounts.token_mint_a.key(),
        ctx.accounts.token_mint_b.key(),
        ctx.accounts.token_mint_c.key(),
    ];

    // Set pool token accounts
    pool.token_accounts = vec![
        ctx.accounts.pool_token_a.key(),
        ctx.accounts.pool_token_b.key(),
        ctx.accounts.pool_token_c.key(),
    ];

    // Set initial reserves to 0 (will be updated after transfers)
    pool.reserves = vec![0, 0, 0];

    // Set LP mint
    pool.lp_mint = ctx.accounts.lp_mint.key();

    // Set target weights
    pool.target_weights = target_weights;

    // Set amplification coefficient
    pool.amplification = amplification;

    // Initialize other fields
    pool.total_fees = 0;
    pool.last_update = Clock::get()?.unix_timestamp;
    pool.seed_pool = None; // This is a Seed Pool

    // Transfer tokens from user to pool
    let token_accounts = [
        (&ctx.accounts.user_token_a, &ctx.accounts.pool_token_a),
        (&ctx.accounts.user_token_b, &ctx.accounts.pool_token_b),
        (&ctx.accounts.user_token_c, &ctx.accounts.pool_token_c),
    ];

    for (i, (from, to)) in token_accounts.iter().enumerate() {
        let amount = initial_amounts[i];
        if amount > 0 {
            // Transfer tokens from user to pool
            let cpi_accounts = Transfer {
                from: from.to_account_info(),
                to: to.to_account_info(),
                authority: ctx.accounts.payer.to_account_info(),
            };
            let cpi_program = ctx.accounts.token_program.to_account_info();
            let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
            token::transfer(cpi_ctx, amount)?;

            // Update reserves
            pool.reserves[i] = amount;
        }
    }

    // Mint initial LP tokens to user
    // For simplicity, use the sum of token amounts as the initial LP amount
    let initial_lp_amount: u64 = initial_amounts.iter().sum();

    // CPI to mint LP tokens - fixed seed array
    let seeds = &[&b"pool"[..], &b"seed"[..], &[pool.bump]];
    let signer = &[&seeds[..]];

    let cpi_accounts = token::MintTo {
        mint: ctx.accounts.lp_mint.to_account_info(),
        to: ctx.accounts.user_lp_token.to_account_info(),
        authority: ctx.accounts.pool.to_account_info(),
    };
    let cpi_program = ctx.accounts.token_program.to_account_info();
    let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer);
    token::mint_to(cpi_ctx, initial_lp_amount)?;

    Ok(())
}

#[derive(Accounts)]
#[instruction(amplification: u64, initial_usdc_amount: u64, initial_partner_amount: u64)]
pub struct CreateGrowthPool<'info> {
    // Similar to CreateSeedPool but with only 2 tokens
    // Will need reference to the Seed Pool
    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(
        has_one = authority,
    )]
    pub amm_config: Account<'info, AmmConfig>,

    pub seed_pool: Account<'info, Pool>,

    #[account(
        init,
        payer = payer,
        space = Pool::space(2), // Fixed 2 tokens for Growth Pool
        seeds = [&b"pool"[..], &b"growth"[..], partner_token_mint.key().as_ref()],
        bump
    )]
    pub pool: Account<'info, Pool>,

    // USD* from Seed Pool + Partner Token
    pub usdc_star_mint: Account<'info, Mint>,
    pub partner_token_mint: Account<'info, Mint>,

    // Token accounts owned by the user
    #[account(
        mut,
        token::authority = payer,
        token::mint = usdc_star_mint,
    )]
    pub user_usdc_star: Account<'info, TokenAccount>,

    #[account(
        mut,
        token::authority = payer,
        token::mint = partner_token_mint,
    )]
    pub user_partner_token: Account<'info, TokenAccount>,

    // Pool token accounts
    #[account(
        init,
        payer = payer,
        token::mint = usdc_star_mint,
        token::authority = pool,
        seeds = [&b"pool-token"[..], pool.key().as_ref(), usdc_star_mint.key().as_ref()],
        bump
    )]
    pub pool_usdc_star: Account<'info, TokenAccount>,

    #[account(
        init,
        payer = payer,
        token::mint = partner_token_mint,
        token::authority = pool,
        seeds = [&b"pool-token"[..], pool.key().as_ref(), partner_token_mint.key().as_ref()],
        bump
    )]
    pub pool_partner_token: Account<'info, TokenAccount>,

    // LP token mint
    #[account(
        init,
        payer = payer,
        mint::authority = pool,
        mint::decimals = 6,
        seeds = [&b"lp-mint"[..], pool.key().as_ref()],
        bump
    )]
    pub lp_mint: Account<'info, Mint>,

    // User's LP token account
    #[account(
        init_if_needed,
        payer = payer,
        associated_token::mint = lp_mint,
        associated_token::authority = payer,
    )]
    pub user_lp_token: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,

    /// CHECK: This is the authority on the AMM config
    pub authority: AccountInfo<'info>,
}

pub fn create_growth_pool(
    ctx: Context<CreateGrowthPool>,
    amplification: u64,
    initial_usdc_star_amount: u64,
    initial_partner_amount: u64,
) -> Result<()> {
    // Validate inputs
    require!(
        ctx.accounts.seed_pool.pool_type == PoolType::Seed,
        ErrorCode::InvalidPoolType
    );

    // Set up pool state
    let pool = &mut ctx.accounts.pool;
    pool.bump = ctx.bumps.pool;
    pool.pool_type = PoolType::Growth;
    pool.amm_config = ctx.accounts.amm_config.key();

    // Set token mints
    pool.token_mints = vec![
        ctx.accounts.usdc_star_mint.key(),
        ctx.accounts.partner_token_mint.key(),
    ];

    // Set pool token accounts
    pool.token_accounts = vec![
        ctx.accounts.pool_usdc_star.key(),
        ctx.accounts.pool_partner_token.key(),
    ];

    // Set initial reserves to 0 (will be updated after transfers)
    pool.reserves = vec![0, 0];

    // Set LP mint
    pool.lp_mint = ctx.accounts.lp_mint.key();

    // Set target weights - for Growth Pool it's always 50/50
    pool.target_weights = vec![5000, 5000];

    // Set amplification coefficient
    pool.amplification = amplification;

    // Initialize other fields
    pool.total_fees = 0;
    pool.last_update = Clock::get()?.unix_timestamp;
    pool.seed_pool = Some(ctx.accounts.seed_pool.key());

    // Transfer tokens from user to pool
    // Transfer USD*
    if initial_usdc_star_amount > 0 {
        let cpi_accounts = Transfer {
            from: ctx.accounts.user_usdc_star.to_account_info(),
            to: ctx.accounts.pool_usdc_star.to_account_info(),
            authority: ctx.accounts.payer.to_account_info(),
        };
        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
        token::transfer(cpi_ctx, initial_usdc_star_amount)?;

        // Update reserves
        pool.reserves[0] = initial_usdc_star_amount;
    }

    // Transfer partner token
    if initial_partner_amount > 0 {
        let cpi_accounts = Transfer {
            from: ctx.accounts.user_partner_token.to_account_info(),
            to: ctx.accounts.pool_partner_token.to_account_info(),
            authority: ctx.accounts.payer.to_account_info(),
        };
        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
        token::transfer(cpi_ctx, initial_partner_amount)?;

        // Update reserves
        pool.reserves[1] = initial_partner_amount;
    }

    // Mint initial LP tokens to user
    let initial_lp_amount = std::cmp::min(initial_usdc_star_amount, initial_partner_amount) * 2;

    // CPI to mint LP tokens - fixed seed array
    let partner_token_key = ctx.accounts.partner_token_mint.key();
    let partner_token_ref = partner_token_key.as_ref();
    let seeds = &[
        &b"pool"[..],
        &b"growth"[..],
        partner_token_ref,
        &[pool.bump],
    ];
    let signer = &[&seeds[..]];

    let cpi_accounts = token::MintTo {
        mint: ctx.accounts.lp_mint.to_account_info(),
        to: ctx.accounts.user_lp_token.to_account_info(),
        authority: ctx.accounts.pool.to_account_info(),
    };
    let cpi_program = ctx.accounts.token_program.to_account_info();
    let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer);
    token::mint_to(cpi_ctx, initial_lp_amount)?;

    Ok(())
}
