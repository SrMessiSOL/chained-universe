use anchor_lang::prelude::*;

use crate::constants::{
    MAX_ALLIANCE_IMAGE_URL_LEN, MAX_ALLIANCE_NAME_LEN, MAX_ALLIANCE_TAG_LEN, MAX_MISSIONS,
    MAX_MISSION_COLONY_NAME_LEN, MAX_PLANET_NAME_LEN,
};
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
pub struct StoreConfig {
    pub admin: Pubkey,
    pub usdc_mint: Pubkey,
    pub treasury_usdc_account: Pubkey,
    pub enabled: bool,
    pub bump: u8,
}

#[account]
#[derive(InitSpace)]
pub struct StorePurchaseState {
    pub authority: Pubkey,
    pub daily_epoch: i64,
    pub weekly_epoch: i64,
    pub monthly_epoch: i64,
    pub daily_purchased_mask: u64,
    pub weekly_purchased_mask: u64,
    pub monthly_purchased_mask: u64,
    pub last_updated_ts: i64,
    pub bump: u8,
}

#[account]
#[derive(InitSpace)]
pub struct AllianceState {
    pub founder: Pubkey,
    pub name: [u8; MAX_ALLIANCE_NAME_LEN],
    pub level: u16,
    pub xp: u64,
    pub member_count: u16,
    pub max_members: u16,
    pub total_missions_completed: u64,
    pub created_at: i64,
    pub bump: u8,
}

#[account]
#[derive(InitSpace)]
pub struct AllianceMetadata {
    pub alliance: Pubkey,
    pub tag: [u8; MAX_ALLIANCE_TAG_LEN],
    pub image_url: [u8; MAX_ALLIANCE_IMAGE_URL_LEN],
    pub bump: u8,
}

#[account]
#[derive(InitSpace)]
pub struct AllianceTreasuryState {
    pub alliance: Pubkey,
    pub metal: u64,
    pub crystal: u64,
    pub deuterium: u64,
    pub antimatter: u64,
    pub logistics_hub: u8,
    pub research_grid: u8,
    pub defense_coordination: u8,
    pub trade_network: u8,
    pub total_metal_deposited: u64,
    pub total_crystal_deposited: u64,
    pub total_deuterium_deposited: u64,
    pub total_antimatter_deposited: u64,
    pub bump: u8,
}

#[account]
#[derive(InitSpace)]
pub struct AllianceMembership {
    pub authority: Pubkey,
    pub alliance: Pubkey,
    pub role: u8,
    pub joined_at: i64,
    pub daily_epoch: i64,
    pub weekly_epoch: i64,
    pub monthly_epoch: i64,
    pub daily_claimed_mask: u64,
    pub weekly_claimed_mask: u64,
    pub monthly_claimed_mask: u64,
    pub bump: u8,
}

#[account]
#[derive(InitSpace)]
pub struct AllianceJoinRequest {
    pub applicant: Pubkey,
    pub alliance: Pubkey,
    pub created_at: i64,
    pub bump: u8,
}

#[account]
#[derive(InitSpace)]
pub struct QuestState {
    pub authority: Pubkey,
    pub tutorial_claimed_mask: u64,
    pub daily_epoch: i64,
    pub weekly_epoch: i64,
    pub monthly_epoch: i64,
    pub daily_claimed_mask: u64,
    pub weekly_claimed_mask: u64,
    pub monthly_claimed_mask: u64,
    pub daily_checkin_day: i64,
    pub daily_checkin_streak: u16,
    pub total_checkins: u32,
    pub last_updated_ts: i64,
    pub bump: u8,
}

