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
    194, 171, 76, 163, 210, 137, 5, 66, 103, 236, 205, 120, 111, 87, 59, 250,
    139, 237, 101, 230, 54, 199, 209, 132, 25, 2, 106, 137, 247, 197, 199, 242,
]);
