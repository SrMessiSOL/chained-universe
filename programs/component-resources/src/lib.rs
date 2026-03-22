use bolt_lang::*;

declare_id!("CP6KoShdHvgZbGubYLct1EcQLmngZ1nsWmaKQhbJRtss");

/// Resources Component
/// Tracks stockpiles using lazy-settlement: production is computed from
/// `last_update_ts` at read-time so no background crank is needed.
///
/// All amounts stored as u64 raw units (not fixed-point) for simplicity.
#[component]
pub struct Resources {
    // ── Stockpiles ────────────────────────────────────────────────────────
    pub metal: u64,
    pub crystal: u64,
    pub deuterium: u64,

    // ── Hourly production rates (units/hour) ──────────────────────────────
    pub metal_hour: u64,
    pub crystal_hour: u64,
    pub deuterium_hour: u64,

    // ── Energy balance ────────────────────────────────────────────────────
    pub energy_production: u64,
    pub energy_consumption: u64,

    // ── Storage caps ──────────────────────────────────────────────────────
    pub metal_cap: u64,
    pub crystal_cap: u64,
    pub deuterium_cap: u64,

    /// Last time resources were settled (Unix seconds)
    pub last_update_ts: i64,
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
        let elapsed_secs = (now - self.last_update_ts) as u64;

        // Efficiency: 0–100 scaled to 0–10_000
        let eff = if self.energy_consumption == 0 {
            10_000u64
        } else {
            (self.energy_production.min(self.energy_consumption) * 10_000)
                / self.energy_consumption
        };

        let dm = (self.metal_hour * elapsed_secs * eff) / (3600 * 10_000);
        let dc = (self.crystal_hour * elapsed_secs * eff) / (3600 * 10_000);
        let dd = (self.deuterium_hour * elapsed_secs * eff) / (3600 * 10_000);
        (dm, dc, dd)
    }

    /// Apply pending production, respecting storage caps.
    pub fn settle(&mut self, now: i64) {
        let (dm, dc, dd) = self.pending(now);
        self.metal     = (self.metal     + dm).min(self.metal_cap);
        self.crystal   = (self.crystal   + dc).min(self.crystal_cap);
        self.deuterium = (self.deuterium + dd).min(self.deuterium_cap);
        self.last_update_ts = now;
    }

    /// Recompute hourly rates and caps from planet building levels.
    /// Call this after any building upgrade completes.
    pub fn recalculate(&mut self, planet: &PlanetSnapshot) {
        let temp = planet.temperature as f64;

        self.metal_hour = mine_output(planet.metal_mine as u64, 30, 1.1);
        self.crystal_hour = mine_output(planet.crystal_mine as u64, 20, 1.1);
        self.deuterium_hour =
            deut_output(planet.deuterium_synthesizer as u64, temp);

        let solar = solar_output(planet.solar_plant as u64);
        let fusion = fusion_output(planet.fusion_reactor as u64);
        self.energy_production = solar + fusion;

        self.energy_consumption = mine_energy(
            planet.metal_mine as u64,
            planet.crystal_mine as u64,
            planet.deuterium_synthesizer as u64,
        );

        self.metal_cap    = storage_cap(planet.metal_storage as u64);
        self.crystal_cap  = storage_cap(planet.crystal_storage as u64);
        self.deuterium_cap = storage_cap(planet.deuterium_tank as u64);
    }
}

/// Minimal snapshot of planet fields needed for rate calculation
/// (avoids a circular dependency between components).
pub struct PlanetSnapshot {
    pub temperature: i16,
    pub metal_mine: u8,
    pub crystal_mine: u8,
    pub deuterium_synthesizer: u8,
    pub solar_plant: u8,
    pub fusion_reactor: u8,
    pub metal_storage: u8,
    pub crystal_storage: u8,
    pub deuterium_tank: u8,
}

// ── Formula helpers ────────────────────────────────────────────────────────

/// Generic mine output: base_factor × level × multiplier^level
fn mine_output(level: u64, base: u64, mult_fp: f64) -> u64 {
    if level == 0 { return base; }
    let v = (base as f64) * (level as f64) * mult_fp.powi(level as i32);
    v as u64
}

fn deut_output(level: u64, temp: f64) -> u64 {
    if level == 0 { return 0; }
    let base = 10.0 * (level as f64) * 1.1f64.powi(level as i32);
    let factor = 1.44 - 0.004 * temp;
    (base * factor.max(0.1)) as u64
}

fn solar_output(level: u64) -> u64 {
    if level == 0 { return 0; }
    (20.0 * (level as f64) * 1.1f64.powi(level as i32)) as u64
}

fn fusion_output(level: u64) -> u64 {
    if level == 0 { return 0; }
    (30.0 * (level as f64) * 1.05f64.powi(level as i32)) as u64
}

fn mine_energy(m: u64, c: u64, d: u64) -> u64 {
    let em = if m == 0 { 0 } else { (10.0 * (m as f64) * 1.1f64.powi(m as i32)) as u64 };
    let ec = if c == 0 { 0 } else { (10.0 * (c as f64) * 1.1f64.powi(c as i32)) as u64 };
    let ed = if d == 0 { 0 } else { (20.0 * (d as f64) * 1.1f64.powi(d as i32)) as u64 };
    em + ec + ed
}

fn storage_cap(level: u64) -> u64 {
    if level == 0 { return 10_000; }
    (5_000.0 * 2.5f64.powi(level as i32)) as u64
}
