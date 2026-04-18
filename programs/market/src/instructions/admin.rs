use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token, TokenAccount};

use crate::constants::{MARKET_CONFIG_SPACE, GAME_STATE_PROGRAM_ID};
use crate::error::MarketError;
use crate::state::MarketConfig;

#[derive(Accounts)]
pub struct InitializeMarket<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    #[account(init, payer = admin, space = MARKET_CONFIG_SPACE, seeds = [b"market_config"], bump)]
    pub market_config: Account<'info, MarketConfig>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct InitializeEscrow<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    #[account(seeds = [b"market_config"], bump = market_config.bump, has_one = admin @ MarketError::Unauthorized)]
    pub market_config: Account<'info, MarketConfig>,
    #[account(address = market_config.antimatter_mint @ MarketError::InvalidMint)]
    pub antimatter_mint: Account<'info, Mint>,
    #[account(init, payer = admin, seeds = [b"market_escrow"], bump, token::mint = antimatter_mint, token::authority = market_escrow_authority)]
    pub market_escrow: Account<'info, TokenAccount>,
    #[account(seeds = [b"market_authority"], bump)]
    pub market_escrow_authority: UncheckedAccount<'info>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct UpdateMarketConfig<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    #[account(mut, seeds = [b"market_config"], bump = market_config.bump, has_one = admin @ MarketError::Unauthorized)]
    pub market_config: Account<'info, MarketConfig>,
}

pub fn initialize_market(
    ctx: Context<InitializeMarket>,
    antimatter_mint: Pubkey,
) -> Result<()> {
    ctx.accounts.market_config.set_inner(MarketConfig {
        admin: ctx.accounts.admin.key(),
        antimatter_mint,
        total_volume: 0,
        total_offers: 0,
        bump: ctx.bumps.market_config,
    });
    Ok(())
}

pub fn initialize_escrow(_ctx: Context<InitializeEscrow>) -> Result<()> {
    msg!("Market escrow initialized");
    Ok(())
}

pub fn update_market_config(
    ctx: Context<UpdateMarketConfig>,
    antimatter_mint: Pubkey,
) -> Result<()> {
    ctx.accounts.market_config.antimatter_mint = antimatter_mint;
    Ok(())
}
