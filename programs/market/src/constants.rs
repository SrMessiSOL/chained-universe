use anchor_lang::prelude::*;

use crate::state::{MarketConfig, MarketOffer, PlanetListing};

pub const ANTIMATTER_DECIMALS: u8 = 6;
pub const ANTIMATTER_SCALE: u64 = 1_000_000;
pub const MAX_OFFERS_PER_WALLET: u32 = 20;
pub const MIN_RESOURCE_AMOUNT: u64 = 1_000;
pub const MARKET_FEE_BPS: u64 = 25;
pub const OFFER_ACCOUNT_SPACE: usize = 8 + MarketOffer::INIT_SPACE;
pub const PLANET_LISTING_ACCOUNT_SPACE: usize = 8 + PlanetListing::INIT_SPACE;
pub const MARKET_CONFIG_SPACE: usize = 8 + MarketConfig::INIT_SPACE;

pub const PROTOCOL_AUTHORITY: Pubkey = Pubkey::new_from_array([
    203, 132, 160, 224, 36, 85, 73, 7, 47, 237, 167, 190, 14, 187, 187, 93, 163, 241, 26, 147,
    33, 250, 136, 102, 231, 253, 14, 8, 203, 234, 209, 64,
]);
pub const PROTOCOL_ANTIMATTER_MINT: Pubkey = Pubkey::new_from_array([
    210, 124, 79, 139, 189, 97, 171, 121, 236, 30, 15, 224, 71, 28, 151, 137, 112, 205, 123, 216,
    200, 197, 217, 132, 30, 230, 156, 231, 135, 221, 136, 128,
]);

pub const LOCK_RESOURCES_FOR_MARKET_DISCRIMINATOR: [u8; 8] =
    [0x77, 0x52, 0x53, 0xd9, 0x39, 0x6e, 0xc9, 0x8b];
pub const RELEASE_RESOURCES_FROM_MARKET_DISCRIMINATOR: [u8; 8] =
    [0xd7, 0x8f, 0xe2, 0xee, 0x0c, 0x56, 0x12, 0x7c];
pub const TRANSFER_RESOURCES_FROM_MARKET_DISCRIMINATOR: [u8; 8] =
    [0xe2, 0xea, 0x85, 0x31, 0xe4, 0x20, 0x2a, 0x0c];
pub const TRANSFER_PLANET_FROM_MARKET_DISCRIMINATOR: [u8; 8] =
    [64, 26, 189, 103, 47, 71, 195, 229];

pub const GAME_STATE_PROGRAM_ID: Pubkey = Pubkey::new_from_array([
    212, 112, 137, 119, 7, 42, 110, 211, 40, 214, 58, 138, 217, 212, 16, 233, 143, 250, 118,
    186, 107, 69, 190, 51, 16, 94, 153, 90, 66, 250, 28, 110,
]);
