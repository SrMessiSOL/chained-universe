use bolt_lang::*;
use component_planet::Planet;
use component_resources::Resources;

declare_id!("E94HChSfw57Px2BJPKLnoaj17v6NKN7vXnoQGLSpxUve");

// ── Cost tables (unchanged) ─────────────────────────────────────────────────
fn pow15(n: u64) -> u64 {
    let mut r: u64 = 1_000;
    for _ in 0..n { r = r * 3 / 2; }
    r
}

fn base_cost(idx: u8) -> (u32, u32, u32) {
    match idx {
        0  => (60,        15,       0),
        1  => (48,        24,       0),
        2  => (225,       75,       0),
        3  => (75,        30,       0),
        4  => (900,       360,      900),
        5  => (400,       120,      200),
        6  => (1_000_000, 500_000,  100_000),
        7  => (400,       200,      100),
        8  => (1000,      0,        0),
        9  => (1000,      500,      0),
        10 => (1000,      1000,     0),
        11 => (200,       400,      200),
        12 => (20,        20,       0),
        _  => (0,         0,        0),
    }
}

fn upgrade_cost(idx: u8, level: u64) -> (u64, u64, u64) {
    let (bm, bc, bd) = base_cost(idx);
    let mult = pow15(level.saturating_sub(1));
    (
        (bm as u64 * mult) / 1_000,
        (bc as u64 * mult) / 1_000,
        (bd as u64 * mult) / 1_000,
    )
}

fn build_seconds(idx: u8, level: u64, robotics: u64) -> i64 {
    let (bm, bc, _) = base_cost(idx);
    let total = ((bm as u64 + bc as u64) * pow15(level.saturating_sub(1))) / 1_000;
    (total / (5u64 * (1 + robotics)).max(1)).max(1) as i64
}

// ── Resource helpers (unchanged) ────────────────────────────────────────────
fn settle_resources(res: &mut Resources, now: i64) {
    if res.last_update_ts <= 0 || now <= res.last_update_ts {
        res.last_update_ts = now;
        return;
    }
    let dt = (now - res.last_update_ts) as u64;

    let eff_num = if res.energy_consumption == 0 {
        10_000u64
    } else {
        (res.energy_production * 10_000 / res.energy_consumption).min(10_000)
    };

    let add_res = |current: u64, rate_per_hour: u64, cap: u64| -> u64 {
        let produced = rate_per_hour.saturating_mul(dt).saturating_mul(eff_num) / 3600 / 10_000;
        current.saturating_add(produced).min(cap)
    };

    res.metal     = add_res(res.metal,     res.metal_hour,     res.metal_cap);
    res.crystal   = add_res(res.crystal,   res.crystal_hour,   res.crystal_cap);
    res.deuterium = add_res(res.deuterium, res.deuterium_hour, res.deuterium_cap);
    res.last_update_ts = now;
}

fn recalculate_rates(planet: &Planet, res: &mut Resources) {
    let mine_rate = |level: u8, base: u64| -> u64 {
        if level == 0 { return 0; }
        base * (level as u64) * 11u64.pow(level as u32) / 10u64.pow(level as u32)
    };

    res.metal_hour   = mine_rate(planet.metal_mine, 30);
    res.crystal_hour = mine_rate(planet.crystal_mine, 20);

    let temp_factor = (240i32 - planet.temperature as i32).max(0) as u64;
    res.deuterium_hour = if planet.deuterium_synthesizer == 0 {
        0
    } else {
        mine_rate(planet.deuterium_synthesizer, 10) * temp_factor / 200
    };

    let solar_prod = mine_rate(planet.solar_plant, 20);
    let fusion_prod = if planet.fusion_reactor == 0 { 0 } else { mine_rate(planet.fusion_reactor, 30) * 180 / 100 };
    res.energy_production = solar_prod + fusion_prod;

    res.energy_consumption = mine_rate(planet.metal_mine, 10)
        + mine_rate(planet.crystal_mine, 10)
        + mine_rate(planet.deuterium_synthesizer, 20);

    let store_cap = |level: u8| -> u64 {
        if level == 0 { 1_000_000 } else { 1_000_000 * 2u64.pow(level as u32) }
    };
    res.metal_cap     = store_cap(planet.metal_storage);
    res.crystal_cap   = store_cap(planet.crystal_storage);
    res.deuterium_cap = store_cap(planet.deuterium_tank);
}

