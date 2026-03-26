use bolt_lang::*;

declare_id!("5UuCSuNqVXwCd7qPFQXj8Kp7DAqbB5ZuHFLZZ32paPLD");

/// Fleet Component
#[component_deserialize]
#[derive(Default)]
pub struct Mission {
    pub mission_type:      u8,
    pub destination:       Pubkey,
    pub depart_ts:         i64,
    pub arrive_ts:         i64,
    pub return_ts:         i64,
    pub s_small_cargo:     u32,
    pub s_large_cargo:     u32,
    pub s_light_fighter:   u32,
    pub s_heavy_fighter:   u32,
    pub s_cruiser:         u32,
    pub s_battleship:      u32,
    pub s_battlecruiser:   u32,
    pub s_bomber:          u32,
    pub s_destroyer:       u32,
    pub s_deathstar:       u32,
    pub s_recycler:        u32,
    pub s_espionage_probe: u32,
    pub s_colony_ship:     u32,
    pub cargo_metal:       u64,
    pub cargo_crystal:     u64,
    pub cargo_deuterium:   u64,
    pub applied:           bool,
}

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
    pub missions:         [Mission; 4],
}

impl Default for Fleet {
    fn default() -> Self {
        Self {
            bolt_metadata:    Default::default(),
            creator:          Pubkey::default(),
            small_cargo:      0,
            large_cargo:      0,
            light_fighter:    0,
            heavy_fighter:    0,
            cruiser:          0,
            battleship:       0,
            battlecruiser:    0,
            bomber:           0,
            destroyer:        0,
            deathstar:        0,
            recycler:         0,
            espionage_probe:  0,
            colony_ship:      0,
            solar_satellite:  0,
            active_missions:  0,
            missions:         [Mission::default(), Mission::default(), Mission::default(), Mission::default()],
        }
    }
}