use anchor_lang::prelude::*;

use crate::constants::{MAX_MISSION_COLONY_NAME_LEN, MAX_MISSIONS, MAX_PLANET_NAME_LEN};
use crate::error::GameStateError;

    #[account]
    #[derive(InitSpace)]
    pub struct PlayerProfile {
        pub authority: Pubkey,
        pub planet_count: u32,
        pub bump: u8,
    }

    #[account]
    #[derive(InitSpace)]
    pub struct AuthorizedVault {
        pub authority: Pubkey,
        pub vault: Pubkey,
        pub expires_at: i64,
        pub revoked: bool,
        pub bump: u8,
    }

    #[account]
    #[derive(InitSpace)]
    pub struct VaultBackup {
        pub authority: Pubkey,
        pub vault: Pubkey,
        pub version: u8,
        #[max_len(512)]
        pub ciphertext: Vec<u8>,
        pub iv: [u8; 12],
        pub salt: [u8; 16],
        pub kdf_salt: [u8; 16],
        pub updated_at: i64,
        pub bump: u8,
    }

    #[account]
    #[derive(InitSpace)]
    pub struct GameConfig {
        pub admin: Pubkey,
        pub antimatter_mint: Pubkey,
        pub bump: u8,
    }

    #[account]
    #[derive(InitSpace)]
    pub struct PlanetState {
        pub authority: Pubkey,
        pub player: Pubkey,
        pub planet_index: u32,
        pub galaxy: u16,
        pub system: u16,
        pub position: u8,
        pub name: [u8; MAX_PLANET_NAME_LEN],
        pub diameter: u32,
        pub temperature: i16,
        pub max_fields: u16,
        pub used_fields: u16,
        pub metal_mine: u8,
        pub crystal_mine: u8,
        pub deuterium_synthesizer: u8,
        pub solar_plant: u8,
        pub fusion_reactor: u8,
        pub robotics_factory: u8,
        pub nanite_factory: u8,
        pub shipyard: u8,
        pub metal_storage: u8,
        pub crystal_storage: u8,
        pub deuterium_tank: u8,
        pub research_lab: u8,
        pub missile_silo: u8,
        pub energy_tech: u8,
        pub combustion_drive: u8,
        pub impulse_drive: u8,
        pub hyperspace_drive: u8,
        pub computer_tech: u8,
        pub astrophysics: u8,
        pub igr_network: u8,
        pub research_queue_item: u8,
        pub research_queue_target: u8,
        pub research_finish_ts: i64,
        pub build_queue_item: u8,
        pub build_queue_target: u8,
        pub build_finish_ts: i64,
        pub metal: u64,
        pub crystal: u64,
        pub deuterium: u64,
        pub metal_hour: u64,
        pub crystal_hour: u64,
        pub deuterium_hour: u64,
        pub energy_production: u64,
        pub energy_consumption: u64,
        pub metal_cap: u64,
        pub crystal_cap: u64,
        pub deuterium_cap: u64,
        pub last_update_ts: i64,
        pub small_cargo: u32,
        pub large_cargo: u32,
        pub light_fighter: u32,
        pub heavy_fighter: u32,
        pub cruiser: u32,
        pub battleship: u32,
        pub battlecruiser: u32,
        pub bomber: u32,
        pub destroyer: u32,
        pub deathstar: u32,
        pub recycler: u32,
        pub espionage_probe: u32,
        pub colony_ship: u32,
        pub solar_satellite: u32,
        pub active_missions: u8,
        pub missions: [MissionState; MAX_MISSIONS],
        pub bump: u8,
        pub ship_build_item: u8,
        pub ship_build_qty: u32,
        pub ship_build_finish_ts: i64,
    }

    #[account]
    #[derive(InitSpace)]
    pub struct PlanetCoordinates {
        pub galaxy: u16,
        pub system: u16,
        pub position: u8,
        pub planet: Pubkey,
        pub authority: Pubkey,
        pub bump: u8,
    }

    impl PlanetState {
        pub fn building_level(&self, idx: u8) -> u8 {
            match idx {
                0 => self.metal_mine, 1 => self.crystal_mine, 2 => self.deuterium_synthesizer,
                3 => self.solar_plant, 4 => self.fusion_reactor, 5 => self.robotics_factory,
                6 => self.nanite_factory, 7 => self.shipyard, 8 => self.metal_storage,
                9 => self.crystal_storage, 10 => self.deuterium_tank, 11 => self.research_lab,
                12 => self.missile_silo, _ => 0,
            }
        }

        pub fn set_building_level(&mut self, idx: u8, level: u8) {
            match idx {
                0 => self.metal_mine = level, 1 => self.crystal_mine = level,
                2 => self.deuterium_synthesizer = level, 3 => self.solar_plant = level,
                4 => self.fusion_reactor = level, 5 => self.robotics_factory = level,
                6 => self.nanite_factory = level, 7 => self.shipyard = level,
                8 => self.metal_storage = level, 9 => self.crystal_storage = level,
                10 => self.deuterium_tank = level, 11 => self.research_lab = level,
                12 => self.missile_silo = level, _ => {}
            }
        }

        pub fn research_level(&self, idx: u8) -> u8 {
            match idx {
                0 => self.energy_tech, 1 => self.combustion_drive, 2 => self.impulse_drive,
                3 => self.hyperspace_drive, 4 => self.computer_tech, 5 => self.astrophysics,
                6 => self.igr_network, _ => 0,
            }
        }

        pub fn set_research_level(&mut self, idx: u8, level: u8) {
            match idx {
                0 => self.energy_tech = level, 1 => self.combustion_drive = level,
                2 => self.impulse_drive = level, 3 => self.hyperspace_drive = level,
                4 => self.computer_tech = level, 5 => self.astrophysics = level,
                6 => self.igr_network = level, _ => {}
            }
        }

        pub fn max_usable_mission_slots(&self) -> usize {
            (1 + (self.computer_tech as usize / 5)).min(MAX_MISSIONS)
        }

        pub fn free_mission_slot(&self) -> Option<usize> {
            let usable_slots = self.max_usable_mission_slots();
            (0..usable_slots).find(|&i| self.missions[i].mission_type == 0)
        }

        pub fn mission(&self, slot: usize) -> MissionState { self.missions[slot] }
        pub fn set_mission(&mut self, slot: usize, m: MissionState) { self.missions[slot] = m; }
        pub fn set_mission_applied(&mut self, slot: usize, applied: bool) { self.missions[slot].applied = applied; }
        pub fn clear_mission(&mut self, slot: usize) { self.missions[slot] = MissionState::default(); }

        pub fn return_mission_assets(&mut self, slot: usize) {
            let m = self.missions[slot];
            self.light_fighter = self.light_fighter.saturating_add(m.light_fighter);
            self.heavy_fighter = self.heavy_fighter.saturating_add(m.heavy_fighter);
            self.cruiser = self.cruiser.saturating_add(m.cruiser);
            self.battleship = self.battleship.saturating_add(m.battleship);
            self.battlecruiser = self.battlecruiser.saturating_add(m.battlecruiser);
            self.bomber = self.bomber.saturating_add(m.bomber);
            self.destroyer = self.destroyer.saturating_add(m.destroyer);
            self.deathstar = self.deathstar.saturating_add(m.deathstar);
            self.small_cargo = self.small_cargo.saturating_add(m.small_cargo);
            self.large_cargo = self.large_cargo.saturating_add(m.large_cargo);
            self.recycler = self.recycler.saturating_add(m.recycler);
            self.espionage_probe = self.espionage_probe.saturating_add(m.espionage_probe);
            self.colony_ship = self.colony_ship.saturating_add(m.colony_ship);
            self.metal = self.metal.saturating_add(m.cargo_metal);
            self.crystal = self.crystal.saturating_add(m.cargo_crystal);
            self.deuterium = self.deuterium.saturating_add(m.cargo_deuterium);
        }

        pub fn return_mission_ships_only(&mut self, slot: usize) {
            let m = self.missions[slot];
            self.light_fighter = self.light_fighter.saturating_add(m.light_fighter);
            self.heavy_fighter = self.heavy_fighter.saturating_add(m.heavy_fighter);
            self.cruiser = self.cruiser.saturating_add(m.cruiser);
            self.battleship = self.battleship.saturating_add(m.battleship);
            self.battlecruiser = self.battlecruiser.saturating_add(m.battlecruiser);
            self.bomber = self.bomber.saturating_add(m.bomber);
            self.destroyer = self.destroyer.saturating_add(m.destroyer);
            self.deathstar = self.deathstar.saturating_add(m.deathstar);
            self.small_cargo = self.small_cargo.saturating_add(m.small_cargo);
            self.large_cargo = self.large_cargo.saturating_add(m.large_cargo);
            self.recycler = self.recycler.saturating_add(m.recycler);
            self.espionage_probe = self.espionage_probe.saturating_add(m.espionage_probe);
            self.colony_ship = self.colony_ship.saturating_add(m.colony_ship);
        }

        pub fn add_ship(&mut self, ship_type: u8, quantity: u32) -> Result<()> {
            match ship_type {
                0 => self.small_cargo = self.small_cargo.saturating_add(quantity),
                1 => self.large_cargo = self.large_cargo.saturating_add(quantity),
                2 => self.light_fighter = self.light_fighter.saturating_add(quantity),
                3 => self.heavy_fighter = self.heavy_fighter.saturating_add(quantity),
                4 => self.cruiser = self.cruiser.saturating_add(quantity),
                5 => self.battleship = self.battleship.saturating_add(quantity),
                6 => self.battlecruiser = self.battlecruiser.saturating_add(quantity),
                7 => self.bomber = self.bomber.saturating_add(quantity),
                8 => self.destroyer = self.destroyer.saturating_add(quantity),
                9 => self.deathstar = self.deathstar.saturating_add(quantity),
                10 => self.recycler = self.recycler.saturating_add(quantity),
                11 => self.espionage_probe = self.espionage_probe.saturating_add(quantity),
                12 => self.colony_ship = self.colony_ship.saturating_add(quantity),
                13 => self.solar_satellite = self.solar_satellite.saturating_add(quantity),
                _ => return Err(GameStateError::InvalidShipType.into()),
            }
            Ok(())
        }
    }

    #[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq)]