// ── Updated System ──────────────────────────────────────────────────────────
#[system]
pub mod system_build {
    pub fn execute(ctx: Context<Components>, args: Vec<u8>) -> Result<Components> {
        require!(args.len() >= 10, BuildError::InvalidArgs);

        let instruction = args[0];
        let now = i64::from_le_bytes(args[2..10].try_into().unwrap());

        settle_resources(&mut ctx.accounts.resources, now);

        match instruction {
            // Start build
            0 => {
                let idx     = args[1];
                let current = ctx.accounts.planet.get_level(idx);
                let next    = current + 1;
                let (cm, cc, cd) = upgrade_cost(idx, next as u64);

                require!(
                    ctx.accounts.planet.build_finish_ts == 0 || now >= ctx.accounts.planet.build_finish_ts,
                    BuildError::QueueBusy
                );
                require!(
                    ctx.accounts.planet.used_fields < ctx.accounts.planet.max_fields,
                    BuildError::NoFields
                );
                require!(ctx.accounts.resources.metal     >= cm, BuildError::InsufficientMetal);
                require!(ctx.accounts.resources.crystal   >= cc, BuildError::InsufficientCrystal);
                require!(ctx.accounts.resources.deuterium >= cd, BuildError::InsufficientDeuterium);

                ctx.accounts.resources.metal     -= cm;
                ctx.accounts.resources.crystal   -= cc;
                ctx.accounts.resources.deuterium -= cd;

                let dur = build_seconds(idx, next as u64, ctx.accounts.planet.robotics_factory as u64);

                ctx.accounts.planet.build_queue_item   = idx;
                ctx.accounts.planet.build_queue_target = next;
                ctx.accounts.planet.build_finish_ts    = now + dur;
                ctx.accounts.planet.used_fields       += 1;
                // planet_index is just stored, no logic change needed
            }

            // Finish build
            1 => {
                require!(ctx.accounts.planet.build_finish_ts > 0, BuildError::NoBuild);
                require!(now >= ctx.accounts.planet.build_finish_ts, BuildError::NotFinished);

                let idx   = ctx.accounts.planet.build_queue_item;
                let level = ctx.accounts.planet.build_queue_target;
                ctx.accounts.planet.set_level(idx, level);

                // Recalculate rates
                let planet_snapshot = PlanetData {
                    temperature:           ctx.accounts.planet.temperature,
                    metal_mine:            ctx.accounts.planet.metal_mine,
                    crystal_mine:          ctx.accounts.planet.crystal_mine,
                    deuterium_synthesizer: ctx.accounts.planet.deuterium_synthesizer,
                    solar_plant:           ctx.accounts.planet.solar_plant,
                    fusion_reactor:        ctx.accounts.planet.fusion_reactor,
                    metal_storage:         ctx.accounts.planet.metal_storage,
                    crystal_storage:       ctx.accounts.planet.crystal_storage,
                    deuterium_tank:        ctx.accounts.planet.deuterium_tank,
                };
                recalc_rates_from_snapshot(&planet_snapshot, &mut ctx.accounts.resources);

                ctx.accounts.planet.build_queue_item   = 255;
                ctx.accounts.planet.build_queue_target = 0;
                ctx.accounts.planet.build_finish_ts    = 0;
            }

            _ => return Err(BuildError::InvalidArgs.into()),
        }

        Ok(ctx.accounts)
    }

    #[system_input]
    pub struct Components {
        pub planet:    Planet,
        pub resources: Resources,
    }
}

// PlanetData snapshot (unchanged)
struct PlanetData {
    temperature:           i16,
    metal_mine:            u8,
    crystal_mine:          u8,
    deuterium_synthesizer: u8,
    solar_plant:           u8,
    fusion_reactor:        u8,
    metal_storage:         u8,
    crystal_storage:       u8,
    deuterium_tank:        u8,
}

fn recalc_rates_from_snapshot(p: &PlanetData, res: &mut Resources) {
    let mine_rate = |level: u8, base: u64| -> u64 {
        if level == 0 { return 0; }
        base * (level as u64) * 11u64.pow(level as u32) / 10u64.pow(level as u32)
    };

    res.metal_hour   = mine_rate(p.metal_mine, 30);
    res.crystal_hour = mine_rate(p.crystal_mine, 20);

    let temp_factor = (240i32 - p.temperature as i32).max(0) as u64;
    res.deuterium_hour = if p.deuterium_synthesizer == 0 { 0 } else { mine_rate(p.deuterium_synthesizer, 10) * temp_factor / 200 };

    let solar_prod = mine_rate(p.solar_plant, 20);
    let fusion_prod = if p.fusion_reactor == 0 { 0 } else { mine_rate(p.fusion_reactor, 30) * 180 / 100 };
    res.energy_production = solar_prod + fusion_prod;

    res.energy_consumption = mine_rate(p.metal_mine, 10) + mine_rate(p.crystal_mine, 10) + mine_rate(p.deuterium_synthesizer, 20);

    let store_cap = |level: u8| -> u64 { if level == 0 { 1_000_000 } else { 1_000_000 * 2u64.pow(level as u32) } };
    res.metal_cap     = store_cap(p.metal_storage);
    res.crystal_cap   = store_cap(p.crystal_storage);
    res.deuterium_cap = store_cap(p.deuterium_tank);
}

#[error_code]
pub enum BuildError {
    #[msg("Invalid args")]           InvalidArgs,
    #[msg("Queue busy")]             QueueBusy,
    #[msg("No fields")]              NoFields,
    #[msg("Insufficient metal")]     InsufficientMetal,
    #[msg("Insufficient crystal")]   InsufficientCrystal,
    #[msg("Insufficient deuterium")] InsufficientDeuterium,
    #[msg("No build in progress")]   NoBuild,
    #[msg("Not finished")]           NotFinished,
}