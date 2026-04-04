use bolt_lang::*;

declare_id!("CsHSUWnCL4rTi9WYcVRXyy2Sq9TgcH4Lr7WcZNViG5NY");

/// Fleet Component — missions are stored as flat fields (m0_*, m1_*, m2_*, m3_*)
/// to avoid the borsh version conflict caused by nested structs with #[component_deserialize].
#[component(delegate)]
pub struct Fleet {
    pub creator:          Pubkey,
    pub small_cargo:      u32,
    pub large_cargo:      u32,
    pub light_fighter:    u32,
    pub heavy_fighter:    u32,
    pub cruiser:          u32,
    pub battleship:       u32,
    pub battlecruiser:    u32,
    pub bomber:           u32,
    pub destroyer:        u32,
    pub deathstar:        u32,
    pub recycler:         u32,
    pub espionage_probe:  u32,
    pub colony_ship:      u32,
    pub solar_satellite:  u32,
    pub active_missions:  u8,

    // ── Mission slot 0 ────────────────────────────────────────────────────────
    pub m0_type:             u8,
    pub m0_target_galaxy:    u16,
    pub m0_target_system:    u16,
    pub m0_target_position:  u8,
    pub m0_colony_name:      [u8; 32],
    pub m0_depart_ts:        i64,
    pub m0_arrive_ts:        i64,
    pub m0_return_ts:        i64,
    pub m0_small_cargo:      u32,
    pub m0_large_cargo:      u32,
    pub m0_light_fighter:    u32,
    pub m0_heavy_fighter:    u32,
    pub m0_cruiser:          u32,
    pub m0_battleship:       u32,
    pub m0_battlecruiser:    u32,
    pub m0_bomber:           u32,
    pub m0_destroyer:        u32,
    pub m0_deathstar:        u32,
    pub m0_recycler:         u32,
    pub m0_espionage_probe:  u32,
    pub m0_colony_ship:      u32,
    pub m0_cargo_metal:      u64,
    pub m0_cargo_crystal:    u64,
    pub m0_cargo_deuterium:  u64,
    pub m0_applied:          bool,

    // ── Mission slot 1 ────────────────────────────────────────────────────────
    pub m1_type:             u8,
    pub m1_target_galaxy:    u16,
    pub m1_target_system:    u16,
    pub m1_target_position:  u8,
    pub m1_colony_name:      [u8; 32],
    pub m1_depart_ts:        i64,
    pub m1_arrive_ts:        i64,
    pub m1_return_ts:        i64,
    pub m1_small_cargo:      u32,
    pub m1_large_cargo:      u32,
    pub m1_light_fighter:    u32,
    pub m1_heavy_fighter:    u32,
    pub m1_cruiser:          u32,
    pub m1_battleship:       u32,
    pub m1_battlecruiser:    u32,
    pub m1_bomber:           u32,
    pub m1_destroyer:        u32,
    pub m1_deathstar:        u32,
    pub m1_recycler:         u32,
    pub m1_espionage_probe:  u32,
    pub m1_colony_ship:      u32,
    pub m1_cargo_metal:      u64,
    pub m1_cargo_crystal:    u64,
    pub m1_cargo_deuterium:  u64,
    pub m1_applied:          bool,

    // ── Mission slot 2 ────────────────────────────────────────────────────────
    pub m2_type:             u8,
    pub m2_target_galaxy:    u16,
    pub m2_target_system:    u16,
    pub m2_target_position:  u8,
    pub m2_colony_name:      [u8; 32],
    pub m2_depart_ts:        i64,
    pub m2_arrive_ts:        i64,
    pub m2_return_ts:        i64,
    pub m2_small_cargo:      u32,
    pub m2_large_cargo:      u32,
    pub m2_light_fighter:    u32,
    pub m2_heavy_fighter:    u32,
    pub m2_cruiser:          u32,
    pub m2_battleship:       u32,
    pub m2_battlecruiser:    u32,
    pub m2_bomber:           u32,
    pub m2_destroyer:        u32,
    pub m2_deathstar:        u32,
    pub m2_recycler:         u32,
    pub m2_espionage_probe:  u32,
    pub m2_colony_ship:      u32,
    pub m2_cargo_metal:      u64,
    pub m2_cargo_crystal:    u64,
    pub m2_cargo_deuterium:  u64,
    pub m2_applied:          bool,

