use anchor_lang::prelude::*;

use crate::state::{MarketConfig, MarketOffer};

pub const ANTIMATTER_DECIMALS: u8 = 6;
pub const ANTIMATTER_SCALE: u64 = 1_000_000;
pub const MAX_OFFERS_PER_WALLET: u32 = 20;
pub const MIN_RESOURCE_AMOUNT: u64 = 1_000;
pub const MARKET_FEE_BPS: u64 = 25;
pub const OFFER_ACCOUNT_SPACE: usize = 8 + MarketOffer::INIT_SPACE;
pub const MARKET_CONFIG_SPACE: usize = 8 + MarketConfig::INIT_SPACE;

pub const LOCK_RESOURCES_FOR_MARKET_DISCRIMINATOR: [u8; 8] =
    [0x77, 0x52, 0x53, 0xd9, 0x39, 0x6e, 0xc9, 0x8b];
pub const RELEASE_RESOURCES_FROM_MARKET_DISCRIMINATOR: [u8; 8] =
    [0xd7, 0x8f, 0xe2, 0xee, 0x0c, 0x56, 0x12, 0x7c];
pub const TRANSFER_RESOURCES_FROM_MARKET_DISCRIMINATOR: [u8; 8] =
    [0xe2, 0xea, 0x85, 0x31, 0xe4, 0x20, 0x2a, 0x0c];

pub const GAME_STATE_PROGRAM_ID: Pubkey = Pubkey::new_from_array([
    103, 148, 27, 1, 200, 217, 76, 87, 92, 42, 194, 80, 114, 230, 121, 192, 54, 239, 209, 103,
    217, 18, 202, 213, 138, 22, 161, 194, 40, 24, 140, 181,
]);
