use bolt_lang::*;

declare_id!("CP6KoShdHvgZbGubYLct1EcQLmngZ1nsWmaKQhbJRtss");

/// Resources Component
///
/// On-chain layout (after 8-byte discriminator):
///   [8..16]   metal            u64
///   [16..24]  crystal          u64
///   [24..32]  deuterium        u64
///   [32..40]  metal_hour       u64
///   [40..48]  crystal_hour     u64
///   [48..56]  deuterium_hour   u64
///   [56..64]  energy_production u64
///   [64..72]  energy_consumption u64
///   [72..80]  metal_cap        u64
///   [80..88]  crystal_cap      u64
///   [88..96]  deuterium_cap    u64
///   [96..104] last_update_ts   i64
///   [END-32..END] bolt_metadata
#[component(delegate)]
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
            bolt_metadata:      Default::default(),
            metal:              0,
            crystal:            0,
            deuterium:          0,
            metal_hour:         0,
            crystal_hour:       0,
            deuterium_hour:     0,
            energy_production:  0,
            energy_consumption: 0,
            metal_cap:          0,
            crystal_cap:        0,
            deuterium_cap:      0,
            last_update_ts:     0,
        }
    }
}