    // ── Mission slot 3 ────────────────────────────────────────────────────────
    pub m3_type:             u8,
    pub m3_target_galaxy:    u16,
    pub m3_target_system:    u16,
    pub m3_target_position:  u8,
    pub m3_colony_name:      [u8; 32],
    pub m3_depart_ts:        i64,
    pub m3_arrive_ts:        i64,
    pub m3_return_ts:        i64,
    pub m3_small_cargo:      u32,
    pub m3_large_cargo:      u32,
    pub m3_light_fighter:    u32,
    pub m3_heavy_fighter:    u32,
    pub m3_cruiser:          u32,
    pub m3_battleship:       u32,
    pub m3_battlecruiser:    u32,
    pub m3_bomber:           u32,
    pub m3_destroyer:        u32,
    pub m3_deathstar:        u32,
    pub m3_recycler:         u32,
    pub m3_espionage_probe:  u32,
    pub m3_colony_ship:      u32,
    pub m3_cargo_metal:      u64,
    pub m3_cargo_crystal:    u64,
    pub m3_cargo_deuterium:  u64,
    pub m3_applied:          bool,
}

impl Default for Fleet {
    fn default() -> Self {
        Self {
            bolt_metadata:       Default::default(),
            creator:             Pubkey::default(),
            small_cargo:         0,
            large_cargo:         0,
            light_fighter:       0,
            heavy_fighter:       0,
            cruiser:             0,
            battleship:          0,
            battlecruiser:       0,
            bomber:              0,
            destroyer:           0,
            deathstar:           0,
            recycler:            0,
            espionage_probe:     0,
            colony_ship:         0,
            solar_satellite:     0,
            active_missions:     0,
            m0_type: 0, m0_target_galaxy: 0, m0_target_system: 0, m0_target_position: 0, m0_colony_name: [0; 32],
            m0_depart_ts: 0, m0_arrive_ts: 0, m0_return_ts: 0,
            m0_small_cargo: 0, m0_large_cargo: 0, m0_light_fighter: 0,
            m0_heavy_fighter: 0, m0_cruiser: 0, m0_battleship: 0,
            m0_battlecruiser: 0, m0_bomber: 0, m0_destroyer: 0,
            m0_deathstar: 0, m0_recycler: 0, m0_espionage_probe: 0,
            m0_colony_ship: 0, m0_cargo_metal: 0, m0_cargo_crystal: 0,
            m0_cargo_deuterium: 0, m0_applied: false,
            m1_type: 0, m1_target_galaxy: 0, m1_target_system: 0, m1_target_position: 0, m1_colony_name: [0; 32],
            m1_depart_ts: 0, m1_arrive_ts: 0, m1_return_ts: 0,
            m1_small_cargo: 0, m1_large_cargo: 0, m1_light_fighter: 0,
            m1_heavy_fighter: 0, m1_cruiser: 0, m1_battleship: 0,
            m1_battlecruiser: 0, m1_bomber: 0, m1_destroyer: 0,
            m1_deathstar: 0, m1_recycler: 0, m1_espionage_probe: 0,
            m1_colony_ship: 0, m1_cargo_metal: 0, m1_cargo_crystal: 0,
            m1_cargo_deuterium: 0, m1_applied: false,
            m2_type: 0, m2_target_galaxy: 0, m2_target_system: 0, m2_target_position: 0, m2_colony_name: [0; 32],
            m2_depart_ts: 0, m2_arrive_ts: 0, m2_return_ts: 0,
            m2_small_cargo: 0, m2_large_cargo: 0, m2_light_fighter: 0,
            m2_heavy_fighter: 0, m2_cruiser: 0, m2_battleship: 0,
            m2_battlecruiser: 0, m2_bomber: 0, m2_destroyer: 0,
            m2_deathstar: 0, m2_recycler: 0, m2_espionage_probe: 0,
            m2_colony_ship: 0, m2_cargo_metal: 0, m2_cargo_crystal: 0,
            m2_cargo_deuterium: 0, m2_applied: false,
            m3_type: 0, m3_target_galaxy: 0, m3_target_system: 0, m3_target_position: 0, m3_colony_name: [0; 32],
            m3_depart_ts: 0, m3_arrive_ts: 0, m3_return_ts: 0,
            m3_small_cargo: 0, m3_large_cargo: 0, m3_light_fighter: 0,
            m3_heavy_fighter: 0, m3_cruiser: 0, m3_battleship: 0,
            m3_battlecruiser: 0, m3_bomber: 0, m3_destroyer: 0,
            m3_deathstar: 0, m3_recycler: 0, m3_espionage_probe: 0,
            m3_colony_ship: 0, m3_cargo_metal: 0, m3_cargo_crystal: 0,
            m3_cargo_deuterium: 0, m3_applied: false,
        }
    }
}

