use bolt_lang::*;

declare_id!("4AAQeP54KQy4HSjMsMS9VwVY8mWy4BisdsTwSxen4Df6");

/// Planet Component
/// Stores per-planet metadata and all building levels.
/// Attached to a planet Entity via the BOLT World program.
#[component]
#[derive(Default)]
pub struct Planet {
    /// Owner wallet
    pub owner: Pubkey,
    /// Planet name (UTF-8, max 32 bytes)
    pub name: [u8; 32],
    /// Universe coordinates
    pub galaxy: u16,
    pub system: u16,
    pub position: u8,
    /// Physical properties
    pub diameter: u32,       // km
    pub temperature: i16,    // °C
    pub max_fields: u16,
    pub used_fields: u16,

    // ── Building levels ───────────────────────────────────────────────────
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

    // ── Build queue (one slot) ────────────────────────────────────────────
    /// Building type index (0–12); 255 = none
    pub build_queue_item: u8,
    /// Target level after upgrade
    pub build_queue_target: u8,
    /// Unix timestamp when build completes (0 = idle)
    pub build_finish_ts: i64,
}

impl Planet {
    /// Return the current level for a building type index
    pub fn get_level(&self, idx: u8) -> u8 {
        match idx {
            0  => self.metal_mine,
            1  => self.crystal_mine,
            2  => self.deuterium_synthesizer,
            3  => self.solar_plant,
            4  => self.fusion_reactor,
            5  => self.robotics_factory,
            6  => self.nanite_factory,
            7  => self.shipyard,
            8  => self.metal_storage,
            9  => self.crystal_storage,
            10 => self.deuterium_tank,
            11 => self.research_lab,
            12 => self.missile_silo,
            _  => 0,
        }
    }

    /// Apply a completed level-up
    pub fn set_level(&mut self, idx: u8, level: u8) {
        match idx {
            0  => self.metal_mine = level,
            1  => self.crystal_mine = level,
            2  => self.deuterium_synthesizer = level,
            3  => self.solar_plant = level,
            4  => self.fusion_reactor = level,
            5  => self.robotics_factory = level,
            6  => self.nanite_factory = level,
            7  => self.shipyard = level,
            8  => self.metal_storage = level,
            9  => self.crystal_storage = level,
            10 => self.deuterium_tank = level,
            11 => self.research_lab = level,
            12 => self.missile_silo = level,
            _  => {}
        }
    }
}