#[account]
#[derive(InitSpace)]
pub struct QuestProgressState {
    pub authority: Pubkey,
    pub daily_epoch: i64,
    pub weekly_epoch: i64,
    pub monthly_epoch: i64,
    pub daily_store_packs_bought: u32,
    pub weekly_store_packs_bought: u32,
    pub monthly_store_packs_bought: u32,
    pub daily_antimatter_spent: u64,
    pub weekly_antimatter_spent: u64,
    pub monthly_antimatter_spent: u64,
    pub daily_planets_colonized: u32,
    pub weekly_planets_colonized: u32,
    pub monthly_planets_colonized: u32,
    pub daily_attacks_resolved: u32,
    pub weekly_attacks_resolved: u32,
    pub monthly_attacks_resolved: u32,
    pub daily_transports_resolved: u32,
    pub weekly_transports_resolved: u32,
    pub monthly_transports_resolved: u32,
    pub daily_spy_missions_resolved: u32,
    pub weekly_spy_missions_resolved: u32,
    pub monthly_spy_missions_resolved: u32,
    pub last_updated_ts: i64,
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
    pub weapons_technology: u8,
    pub shielding_technology: u8,
    pub armor_technology: u8,
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
    pub created_at: i64,
    pub protection_until_ts: i64,
    pub market_unlocked_at: i64,
    pub attack_unlocked_at: i64,
    pub last_attack_launch_ts: i64,
    pub last_attacked_ts: i64,
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
    pub rocket_launcher: u32,
    pub light_laser: u32,
    pub heavy_laser: u32,
    pub gauss_cannon: u32,
    pub ion_cannon: u32,
    pub plasma_turret: u32,
    pub small_shield_dome: u32,
    pub large_shield_dome: u32,
    pub anti_ballistic_missile: u32,
    pub interplanetary_missile: u32,
    pub active_missions: u8,
    pub missions: [MissionState; MAX_MISSIONS],
    pub bump: u8,
    pub ship_build_item: u8,
    pub ship_build_qty: u32,
    pub ship_build_finish_ts: i64,
    pub defense_build_item: u8,
    pub defense_build_qty: u32,
    pub defense_build_finish_ts: i64,
}

#[account]
#[derive(InitSpace)]
pub struct PublicPlanetState {
    pub authority: Pubkey,
    pub player: Pubkey,
    pub planet_index: u32,
    pub galaxy: u16,
    pub system: u16,
    pub position: u8,
    pub version: u8,
    pub name: [u8; MAX_PLANET_NAME_LEN],
    pub created_at: i64,
    pub bump: u8,
}

#[account]
#[derive(InitSpace)]
pub struct PublicPlanetCoordinates {
    pub galaxy: u16,
    pub system: u16,
    pub position: u8,
    pub public_planet: Pubkey,
    pub authority: Pubkey,
    pub bump: u8,
}

#[account]
#[derive(InitSpace)]
pub struct PlanetCoordinates {
    pub galaxy: u16,
    pub system: u16,
    pub position: u8,
    pub planet: Pubkey,
    pub authority: Pubkey,
    pub debris_metal: u64,
    pub debris_crystal: u64,
    pub bump: u8,
}

impl PlanetState {
    pub fn building_level(&self, idx: u8) -> u8 {
        match idx {
            0 => self.metal_mine,
            1 => self.crystal_mine,
            2 => self.deuterium_synthesizer,
            3 => self.solar_plant,
            4 => self.fusion_reactor,
            5 => self.robotics_factory,
            6 => self.nanite_factory,
            7 => self.shipyard,
            8 => self.metal_storage,
            9 => self.crystal_storage,
            10 => self.deuterium_tank,
            11 => self.research_lab,
            12 => self.missile_silo,
            _ => 0,
        }
    }

    pub fn set_building_level(&mut self, idx: u8, level: u8) {
        match idx {
            0 => self.metal_mine = level,
            1 => self.crystal_mine = level,
            2 => self.deuterium_synthesizer = level,
            3 => self.solar_plant = level,
            4 => self.fusion_reactor = level,
            5 => self.robotics_factory = level,
            6 => self.nanite_factory = level,
            7 => self.shipyard = level,
            8 => self.metal_storage = level,
            9 => self.crystal_storage = level,
            10 => self.deuterium_tank = level,
            11 => self.research_lab = level,
            12 => self.missile_silo = level,
            _ => {}
        }
    }

    pub fn research_level(&self, idx: u8) -> u8 {
        match idx {
            0 => self.energy_tech,
            1 => self.combustion_drive,
            2 => self.impulse_drive,
            3 => self.hyperspace_drive,
            4 => self.computer_tech,
            5 => self.astrophysics,
            6 => self.igr_network,
            7 => self.weapons_technology,
            8 => self.shielding_technology,
            9 => self.armor_technology,
            _ => 0,
        }
    }

    pub fn set_research_level(&mut self, idx: u8, level: u8) {
        match idx {
            0 => self.energy_tech = level,
            1 => self.combustion_drive = level,
            2 => self.impulse_drive = level,
            3 => self.hyperspace_drive = level,
            4 => self.computer_tech = level,
            5 => self.astrophysics = level,
            6 => self.igr_network = level,
            7 => self.weapons_technology = level,
            8 => self.shielding_technology = level,
            9 => self.armor_technology = level,
            _ => {}
        }
    }

    pub fn max_usable_mission_slots(&self) -> usize {
        (1 + (self.computer_tech as usize / 5)).min(MAX_MISSIONS)
    }