/// Helper macros to access mission slots by index at runtime.
/// Used by all systems instead of fleet.missions[slot].
impl Fleet {
    pub fn m_type(&self, slot: usize) -> u8 {
        match slot { 0=>self.m0_type, 1=>self.m1_type, 2=>self.m2_type, _=>self.m3_type }
    }
    pub fn m_target_galaxy(&self, slot: usize) -> u16 {
        match slot { 0=>self.m0_target_galaxy, 1=>self.m1_target_galaxy, 2=>self.m2_target_galaxy, _=>self.m3_target_galaxy }
    }
    pub fn m_target_system(&self, slot: usize) -> u16 {
        match slot { 0=>self.m0_target_system, 1=>self.m1_target_system, 2=>self.m2_target_system, _=>self.m3_target_system }
    }
    pub fn m_target_position(&self, slot: usize) -> u8 {
        match slot { 0=>self.m0_target_position, 1=>self.m1_target_position, 2=>self.m2_target_position, _=>self.m3_target_position }
    }
    pub fn m_colony_name(&self, slot: usize) -> [u8; 32] {
        match slot { 0=>self.m0_colony_name, 1=>self.m1_colony_name, 2=>self.m2_colony_name, _=>self.m3_colony_name }
    }
    pub fn m_depart_ts(&self, slot: usize) -> i64 {
        match slot { 0=>self.m0_depart_ts, 1=>self.m1_depart_ts, 2=>self.m2_depart_ts, _=>self.m3_depart_ts }
    }
    pub fn m_arrive_ts(&self, slot: usize) -> i64 {
        match slot { 0=>self.m0_arrive_ts, 1=>self.m1_arrive_ts, 2=>self.m2_arrive_ts, _=>self.m3_arrive_ts }
    }
    pub fn m_return_ts(&self, slot: usize) -> i64 {
        match slot { 0=>self.m0_return_ts, 1=>self.m1_return_ts, 2=>self.m2_return_ts, _=>self.m3_return_ts }
    }
    pub fn m_applied(&self, slot: usize) -> bool {
        match slot { 0=>self.m0_applied, 1=>self.m1_applied, 2=>self.m2_applied, _=>self.m3_applied }
    }
    pub fn m_light_fighter(&self, slot: usize) -> u32 {
        match slot { 0=>self.m0_light_fighter, 1=>self.m1_light_fighter, 2=>self.m2_light_fighter, _=>self.m3_light_fighter }
    }
    pub fn m_heavy_fighter(&self, slot: usize) -> u32 {
        match slot { 0=>self.m0_heavy_fighter, 1=>self.m1_heavy_fighter, 2=>self.m2_heavy_fighter, _=>self.m3_heavy_fighter }
    }
    pub fn m_cruiser(&self, slot: usize) -> u32 {
        match slot { 0=>self.m0_cruiser, 1=>self.m1_cruiser, 2=>self.m2_cruiser, _=>self.m3_cruiser }
    }
    pub fn m_battleship(&self, slot: usize) -> u32 {
        match slot { 0=>self.m0_battleship, 1=>self.m1_battleship, 2=>self.m2_battleship, _=>self.m3_battleship }
    }
    pub fn m_battlecruiser(&self, slot: usize) -> u32 {
        match slot { 0=>self.m0_battlecruiser, 1=>self.m1_battlecruiser, 2=>self.m2_battlecruiser, _=>self.m3_battlecruiser }
    }
    pub fn m_bomber(&self, slot: usize) -> u32 {
        match slot { 0=>self.m0_bomber, 1=>self.m1_bomber, 2=>self.m2_bomber, _=>self.m3_bomber }
    }
    pub fn m_destroyer(&self, slot: usize) -> u32 {
        match slot { 0=>self.m0_destroyer, 1=>self.m1_destroyer, 2=>self.m2_destroyer, _=>self.m3_destroyer }
    }
    pub fn m_deathstar(&self, slot: usize) -> u32 {
        match slot { 0=>self.m0_deathstar, 1=>self.m1_deathstar, 2=>self.m2_deathstar, _=>self.m3_deathstar }
    }
    pub fn m_small_cargo(&self, slot: usize) -> u32 {
        match slot { 0=>self.m0_small_cargo, 1=>self.m1_small_cargo, 2=>self.m2_small_cargo, _=>self.m3_small_cargo }
    }
    pub fn m_large_cargo(&self, slot: usize) -> u32 {
        match slot { 0=>self.m0_large_cargo, 1=>self.m1_large_cargo, 2=>self.m2_large_cargo, _=>self.m3_large_cargo }
    }
    pub fn m_recycler(&self, slot: usize) -> u32 {
        match slot { 0=>self.m0_recycler, 1=>self.m1_recycler, 2=>self.m2_recycler, _=>self.m3_recycler }
    }
    pub fn m_espionage_probe(&self, slot: usize) -> u32 {
        match slot { 0=>self.m0_espionage_probe, 1=>self.m1_espionage_probe, 2=>self.m2_espionage_probe, _=>self.m3_espionage_probe }
    }
    pub fn m_colony_ship(&self, slot: usize) -> u32 {
        match slot { 0=>self.m0_colony_ship, 1=>self.m1_colony_ship, 2=>self.m2_colony_ship, _=>self.m3_colony_ship }
    }
    pub fn m_cargo_metal(&self, slot: usize) -> u64 {
        match slot { 0=>self.m0_cargo_metal, 1=>self.m1_cargo_metal, 2=>self.m2_cargo_metal, _=>self.m3_cargo_metal }
    }
    pub fn m_cargo_crystal(&self, slot: usize) -> u64 {
        match slot { 0=>self.m0_cargo_crystal, 1=>self.m1_cargo_crystal, 2=>self.m2_cargo_crystal, _=>self.m3_cargo_crystal }
    }
    pub fn m_cargo_deuterium(&self, slot: usize) -> u64 {
        match slot { 0=>self.m0_cargo_deuterium, 1=>self.m1_cargo_deuterium, 2=>self.m2_cargo_deuterium, _=>self.m3_cargo_deuterium }
    }

