use bolt_lang::*;
use component_fleet::Fleet;
use component_resources::Resources;

declare_id!("BVn9NZ51LqhbDowqhaJvxmXK6VGsP1k3dLtJEL8Fjmxv");

fn u32_at(b: &[u8], o: usize) -> u32 {
    u32::from_le_bytes(b[o..o+4].try_into().unwrap_or([0;4]))
}
fn u64_at(b: &[u8], o: usize) -> u64 {
    u64::from_le_bytes(b[o..o+8].try_into().unwrap_or([0;8]))
}
fn i64_at(b: &[u8], o: usize) -> i64 {
    i64::from_le_bytes(b[o..o+8].try_into().unwrap_or([0;8]))
}
const TRANSPORT_MISSION: u8 = 2;
const COLONIZE_MISSION: u8 = 5;
const BASE_ARGS_LEN: usize = 94;
const TARGET_COORDS_OFFSET: usize = 94;
const TARGET_COORDS_LEN: usize = 5;
const COLONY_NAME_LEN: usize = 32;

fn colony_name_at(b: &[u8], o: usize) -> [u8; COLONY_NAME_LEN] {
    let mut name = [0u8; COLONY_NAME_LEN];
    if b.len() >= o + COLONY_NAME_LEN {
        name.copy_from_slice(&b[o..o + COLONY_NAME_LEN]);
    }
    name
}

// FIX: Resources has no .settle() method — inline the logic as a free function.
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
        let produced = rate_per_hour
            .saturating_mul(dt)
            .saturating_mul(eff_num)
            / 3600
            / 10_000;
        current.saturating_add(produced).min(cap)
    };
    res.metal     = add_res(res.metal,     res.metal_hour,     res.metal_cap);
    res.crystal   = add_res(res.crystal,   res.crystal_hour,   res.crystal_cap);
    res.deuterium = add_res(res.deuterium, res.deuterium_hour, res.deuterium_cap);
    res.last_update_ts = now;
}