    pub fn free_mission_slot(&self) -> Option<usize> {
        let usable_slots = self.max_usable_mission_slots();
        (0..usable_slots).find(|&i| self.missions[i].mission_type == 0)
    }

    pub fn mission(&self, slot: usize) -> MissionState {
        self.missions[slot]
    }

    pub fn set_mission(&mut self, slot: usize, mission: MissionState) {
        self.missions[slot] = mission;
    }

    pub fn set_mission_applied(&mut self, slot: usize, applied: bool) {
        self.missions[slot].applied = applied;
    }

    pub fn clear_mission(&mut self, slot: usize) {
        self.missions[slot] = MissionState::default();
    }

    pub fn ensure_resource_room(&self, metal: u64, crystal: u64, deuterium: u64) -> Result<()> {
        require!(
            matches!(self.metal.checked_add(metal), Some(value) if value <= self.metal_cap),
            GameStateError::ResourceCapExceeded
        );
        require!(
            matches!(self.crystal.checked_add(crystal), Some(value) if value <= self.crystal_cap),
            GameStateError::ResourceCapExceeded
        );
        require!(
            matches!(
                self.deuterium.checked_add(deuterium),
                Some(value) if value <= self.deuterium_cap
            ),
            GameStateError::ResourceCapExceeded
        );
        Ok(())
    }

    pub fn credit_resources(&mut self, metal: u64, crystal: u64, deuterium: u64) -> Result<()> {
        self.ensure_resource_room(metal, crystal, deuterium)?;
        self.metal = self.metal.saturating_add(metal);
        self.crystal = self.crystal.saturating_add(crystal);
        self.deuterium = self.deuterium.saturating_add(deuterium);
        Ok(())
    }

    pub fn return_mission_assets(&mut self, slot: usize) -> Result<()> {
        let mission = self.missions[slot];
        self.ensure_resource_room(
            mission.cargo_metal,
            mission.cargo_crystal,
            mission.cargo_deuterium,
        )?;
        self.light_fighter = self.light_fighter.saturating_add(mission.light_fighter);
        self.heavy_fighter = self.heavy_fighter.saturating_add(mission.heavy_fighter);
        self.cruiser = self.cruiser.saturating_add(mission.cruiser);
        self.battleship = self.battleship.saturating_add(mission.battleship);
        self.battlecruiser = self.battlecruiser.saturating_add(mission.battlecruiser);
        self.bomber = self.bomber.saturating_add(mission.bomber);
        self.destroyer = self.destroyer.saturating_add(mission.destroyer);
        self.deathstar = self.deathstar.saturating_add(mission.deathstar);
        self.small_cargo = self.small_cargo.saturating_add(mission.small_cargo);
        self.large_cargo = self.large_cargo.saturating_add(mission.large_cargo);
        self.recycler = self.recycler.saturating_add(mission.recycler);
        self.espionage_probe = self.espionage_probe.saturating_add(mission.espionage_probe);
        self.colony_ship = self.colony_ship.saturating_add(mission.colony_ship);
        self.metal = self.metal.saturating_add(mission.cargo_metal);
        self.crystal = self.crystal.saturating_add(mission.cargo_crystal);
        self.deuterium = self.deuterium.saturating_add(mission.cargo_deuterium);
        Ok(())
    }

