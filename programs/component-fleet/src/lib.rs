use bolt_lang::*;

declare_id!("5UuCSuNqVXwCd7qPFQXj8Kp7DAqbB5ZuHFLZZ32paPLD");

pub const MAX_MISSIONS: usize = 4;

/// Fleet Component
/// Tracks stationed ships and up to 8 in-flight missions.
#[component]
#[derive(Default)]
pub struct Fleet {
    // ── Stationed ships ───────────────────────────────────────────────────
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

    /// Number of active outbound missions
    pub active_missions: u8,
    /// In-flight mission slots
    pub missions: [Mission; MAX_MISSIONS],
}

/// One in-flight fleet mission (fixed-size, lives inside Fleet component)
#[component_deserialize]
#[derive(Default)]
pub struct Mission {
    /// 0=none,1=attack,2=transport,3=deploy,4=espionage,5=colonize,6=recycle
    pub mission_type: u8,
    /// Destination planet entity PDA
    pub destination: Pubkey,
    pub depart_ts: i64,
    pub arrive_ts: i64,
    /// Return ETA (0 if one-way / not yet set)
    pub return_ts: i64,
    // Ships committed
    pub s_small_cargo: u32,
    pub s_large_cargo: u32,
    pub s_light_fighter: u32,
    pub s_heavy_fighter: u32,
    pub s_cruiser: u32,
    pub s_battleship: u32,
    pub s_battlecruiser: u32,
    pub s_bomber: u32,
    pub s_destroyer: u32,
    pub s_deathstar: u32,
    pub s_recycler: u32,
    pub s_espionage_probe: u32,
    pub s_colony_ship: u32,
    // Cargo
    pub cargo_metal: u64,
    pub cargo_crystal: u64,
    pub cargo_deuterium: u64,
    /// True once the mission effect (battle/loot) has been applied
    pub applied: bool,
}

impl Mission {
    pub fn is_empty(&self) -> bool {
        self.mission_type == 0
    }

    pub fn total_cargo_capacity(&self) -> u64 {
        self.s_small_cargo as u64 * 5_000
            + self.s_large_cargo as u64 * 25_000
            + self.s_recycler as u64 * 20_000
            + self.s_cruiser as u64 * 800
            + self.s_battleship as u64 * 1_500
    }
}

impl Fleet {
    /// Find the first empty mission slot, returns None if all full.
    pub fn first_free_slot(&self) -> Option<usize> {
        self.missions.iter().position(|m| m.is_empty())
    }

    /// Total attack power (for quick combat estimate).
    pub fn attack_power(&self) -> u64 {
        self.small_cargo as u64 * 5
            + self.large_cargo as u64 * 5
            + self.light_fighter as u64 * 50
            + self.heavy_fighter as u64 * 150
            + self.cruiser as u64 * 400
            + self.battleship as u64 * 1_000
            + self.battlecruiser as u64 * 700
            + self.bomber as u64 * 1_000
            + self.destroyer as u64 * 2_000
            + self.deathstar as u64 * 200_000
    }

    pub fn shield_power(&self) -> u64 {
        self.light_fighter as u64 * 10
            + self.heavy_fighter as u64 * 25
            + self.cruiser as u64 * 50
            + self.battleship as u64 * 200
            + self.battlecruiser as u64 * 400
            + self.destroyer as u64 * 500
            + self.deathstar as u64 * 50_000
    }

    pub fn hull_points(&self) -> u64 {
        self.light_fighter as u64 * 800
            + self.heavy_fighter as u64 * 3_000
            + self.cruiser as u64 * 13_500
            + self.battleship as u64 * 30_000
            + self.battlecruiser as u64 * 35_000
            + self.bomber as u64 * 30_000
            + self.destroyer as u64 * 55_000
            + self.deathstar as u64 * 2_000_000
    }
}
