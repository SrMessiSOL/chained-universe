use bolt_lang::*;
use component_planet::Planet;
use component_resources::Resources;

declare_id!("kk7e2mNXHaU3VVtmtzLCZGYP88MDL7EbkFbb9sySfiV");

fn pow15(n: u64) -> u64 {
    let mut r: u64 = 1_000;
    for _ in 0..n { r = r * 3 / 2; }
    r
}

fn base_cost(idx: u8) -> (u32, u32, u32) {
    match idx {
        0  => (60,   15,   0),
        1  => (48,   24,   0),
        2  => (225,  75,   0),
        3  => (75,   30,   0),
        4  => (900,  360,  900),
        5  => (400,  120,  200),
        6  => (1_000_000, 500_000, 100_000),
        7  => (400,  200,  100),
        8  => (1000, 0,    0),
        9  => (1000, 500,  0),
        10 => (1000, 1000, 0),
        11 => (200,  400,  200),
        12 => (20,   20,   0),
        _  => (0,    0,    0),
    }
}

fn upgrade_cost(idx: u8, level: u64) -> (u64, u64, u64) {
    let (bm, bc, bd) = base_cost(idx);
    let mult = pow15(level.saturating_sub(1));
    ((bm as u64 * mult) / 1_000, (bc as u64 * mult) / 1_000, (bd as u64 * mult) / 1_000)
}

fn build_seconds(idx: u8, level: u64, robotics: u64) -> i64 {
    let (bm, bc, _) = base_cost(idx);
    let total = ((bm as u64 + bc as u64) * pow15(level.saturating_sub(1))) / 1_000;
    // Divisor 5 gives meaningful times at low levels (15s for metal mine lv1)
    // while scaling to hours at high levels. Original 2_500 was calibrated for
    // OGame's larger base costs and kept everything at 1s here.
    (total / (5u64 * (1 + robotics)).max(1)).max(1) as i64
}

fn require_component_authority(
    authority: &AccountInfo,
    planet: &Planet,
    resources: &Resources,
) -> Result<()> {
    require!(authority.is_signer, BuildError::Unauthorized);
    require_keys_eq!(planet.bolt_metadata.authority, *authority.key, BuildError::Unauthorized);
    require_keys_eq!(resources.bolt_metadata.authority, *authority.key, BuildError::Unauthorized);
    Ok(())
}

#[system]
pub mod system_build {
    pub fn execute(ctx: Context<Components>, args: Vec<u8>) -> Result<Components> {
        require_component_authority(
            &ctx.accounts.authority,
            &ctx.accounts.planet,
            &ctx.accounts.resources,
        )?;

        require!(args.len() >= 10, BuildError::InvalidArgs);
        let instruction = args[0];
        let now = i64::from_le_bytes(args[2..10].try_into().unwrap());
        ctx.accounts.resources.settle(now);

        match instruction {
            0 => {
                let idx     = args[1];
                let current = ctx.accounts.planet.get_level(idx);
                let next    = current + 1;
                let (cm, cc, cd) = upgrade_cost(idx, next as u64);
                require!(
                    ctx.accounts.planet.build_finish_ts == 0
                        || now >= ctx.accounts.planet.build_finish_ts,
                    BuildError::QueueBusy
                );
                require!(ctx.accounts.planet.used_fields < ctx.accounts.planet.max_fields, BuildError::NoFields);
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
            }
            1 => {
                require!(ctx.accounts.planet.build_finish_ts > 0, BuildError::NoBuild);
                require!(now >= ctx.accounts.planet.build_finish_ts, BuildError::NotFinished);
                let idx   = ctx.accounts.planet.build_queue_item;
                let level = ctx.accounts.planet.build_queue_target;
                ctx.accounts.planet.set_level(idx, level);
                let snap = component_resources::PlanetSnapshot {
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
                ctx.accounts.resources.recalculate(&snap);
                // FIX: reset to 255 (not 0) to signal "no build queued"
                // The UI and on-chain logic both treat 255 as the empty sentinel.
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

#[error_code]
pub enum BuildError {
    #[msg("Invalid args")]           InvalidArgs,
    #[msg("Unauthorized")]           Unauthorized,
    #[msg("Queue busy")]             QueueBusy,
    #[msg("No fields")]              NoFields,
    #[msg("Insufficient metal")]     InsufficientMetal,
    #[msg("Insufficient crystal")]   InsufficientCrystal,
    #[msg("Insufficient deuterium")] InsufficientDeuterium,
    #[msg("No build in progress")]   NoBuild,
    #[msg("Not finished")]           NotFinished,
}