pub enum ResourceType {
    Metal = 0,
    Crystal = 1,
    Deuterium = 2,
}

    #[derive(AnchorSerialize, AnchorDeserialize, InitSpace, Copy, Clone, Default)]
    pub struct MissionState {
        pub mission_type: u8,
        pub target_galaxy: u16,
        pub target_system: u16,
        pub target_position: u8,
        pub colony_name: [u8; MAX_MISSION_COLONY_NAME_LEN],
        pub depart_ts: i64,
        pub arrive_ts: i64,
        pub return_ts: i64,
        pub small_cargo: u32,
        pub large_cargo: u32,
        pub light_fighter: u32,
        pub heavy_fighter: u32,
        pub cruiser: u32,
        pub battleship: u32,
        pub battlecruiser: u32,
        pub bomber: u32,
        pub destroyer: u32,
        pub deathstar: u32,
        pub recycler: u32,
        pub espionage_probe: u32,
        pub colony_ship: u32,
        pub cargo_metal: u64,
        pub cargo_crystal: u64,
        pub cargo_deuterium: u64,
        pub applied: bool,
        pub speed_factor: u8,
    }

    #[derive(AnchorSerialize, AnchorDeserialize)]
    pub struct InitializePlanetParams {
        pub name: String,
        pub galaxy: u16,
        pub system: u16,
        pub position: u8,
        pub diameter: u32,
        pub temperature: i16,
        pub max_fields: u16,
        pub used_fields: u16,
        pub metal_mine: u8,
        pub crystal_mine: u8,
        pub deuterium_synthesizer: u8,
        pub solar_plant: u8,
        pub fusion_reactor: u8,
        pub robotics_factory: u8,
        pub nanite_factory: u8,
        pub shipyard: u8,
        pub metal_storage: u8,
        pub crystal_storage: u8,
        pub deuterium_tank: u8,
        pub research_lab: u8,
        pub missile_silo: u8,
        pub energy_tech: u8,
        pub combustion_drive: u8,
        pub impulse_drive: u8,
        pub hyperspace_drive: u8,
        pub computer_tech: u8,
        pub astrophysics: u8,
        pub igr_network: u8,
        pub research_queue_item: u8,
        pub research_queue_target: u8,
        pub research_finish_ts: i64,
        pub build_queue_item: u8,
        pub build_queue_target: u8,
        pub build_finish_ts: i64,
        pub metal: u64,
        pub crystal: u64,
        pub deuterium: u64,
        pub metal_hour: u64,
        pub crystal_hour: u64,
        pub deuterium_hour: u64,
        pub energy_production: u64,
        pub energy_consumption: u64,
        pub metal_cap: u64,
        pub crystal_cap: u64,
        pub deuterium_cap: u64,
        pub last_update_ts: i64,
        pub small_cargo: u32,
        pub large_cargo: u32,
        pub light_fighter: u32,
        pub heavy_fighter: u32,
        pub cruiser: u32,
        pub battleship: u32,
        pub battlecruiser: u32,
        pub bomber: u32,
        pub destroyer: u32,
        pub deathstar: u32,
        pub recycler: u32,
        pub espionage_probe: u32,
        pub colony_ship: u32,
        pub solar_satellite: u32,
        pub ship_build_item: u8,
        pub ship_build_qty: u32,
        pub ship_build_finish_ts: i64,
    }

    /// Params for `initialize_homeworld` — galaxy/system/position are optional hints.
    /// If galaxy == 0, program derives coordinates from authority pubkey bytes.
    /// Client should pass the resolved coords (non-zero) so the `planet_coords` PDA
    /// can be derived correctly for the `InitializePlanetVault` context.
    #[derive(AnchorSerialize, AnchorDeserialize)]
    pub struct InitializeHomeworldParams {
        pub now: i64,
        pub name: String,
        pub galaxy: u16,
        pub system: u16,
        pub position: u8,
    }

    #[derive(AnchorSerialize, AnchorDeserialize)]
    pub struct InitializeColonyParams {
        pub now: i64,
        pub name: String,
        pub galaxy: u16,
        pub system: u16,
        pub position: u8,
        pub cargo_metal: u64,
        pub cargo_crystal: u64,
        pub cargo_deuterium: u64,
        pub small_cargo: u32,
        pub large_cargo: u32,
        pub light_fighter: u32,
        pub heavy_fighter: u32,
        pub cruiser: u32,
        pub battleship: u32,
        pub battlecruiser: u32,
        pub bomber: u32,
        pub destroyer: u32,
        pub deathstar: u32,
        pub recycler: u32,
        pub espionage_probe: u32,
        pub solar_satellite: u32,
    }

    #[derive(AnchorSerialize, AnchorDeserialize)]
    pub struct LaunchFleetParams {
        pub mission_type: u8,
        pub light_fighter: u32,
        pub heavy_fighter: u32,
        pub cruiser: u32,
        pub battleship: u32,
        pub battlecruiser: u32,
        pub bomber: u32,
        pub destroyer: u32,
        pub deathstar: u32,
        pub small_cargo: u32,
        pub large_cargo: u32,
        pub recycler: u32,
        pub espionage_probe: u32,
        pub colony_ship: u32,
        pub cargo_metal: u64,
        pub cargo_crystal: u64,
        pub cargo_deuterium: u64,
        pub speed_factor: u8,
        pub now: i64,
        pub target_galaxy: u16,
        pub target_system: u16,
        pub target_position: u8,
        pub colony_name: String,
    }
