use crate::errors::ErrorCode;
use crate::state::*;
use anchor_lang::prelude::*;

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(
        init,
        payer = authority,
        space = 8 + 1 + 32 + 32 + 8 + (3 * 8), // anchor discriminator + bump + authority + fee_recipient + amplification + target_weights
        seeds = [&b"amm-config"[..]],
        bump
    )]
    pub amm_config: Account<'info, AmmConfig>,

    pub system_program: Program<'info, System>,
}

pub fn handler(
    ctx: Context<Initialize>,
    default_amplification: u64,
    default_target_weights: [u64; 3],
) -> Result<()> {
    let amm_config = &mut ctx.accounts.amm_config;

    // Validate target weights sum to 10000 (100%)
    let sum: u64 = default_target_weights.iter().sum();
    require!(sum == 10000, ErrorCode::InvalidWeights);

    // Set config values - using the new direct bump access
    amm_config.bump = ctx.bumps.amm_config;
    amm_config.authority = ctx.accounts.authority.key();
    amm_config.fee_recipient = ctx.accounts.authority.key(); // Initially set to authority
    amm_config.default_amplification = default_amplification;
    amm_config.default_target_weights = default_target_weights;

    Ok(())
}