    pub fn return_mission_ships_only(&mut self, slot: usize) {
        let mission = self.missions[slot];
        self.light_fighter = self.light_fighter.saturating_add(mission.light_fighter);
        self.heavy_fighter = self.heavy_fighter.saturating_add(mission.heavy_fighter);
        self.cruiser = self.cruiser.saturating_add(mission.cruiser);
        self.battleship = self.battleship.saturating_add(mission.battleship);
        self.battlecruiser = self.battlecruiser.saturating_add(mission.battlecruiser);
        self.bomber = self.bomber.saturating_add(mission.bomber);
        self.destroyer = self.destroyer.saturating_add(mission.destroyer);
        self.deathstar = self.deathstar.saturating_add(mission.deathstar);
        self.small_cargo = self.small_cargo.saturating_add(mission.small_cargo);
        self.large_cargo = self.large_cargo.saturating_add(mission.large_cargo);
        self.recycler = self.recycler.saturating_add(mission.recycler);
        self.espionage_probe = self.espionage_probe.saturating_add(mission.espionage_probe);
        self.colony_ship = self.colony_ship.saturating_add(mission.colony_ship);
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

    pub fn add_defense(&mut self, defense_type: u8, quantity: u32) -> Result<()> {
        match defense_type {
            0 => self.rocket_launcher = self.rocket_launcher.saturating_add(quantity),
            1 => self.light_laser = self.light_laser.saturating_add(quantity),
            2 => self.heavy_laser = self.heavy_laser.saturating_add(quantity),
            3 => self.gauss_cannon = self.gauss_cannon.saturating_add(quantity),
            4 => self.ion_cannon = self.ion_cannon.saturating_add(quantity),
            5 => self.plasma_turret = self.plasma_turret.saturating_add(quantity),
            6 => self.small_shield_dome = self.small_shield_dome.saturating_add(quantity),
            7 => self.large_shield_dome = self.large_shield_dome.saturating_add(quantity),
            8 => self.anti_ballistic_missile = self.anti_ballistic_missile.saturating_add(quantity),
            9 => self.interplanetary_missile = self.interplanetary_missile.saturating_add(quantity),
            _ => return Err(GameStateError::InvalidDefenseType.into()),
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
    pub combat_rounds: u8,
    pub attacker_won: bool,
}

#[event]
pub struct BattleResolvedEvent {
    pub source_planet: Pubkey,
    pub destination_planet: Pubkey,
    pub attacker: Pubkey,
    pub defender: Pubkey,
    pub source_galaxy: u16,
    pub source_system: u16,
    pub source_position: u8,
    pub target_galaxy: u16,
    pub target_system: u16,
    pub target_position: u8,
    pub resolved_at: i64,
    pub mission_slot: u8,
    pub combat_rounds: u8,
    pub attacker_won: bool,
    pub attacker_destroyed: bool,
    pub defender_survived: bool,
    pub loot_metal: u64,
    pub loot_crystal: u64,
    pub loot_deuterium: u64,
    pub debris_metal: u64,
    pub debris_crystal: u64,
    pub recycled_metal: u64,
    pub recycled_crystal: u64,
    pub attacker_small_cargo: u32,
    pub attacker_large_cargo: u32,
    pub attacker_light_fighter: u32,
    pub attacker_heavy_fighter: u32,
    pub attacker_cruiser: u32,
    pub attacker_battleship: u32,
    pub attacker_battlecruiser: u32,
    pub attacker_bomber: u32,
    pub attacker_destroyer: u32,
    pub attacker_deathstar: u32,
    pub attacker_recycler: u32,
    pub attacker_espionage_probe: u32,
    pub attacker_colony_ship: u32,
}

#[event]
pub struct EspionageReportEvent {
    pub source_planet: Pubkey,
    pub destination_planet: Pubkey,
    pub attacker: Pubkey,
    pub defender: Pubkey,
    pub source_galaxy: u16,
    pub source_system: u16,
    pub source_position: u8,
    pub target_galaxy: u16,
    pub target_system: u16,
    pub target_position: u8,
    pub resolved_at: i64,
    pub mission_slot: u8,
    pub reveal_level: u8,
    pub probes_sent: u32,
    pub probes_survived: u32,
    pub probes_lost: u32,
    pub sensor_score: u64,
    pub counter_score: u64,
    pub reported_metal: u64,
    pub reported_crystal: u64,
    pub reported_deuterium: u64,
    pub reported_building_score: u64,
    pub reported_fleet_points: u64,
    pub reported_defense_points: u64,
    pub reported_weapons_technology: u8,
    pub reported_shielding_technology: u8,
    pub reported_armor_technology: u8,
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
    pub weapons_technology: u8,
    pub shielding_technology: u8,
    pub armor_technology: u8,
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
    pub created_at: i64,
    pub protection_until_ts: i64,
    pub market_unlocked_at: i64,
    pub attack_unlocked_at: i64,
    pub last_attack_launch_ts: i64,
    pub last_attacked_ts: i64,
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
    pub rocket_launcher: u32,
    pub light_laser: u32,
    pub heavy_laser: u32,
    pub gauss_cannon: u32,
    pub ion_cannon: u32,
    pub plasma_turret: u32,
    pub small_shield_dome: u32,
    pub large_shield_dome: u32,
    pub anti_ballistic_missile: u32,
    pub interplanetary_missile: u32,
    pub ship_build_item: u8,
    pub ship_build_qty: u32,
    pub ship_build_finish_ts: i64,
    pub defense_build_item: u8,
    pub defense_build_qty: u32,
    pub defense_build_finish_ts: i64,
}

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

#[cfg(test)]
mod tests {
    use super::*;

    fn test_planet() -> PlanetState {
        PlanetState {
            authority: Pubkey::default(),
            player: Pubkey::default(),
            planet_index: 0,
            galaxy: 0,
            system: 0,
            position: 0,
            name: [0; MAX_PLANET_NAME_LEN],
            diameter: 0,
            temperature: 0,
            max_fields: 0,
            used_fields: 0,
            metal_mine: 0,
            crystal_mine: 0,
            deuterium_synthesizer: 0,
            solar_plant: 0,
            fusion_reactor: 0,
            robotics_factory: 0,
            nanite_factory: 0,
            shipyard: 0,
            metal_storage: 0,
            crystal_storage: 0,
            deuterium_tank: 0,
            research_lab: 0,
            missile_silo: 0,
            energy_tech: 0,
            combustion_drive: 0,
            impulse_drive: 0,
            hyperspace_drive: 0,
            computer_tech: 0,
            astrophysics: 0,
            igr_network: 0,
            weapons_technology: 0,
            shielding_technology: 0,
            armor_technology: 0,
            research_queue_item: 0,
            research_queue_target: 0,
            research_finish_ts: 0,
            build_queue_item: 0,
            build_queue_target: 0,
            build_finish_ts: 0,
            metal: 0,
            crystal: 0,
            deuterium: 0,
            metal_hour: 0,
            crystal_hour: 0,
            deuterium_hour: 0,
            energy_production: 0,
            energy_consumption: 0,
            metal_cap: 0,
            crystal_cap: 0,
            deuterium_cap: 0,
            last_update_ts: 0,
            created_at: 0,
            protection_until_ts: 0,
            market_unlocked_at: 0,
            attack_unlocked_at: 0,
            last_attack_launch_ts: 0,
            last_attacked_ts: 0,
            small_cargo: 0,
            large_cargo: 0,
            light_fighter: 0,
            heavy_fighter: 0,
            cruiser: 0,
            battleship: 0,
            battlecruiser: 0,
            bomber: 0,
            destroyer: 0,
            deathstar: 0,
            recycler: 0,
            espionage_probe: 0,
            colony_ship: 0,
            solar_satellite: 0,
            rocket_launcher: 0,
            light_laser: 0,
            heavy_laser: 0,
            gauss_cannon: 0,
            ion_cannon: 0,
            plasma_turret: 0,
            small_shield_dome: 0,
            large_shield_dome: 0,
            anti_ballistic_missile: 0,
            interplanetary_missile: 0,
            active_missions: 0,
            missions: [MissionState::default(); MAX_MISSIONS],
            bump: 0,
            ship_build_item: 0,
            ship_build_qty: 0,
            ship_build_finish_ts: 0,
            defense_build_item: 0,
            defense_build_qty: 0,
            defense_build_finish_ts: 0,
        }
    }

    #[test]
    fn credit_resources_allows_exact_cap() {
        let mut planet = test_planet();
        planet.metal_cap = 100;
        planet.crystal_cap = 200;
        planet.deuterium_cap = 300;

        planet.credit_resources(100, 200, 300).unwrap();

        assert_eq!(planet.metal, 100);
        assert_eq!(planet.crystal, 200);
        assert_eq!(planet.deuterium, 300);
    }

    #[test]
    fn credit_resources_rejects_cap_excess_without_mutating() {
        let mut planet = test_planet();
        planet.metal = 90;
        planet.crystal = 40;
        planet.deuterium = 20;
        planet.metal_cap = 100;
        planet.crystal_cap = 100;
        planet.deuterium_cap = 100;

        assert!(planet.credit_resources(11, 0, 0).is_err());
        assert_eq!(planet.metal, 90);
        assert_eq!(planet.crystal, 40);
        assert_eq!(planet.deuterium, 20);
    }

    #[test]
    fn credit_resources_rejects_u64_overflow_without_mutating() {
        let mut planet = test_planet();
        planet.metal = u64::MAX;
        planet.metal_cap = u64::MAX;
        planet.crystal_cap = u64::MAX;
        planet.deuterium_cap = u64::MAX;

        assert!(planet.credit_resources(1, 0, 0).is_err());
        assert_eq!(planet.metal, u64::MAX);
        assert_eq!(planet.crystal, 0);
        assert_eq!(planet.deuterium, 0);
    }

    #[test]
    fn public_planet_state_stays_public_only() {
        let expected_public_space = 32 + 32 + 4 + 2 + 2 + 1 + 1 + MAX_PLANET_NAME_LEN + 8 + 1;
        assert_eq!(PublicPlanetState::INIT_SPACE, expected_public_space);
        assert!(PublicPlanetState::INIT_SPACE < PlanetState::INIT_SPACE / 4);
    }
}