#[system]
pub mod system_launch {
    pub fn execute(ctx: Context<Components>, args: Vec<u8>) -> Result<Components> {

        require!(args.len() >= BASE_ARGS_LEN, LaunchError::InvalidArgs);

        let mission_type   = args[0];
        let lf  = u32_at(&args, 1);
        let hf  = u32_at(&args, 5);
        let cr  = u32_at(&args, 9);
        let bs  = u32_at(&args, 13);
        let bc  = u32_at(&args, 17);
        let bm  = u32_at(&args, 21);
        let ds  = u32_at(&args, 25);
        let de  = u32_at(&args, 29);
        let sc  = u32_at(&args, 33);
        let lc  = u32_at(&args, 37);
        let rec = u32_at(&args, 41);
        let ep  = u32_at(&args, 45);
        let col = u32_at(&args, 49);
        let cargo_metal     = u64_at(&args, 53);
        let cargo_crystal   = u64_at(&args, 61);
        let cargo_deuterium = u64_at(&args, 69);
        let speed_factor    = args[77].max(10).min(100);
        let now             = i64_at(&args, 78);
        let flight_seconds  = i64_at(&args, 86);
        require!(
            args.len() >= TARGET_COORDS_OFFSET + TARGET_COORDS_LEN,
            LaunchError::MissingTarget
        );
        let target_galaxy = u16::from_le_bytes(
            args[TARGET_COORDS_OFFSET..TARGET_COORDS_OFFSET + 2]
                .try_into()
                .unwrap_or([0; 2]),
        );
        let target_system = u16::from_le_bytes(
            args[TARGET_COORDS_OFFSET + 2..TARGET_COORDS_OFFSET + 4]
                .try_into()
                .unwrap_or([0; 2]),
        );
        let target_position = args[TARGET_COORDS_OFFSET + 4];
        require!((1..=9).contains(&target_galaxy), LaunchError::InvalidTarget);
        require!((1..=499).contains(&target_system), LaunchError::InvalidTarget);
        require!((1..=15).contains(&target_position), LaunchError::InvalidTarget);

        let colony_name = match mission_type {
            TRANSPORT_MISSION => [0u8; COLONY_NAME_LEN],
            COLONIZE_MISSION => {
                require!(
                    args.len() >= TARGET_COORDS_OFFSET + TARGET_COORDS_LEN + COLONY_NAME_LEN,
                    LaunchError::MissingColonyName
                );
                colony_name_at(&args, TARGET_COORDS_OFFSET + TARGET_COORDS_LEN)
            }
            _ => [0u8; COLONY_NAME_LEN],
        };
        let return_ts = match mission_type {
            TRANSPORT_MISSION => now.saturating_add(flight_seconds.saturating_mul(2)),
            COLONIZE_MISSION => 0,
            _ => 0,
        };

        require!(mission_type == TRANSPORT_MISSION || mission_type == COLONIZE_MISSION, LaunchError::InvalidMission);
        require!(flight_seconds > 0, LaunchError::InvalidArgs);
        require!(lf+hf+cr+bs+bc+bm+ds+de+sc+lc+rec+ep+col > 0, LaunchError::EmptyFleet);

        // FIX: was ctx.accounts.resources.settle(now) — method doesn't exist.
        settle_resources(&mut ctx.accounts.resources, now);

        let slot = ctx.accounts.fleet.free_slot().ok_or(LaunchError::NoSlot)?;

        let f = &ctx.accounts.fleet;
        require!(f.light_fighter   >= lf,  LaunchError::InsufficientShips);
        require!(f.heavy_fighter   >= hf,  LaunchError::InsufficientShips);
        require!(f.cruiser         >= cr,  LaunchError::InsufficientShips);
        require!(f.battleship      >= bs,  LaunchError::InsufficientShips);
        require!(f.battlecruiser   >= bc,  LaunchError::InsufficientShips);
        require!(f.bomber          >= bm,  LaunchError::InsufficientShips);
        require!(f.destroyer       >= ds,  LaunchError::InsufficientShips);
        require!(f.deathstar       >= de,  LaunchError::InsufficientShips);
        require!(f.small_cargo     >= sc,  LaunchError::InsufficientShips);
        require!(f.large_cargo     >= lc,  LaunchError::InsufficientShips);
        require!(f.recycler        >= rec, LaunchError::InsufficientShips);
        require!(f.espionage_probe >= ep,  LaunchError::InsufficientShips);
        require!(f.colony_ship     >= col, LaunchError::InsufficientShips);

        let cap = sc as u64*5_000 + lc as u64*25_000 + rec as u64*20_000
            + cr as u64*800 + bs as u64*1_500;
        require!(cargo_metal+cargo_crystal+cargo_deuterium <= cap, LaunchError::ExceedsCargo);

        let res = &mut ctx.accounts.resources;
        require!(res.metal     >= cargo_metal,     LaunchError::InsufficientResources);
        require!(res.crystal   >= cargo_crystal,   LaunchError::InsufficientResources);
        require!(res.deuterium >= cargo_deuterium, LaunchError::InsufficientResources);

        let fuel = (sc as u64*10 + lc as u64*50 + lf as u64*20 + hf as u64*75
            + cr as u64*300 + bs as u64*500 + bc as u64*250 + bm as u64*1_000
            + ds as u64*1_000 + rec as u64*300 + ep as u64 + col as u64*1_000)
            * (speed_factor as u64).pow(2) / 10_000;
        require!(res.deuterium >= cargo_deuterium + fuel, LaunchError::InsufficientDeuterium);

        res.metal     -= cargo_metal;
        res.crystal   -= cargo_crystal;
        res.deuterium -= cargo_deuterium + fuel;

        let f = &mut ctx.accounts.fleet;
        f.light_fighter   -= lf; f.heavy_fighter -= hf;
        f.cruiser         -= cr; f.battleship    -= bs;
        f.battlecruiser   -= bc; f.bomber        -= bm;
        f.destroyer       -= ds; f.deathstar     -= de;
        f.small_cargo     -= sc; f.large_cargo   -= lc;
        f.recycler        -= rec; f.espionage_probe -= ep;
        f.colony_ship     -= col;
        f.set_mission(
            slot,
            mission_type,
            target_galaxy,
            target_system,
            target_position,
            colony_name,
            now,
            now.saturating_add(flight_seconds),
            return_ts,
            lf, hf, cr, bs, bc, bm, ds, de,
            sc, lc, rec, ep, col,
            cargo_metal, cargo_crystal, cargo_deuterium,
        );
        f.active_missions = f.active_missions.saturating_add(1);

        Ok(ctx.accounts)
    }

    #[system_input]
    pub struct Components {
        pub fleet:     Fleet,
        pub resources: Resources,
    }
}

#[error_code]
pub enum LaunchError {
    #[msg("Invalid args")]            InvalidArgs,
    #[msg("Invalid mission")]         InvalidMission,
    #[msg("Mission target is required")] MissingTarget,
    #[msg("Mission target is invalid")] InvalidTarget,
    #[msg("Colonize missions require a colony name")] MissingColonyName,
    #[msg("Empty fleet")]             EmptyFleet,
    #[msg("No fleet slot")]           NoSlot,
    #[msg("Insufficient ships")]      InsufficientShips,
    #[msg("Exceeds cargo")]           ExceedsCargo,
    #[msg("Insufficient resources")]  InsufficientResources,
    #[msg("Insufficient deuterium")]  InsufficientDeuterium,
}
