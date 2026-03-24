use bolt_lang::*;

declare_id!("CP6KoShdHvgZbGubYLct1EcQLmngZ1nsWmaKQhbJRtss");

/// Resources Component
///
/// All amounts stored as u64 raw units (not fixed-point) for simplicity.
#[component]
pub struct Resources {
    pub metal:              u64,
    pub crystal:            u64,
    pub deuterium:          u64,
    pub metal_hour:         u64,
    pub crystal_hour:       u64,
    pub deuterium_hour:     u64,
    pub energy_production:  u64,
    pub energy_consumption: u64,
    pub metal_cap:          u64,
    pub crystal_cap:        u64,
    pub deuterium_cap:      u64,
    pub last_update_ts:     i64,
}

impl Default for Resources {
    fn default() -> Self {
        Self {
            bolt_metadata: Default::default(),
            metal: 500,
            crystal: 500,
            deuterium: 0,
            metal_hour: 33,
            crystal_hour: 22,
            deuterium_hour: 14,
            energy_production: 22,
            energy_consumption: 42,
            metal_cap: 10_000,
            crystal_cap: 10_000,
            deuterium_cap: 10_000,
            last_update_ts: 0,
        }
    }
}

impl Resources {
    /// Compute production delta since `last_update_ts` without mutating.
    /// Applies energy efficiency ratio if in deficit.
    pub fn pending(&self, now: i64) -> (u64, u64, u64) {
        if now <= self.last_update_ts {
            return (0, 0, 0);
        }
    }
}