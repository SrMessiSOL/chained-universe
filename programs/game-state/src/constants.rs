use anchor_lang::prelude::*;

use crate::state::{
    AllianceJoinRequest, AllianceMembership, AllianceState, AuthorizedVault, GameConfig,
    PlanetCoordinates, PlanetState, PlayerProfile, PublicPlanetCoordinates, PublicPlanetState,
    QuestState, StoreConfig, StorePurchaseState, VaultBackup,
};

pub const MAX_PLANET_NAME_LEN: usize = 32;
pub const MAX_MISSION_COLONY_NAME_LEN: usize = 32;
pub const MAX_ALLIANCE_NAME_LEN: usize = 32;
pub const MAX_MISSIONS: usize = 4;
pub const MAX_COMBAT_ROUNDS: u8 = 6;
pub const MAX_RESOURCE_SETTLEMENT_SECONDS: i64 = 86_400;
pub const NEW_PLAYER_PROTECTION_SECONDS: i64 = 7 * 24 * 60 * 60;
pub const ATTACK_UNLOCK_SECONDS: i64 = 300;
pub const MARKET_UNLOCK_SECONDS: i64 = 300;
pub const ATTACK_LAUNCH_COOLDOWN_SECONDS: i64 = 60;
pub const TARGET_ATTACK_COOLDOWN_SECONDS: i64 = 30 * 60;
pub const MAX_PURCHASED_SHIELD_SECONDS: i64 = 7 * 24 * 60 * 60;
pub const DAILY_SHIELD_SECONDS: i64 = 6 * 60 * 60;
pub const WEEKLY_SHIELD_SECONDS: i64 = 24 * 60 * 60;
pub const MONTHLY_SHIELD_SECONDS: i64 = 3 * 24 * 60 * 60;
pub const MIN_ATTACK_COMBAT_POINTS: u64 = 1_000;
pub const MISSION_ATTACK: u8 = 1;
pub const MISSION_TRANSPORT: u8 = 2;
pub const MISSION_COLONIZE: u8 = 5;
pub const MISSION_ESPIONAGE: u8 = 6;
pub const ANTIMATTER_DECIMALS: u8 = 6;
pub const ANTIMATTER_SCALE: u64 = 1_000_000;
pub const ALLIANCE_CREATE_USDC_COST: u64 = 1_000_000;
pub const ALLIANCE_CREATE_ANTIMATTER_COST: u64 = 10_000 * ANTIMATTER_SCALE;
pub const ALLIANCE_CREATE_ANTIMATTER_BURN: u64 = ALLIANCE_CREATE_ANTIMATTER_COST / 2;
pub const ALLIANCE_CREATE_ANTIMATTER_TREASURY: u64 =
    ALLIANCE_CREATE_ANTIMATTER_COST - ALLIANCE_CREATE_ANTIMATTER_BURN;
pub const PROTOCOL_AUTHORITY: Pubkey = Pubkey::new_from_array([
    18, 219, 72, 180, 222, 89, 132, 119, 116, 48, 110, 92, 37, 47, 145, 94, 46, 236, 45, 117, 75,
    253, 48, 98, 150, 23, 63, 86, 44, 10, 129, 59,
]);
pub const PROTOCOL_ANTIMATTER_MINT: Pubkey = Pubkey::new_from_array([
    210, 124, 79, 139, 189, 97, 171, 121, 236, 30, 15, 224, 71, 28, 151, 137, 112, 205, 123, 216,
    200, 197, 217, 132, 30, 230, 156, 231, 135, 221, 136, 128,
]);
pub const STORE_USDC_MINT: Pubkey = Pubkey::new_from_array([
    59, 68, 44, 179, 145, 33, 87, 241, 58, 147, 61, 1, 52, 40, 45, 3, 43, 95, 254, 205, 1, 162,
    219, 241, 183, 121, 6, 8, 223, 0, 46, 167,
]);
pub const STARTING_METAL: u64 = 500;
pub const STARTING_CRYSTAL: u64 = 500;
pub const STARTING_DEUTERIUM: u64 = 100;
pub const BASE_STORAGE_CAP: u64 = 10_000;
pub const PLANET_COORDS_SPACE: usize = 8 + PlanetCoordinates::INIT_SPACE;
pub const PUBLIC_PLANET_COORDS_SPACE: usize = 8 + PublicPlanetCoordinates::INIT_SPACE;
pub const PLAYER_PROFILE_SPACE: usize = 8 + PlayerProfile::INIT_SPACE;
pub const PLANET_STATE_SPACE: usize = 8 + PlanetState::INIT_SPACE;
pub const PUBLIC_PLANET_STATE_SPACE: usize = 8 + PublicPlanetState::INIT_SPACE;
pub const AUTHORIZED_VAULT_SPACE: usize = 8 + AuthorizedVault::INIT_SPACE;
pub const VAULT_BACKUP_SPACE: usize = 8 + VaultBackup::INIT_SPACE;
pub const GAME_CONFIG_SPACE: usize = 8 + GameConfig::INIT_SPACE;
pub const STORE_CONFIG_SPACE: usize = 8 + StoreConfig::INIT_SPACE;
pub const STORE_PURCHASE_STATE_SPACE: usize = 8 + StorePurchaseState::INIT_SPACE;
pub const QUEST_STATE_SPACE: usize = 8 + QuestState::INIT_SPACE;
pub const ALLIANCE_STATE_SPACE: usize = 8 + AllianceState::INIT_SPACE;
pub const ALLIANCE_MEMBERSHIP_SPACE: usize = 8 + AllianceMembership::INIT_SPACE;
pub const ALLIANCE_JOIN_REQUEST_SPACE: usize = 8 + AllianceJoinRequest::INIT_SPACE;
pub const BASE_ALLIANCE_MAX_MEMBERS: u16 = 5;
pub const ALLIANCE_MEMBERS_PER_LEVEL: u16 = 3;
pub const ALLIANCE_XP_UNIT: u64 = 1_000;
pub const MARKET_PROGRAM_ID: Pubkey = Pubkey::new_from_array([
    199, 191, 129, 173, 48, 254, 247, 243, 56, 143, 194, 106, 97, 95, 247, 100, 186, 110, 44, 199,
    200, 196, 181, 11, 54, 135, 246, 43, 169, 50, 45, 71,
]);
