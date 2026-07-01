use anchor_lang::prelude::*;

use crate::state::{AuthorizedVault, GameConfig, PlanetCoordinates, PlanetState, PlayerProfile, VaultBackup};

pub const MAX_PLANET_NAME_LEN: usize = 32;
pub const MAX_MISSION_COLONY_NAME_LEN: usize = 32;
pub const MAX_MISSIONS: usize = 4;
pub const MISSION_TRANSPORT: u8 = 2;
pub const MISSION_COLONIZE: u8 = 5;
pub const ANTIMATTER_DECIMALS: u8 = 6;
pub const ANTIMATTER_SCALE: u64 = 1_000_000;
pub const PLANET_COORDS_SPACE: usize = 8 + PlanetCoordinates::INIT_SPACE;
pub const PLAYER_PROFILE_SPACE: usize = 8 + PlayerProfile::INIT_SPACE;
pub const PLANET_STATE_SPACE: usize = 8 + PlanetState::INIT_SPACE;
pub const AUTHORIZED_VAULT_SPACE: usize = 8 + AuthorizedVault::INIT_SPACE;
pub const VAULT_BACKUP_SPACE: usize = 8 + VaultBackup::INIT_SPACE;
pub const GAME_CONFIG_SPACE: usize = 8 + GameConfig::INIT_SPACE;
pub const MARKET_PROGRAM_ID: Pubkey = Pubkey::new_from_array([
    190, 82, 37, 232, 28, 50, 248, 91, 61, 49, 15, 43, 213, 115, 237, 81,
    239, 139, 230, 221, 59, 251, 31, 76, 160, 16, 0, 153, 247, 21, 15, 41,
]);