    pub fn set_mission_applied(&mut self, slot: usize, applied: bool) {
        match slot {
            0 => self.m0_applied = applied,
            1 => self.m1_applied = applied,
            2 => self.m2_applied = applied,
            _ => self.m3_applied = applied,
        }
    }

    pub fn return_mission_ships(&mut self, slot: usize) {
        self.light_fighter = self.light_fighter.saturating_add(self.m_light_fighter(slot));
        self.heavy_fighter = self.heavy_fighter.saturating_add(self.m_heavy_fighter(slot));
        self.cruiser = self.cruiser.saturating_add(self.m_cruiser(slot));
        self.battleship = self.battleship.saturating_add(self.m_battleship(slot));
        self.battlecruiser = self.battlecruiser.saturating_add(self.m_battlecruiser(slot));
        self.bomber = self.bomber.saturating_add(self.m_bomber(slot));
        self.destroyer = self.destroyer.saturating_add(self.m_destroyer(slot));
        self.deathstar = self.deathstar.saturating_add(self.m_deathstar(slot));
        self.small_cargo = self.small_cargo.saturating_add(self.m_small_cargo(slot));
        self.large_cargo = self.large_cargo.saturating_add(self.m_large_cargo(slot));
        self.recycler = self.recycler.saturating_add(self.m_recycler(slot));
        self.espionage_probe = self.espionage_probe.saturating_add(self.m_espionage_probe(slot));
        self.colony_ship = self.colony_ship.saturating_add(self.m_colony_ship(slot));
    }

