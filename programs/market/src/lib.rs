use anchor_lang::prelude::*;

pub mod constants;
pub mod error;
pub mod instructions;
pub mod state;
pub mod types;
pub mod utils;

use instructions::*;
pub use constants::*;
pub use error::*;
pub use state::*;
pub use types::*;

declare_id!("E6ubJUSv2eqJE93HHma7WAiMrikkUxkBmEkqELvVb8j3");

#[program]
pub mod market {
    use super::*;

    pub fn initialize_market(ctx: Context<InitializeMarket>, antimatter_mint: Pubkey) -> Result<()> {
        instructions::initialize_market(ctx, antimatter_mint)
    }

    pub fn initialize_escrow(ctx: Context<InitializeEscrow>) -> Result<()> {
        instructions::initialize_escrow(ctx)
    }

    pub fn update_market_config(
        ctx: Context<UpdateMarketConfig>,
        antimatter_mint: Pubkey,
    ) -> Result<()> {
        instructions::update_market_config(ctx, antimatter_mint)
    }

    pub fn create_offer(
        ctx: Context<CreateOffer>,
        resource_type: ResourceType,
        resource_amount: u64,
        price_antimatter: u64,
    ) -> Result<()> {
        instructions::create_offer(ctx, resource_type, resource_amount, price_antimatter)
    }

    pub fn cancel_offer(ctx: Context<CancelOffer>) -> Result<()> {
        instructions::cancel_offer(ctx)
    }

    pub fn accept_offer(ctx: Context<AcceptOffer>) -> Result<()> {
        instructions::accept_offer(ctx)
    }
}
