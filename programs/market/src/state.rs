use anchor_lang::prelude::*;

use crate::types::ResourceType;

#[account]
#[derive(InitSpace)]
pub struct MarketConfig {
    pub admin: Pubkey,
    pub antimatter_mint: Pubkey,
    pub total_volume: u128,
    pub total_offers: u64,
    pub bump: u8,
}

#[account]
#[derive(InitSpace)]
pub struct MarketOffer {
    pub seller: Pubkey,
    pub seller_planet: Pubkey,
    pub resource_type: ResourceType,
    pub resource_amount: u64,
    pub price_antimatter: u64,
    pub created_at: i64,
    pub offer_id: u32,
    pub filled: bool,
    pub bump: u8,
}

#[account]
#[derive(InitSpace)]
pub struct SellerCounter {
    pub seller: Pubkey,
    pub next_offer_id: u32,
    pub active_offers: u32,
    pub bump: u8,
}