    /// Clear a mission slot (set all fields to default)
    pub fn clear_mission(&mut self, slot: usize) {
        match slot {
            0 => { self.m0_type=0; self.m0_target_galaxy=0; self.m0_target_system=0; self.m0_target_position=0; self.m0_colony_name=[0; 32]; self.m0_depart_ts=0; self.m0_arrive_ts=0; self.m0_return_ts=0; self.m0_small_cargo=0; self.m0_large_cargo=0; self.m0_light_fighter=0; self.m0_heavy_fighter=0; self.m0_cruiser=0; self.m0_battleship=0; self.m0_battlecruiser=0; self.m0_bomber=0; self.m0_destroyer=0; self.m0_deathstar=0; self.m0_recycler=0; self.m0_espionage_probe=0; self.m0_colony_ship=0; self.m0_cargo_metal=0; self.m0_cargo_crystal=0; self.m0_cargo_deuterium=0; self.m0_applied=false; }
            1 => { self.m1_type=0; self.m1_target_galaxy=0; self.m1_target_system=0; self.m1_target_position=0; self.m1_colony_name=[0; 32]; self.m1_depart_ts=0; self.m1_arrive_ts=0; self.m1_return_ts=0; self.m1_small_cargo=0; self.m1_large_cargo=0; self.m1_light_fighter=0; self.m1_heavy_fighter=0; self.m1_cruiser=0; self.m1_battleship=0; self.m1_battlecruiser=0; self.m1_bomber=0; self.m1_destroyer=0; self.m1_deathstar=0; self.m1_recycler=0; self.m1_espionage_probe=0; self.m1_colony_ship=0; self.m1_cargo_metal=0; self.m1_cargo_crystal=0; self.m1_cargo_deuterium=0; self.m1_applied=false; }
            2 => { self.m2_type=0; self.m2_target_galaxy=0; self.m2_target_system=0; self.m2_target_position=0; self.m2_colony_name=[0; 32]; self.m2_depart_ts=0; self.m2_arrive_ts=0; self.m2_return_ts=0; self.m2_small_cargo=0; self.m2_large_cargo=0; self.m2_light_fighter=0; self.m2_heavy_fighter=0; self.m2_cruiser=0; self.m2_battleship=0; self.m2_battlecruiser=0; self.m2_bomber=0; self.m2_destroyer=0; self.m2_deathstar=0; self.m2_recycler=0; self.m2_espionage_probe=0; self.m2_colony_ship=0; self.m2_cargo_metal=0; self.m2_cargo_crystal=0; self.m2_cargo_deuterium=0; self.m2_applied=false; }
            _ => { self.m3_type=0; self.m3_target_galaxy=0; self.m3_target_system=0; self.m3_target_position=0; self.m3_colony_name=[0; 32]; self.m3_depart_ts=0; self.m3_arrive_ts=0; self.m3_return_ts=0; self.m3_small_cargo=0; self.m3_large_cargo=0; self.m3_light_fighter=0; self.m3_heavy_fighter=0; self.m3_cruiser=0; self.m3_battleship=0; self.m3_battlecruiser=0; self.m3_bomber=0; self.m3_destroyer=0; self.m3_deathstar=0; self.m3_recycler=0; self.m3_espionage_probe=0; self.m3_colony_ship=0; self.m3_cargo_metal=0; self.m3_cargo_crystal=0; self.m3_cargo_deuterium=0; self.m3_applied=false; }
        }
    }

    /// Find a free mission slot (type == 0)
    pub fn free_slot(&self) -> Option<usize> {
        for i in 0..4 { if self.m_type(i) == 0 { return Some(i); } }
        None
    }

    /// Set all ship fields for a mission slot
    pub fn set_mission(
        &mut self, slot: usize,
        mission_type: u8, target_galaxy: u16, target_system: u16, target_position: u8, colony_name: [u8; 32], depart_ts: i64, arrive_ts: i64, return_ts: i64,
        lf: u32, hf: u32, cr: u32, bs: u32, bc: u32, bm: u32, ds: u32, de: u32,
        sc: u32, lc: u32, rec: u32, ep: u32, col: u32,
        cargo_metal: u64, cargo_crystal: u64, cargo_deuterium: u64,
    ) {
        match slot {
            0 => { self.m0_type=mission_type; self.m0_target_galaxy=target_galaxy; self.m0_target_system=target_system; self.m0_target_position=target_position; self.m0_colony_name=colony_name; self.m0_depart_ts=depart_ts; self.m0_arrive_ts=arrive_ts; self.m0_return_ts=return_ts; self.m0_light_fighter=lf; self.m0_heavy_fighter=hf; self.m0_cruiser=cr; self.m0_battleship=bs; self.m0_battlecruiser=bc; self.m0_bomber=bm; self.m0_destroyer=ds; self.m0_deathstar=de; self.m0_small_cargo=sc; self.m0_large_cargo=lc; self.m0_recycler=rec; self.m0_espionage_probe=ep; self.m0_colony_ship=col; self.m0_cargo_metal=cargo_metal; self.m0_cargo_crystal=cargo_crystal; self.m0_cargo_deuterium=cargo_deuterium; self.m0_applied=false; }
            1 => { self.m1_type=mission_type; self.m1_target_galaxy=target_galaxy; self.m1_target_system=target_system; self.m1_target_position=target_position; self.m1_colony_name=colony_name; self.m1_depart_ts=depart_ts; self.m1_arrive_ts=arrive_ts; self.m1_return_ts=return_ts; self.m1_light_fighter=lf; self.m1_heavy_fighter=hf; self.m1_cruiser=cr; self.m1_battleship=bs; self.m1_battlecruiser=bc; self.m1_bomber=bm; self.m1_destroyer=ds; self.m1_deathstar=de; self.m1_small_cargo=sc; self.m1_large_cargo=lc; self.m1_recycler=rec; self.m1_espionage_probe=ep; self.m1_colony_ship=col; self.m1_cargo_metal=cargo_metal; self.m1_cargo_crystal=cargo_crystal; self.m1_cargo_deuterium=cargo_deuterium; self.m1_applied=false; }
            2 => { self.m2_type=mission_type; self.m2_target_galaxy=target_galaxy; self.m2_target_system=target_system; self.m2_target_position=target_position; self.m2_colony_name=colony_name; self.m2_depart_ts=depart_ts; self.m2_arrive_ts=arrive_ts; self.m2_return_ts=return_ts; self.m2_light_fighter=lf; self.m2_heavy_fighter=hf; self.m2_cruiser=cr; self.m2_battleship=bs; self.m2_battlecruiser=bc; self.m2_bomber=bm; self.m2_destroyer=ds; self.m2_deathstar=de; self.m2_small_cargo=sc; self.m2_large_cargo=lc; self.m2_recycler=rec; self.m2_espionage_probe=ep; self.m2_colony_ship=col; self.m2_cargo_metal=cargo_metal; self.m2_cargo_crystal=cargo_crystal; self.m2_cargo_deuterium=cargo_deuterium; self.m2_applied=false; }
            _ => { self.m3_type=mission_type; self.m3_target_galaxy=target_galaxy; self.m3_target_system=target_system; self.m3_target_position=target_position; self.m3_colony_name=colony_name; self.m3_depart_ts=depart_ts; self.m3_arrive_ts=arrive_ts; self.m3_return_ts=return_ts; self.m3_light_fighter=lf; self.m3_heavy_fighter=hf; self.m3_cruiser=cr; self.m3_battleship=bs; self.m3_battlecruiser=bc; self.m3_bomber=bm; self.m3_destroyer=ds; self.m3_deathstar=de; self.m3_small_cargo=sc; self.m3_large_cargo=lc; self.m3_recycler=rec; self.m3_espionage_probe=ep; self.m3_colony_ship=col; self.m3_cargo_metal=cargo_metal; self.m3_cargo_crystal=cargo_crystal; self.m3_cargo_deuterium=cargo_deuterium; self.m3_applied=false; }
        }
    }
}
