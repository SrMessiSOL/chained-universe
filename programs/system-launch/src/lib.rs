use bolt_lang::*;
use component_fleet::{Fleet, Mission};
use component_resources::Resources;

declare_id!("9aHGFS8VAfbEYYCkEGQBBuTKApkD5aiHotH77kMgB5bT");

fn u32_at(b: &[u8], o: usize) -> u32 {
    u32::from_le_bytes(b[o..o+4].try_into().unwrap_or([0;4]))
}
fn u64_at(b: &[u8], o: usize) -> u64 {
    u64::from_le_bytes(b[o..o+8].try_into().unwrap_or([0;8]))
}
fn i64_at(b: &[u8], o: usize) -> i64 {
    i64::from_le_bytes(b[o..o+8].try_into().unwrap_or([0;8]))
}

fn require_component_authority(
    authority: &AccountInfo,
    fleet: &Fleet,
    resources: &Resources,
) -> Result<()> {
    require!(authority.is_signer, LaunchError::Unauthorized);
    require_keys_eq!(fleet.bolt_metadata.authority, *authority.key, LaunchError::Unauthorized);
    require_keys_eq!(resources.bolt_metadata.authority, *authority.key, LaunchError::Unauthorized);
    Ok(())
}

#[system]
pub mod system_launch {
    pub fn execute(ctx: Context<Components>, args: Vec<u8>) -> Result<Components> {
        require_component_authority(
            &ctx.accounts.authority,
            &ctx.accounts.fleet,
            &ctx.accounts.resources,
        )?;

        require!(args.len() >= 94, LaunchError::InvalidArgs);

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

        require!(mission_type >= 1 && mission_type <= 6, LaunchError::InvalidMission);
        require!(flight_seconds > 0, LaunchError::InvalidArgs);
        require!(lf+hf+cr+bs+bc+bm+ds+de+sc+lc+rec+ep+col > 0, LaunchError::EmptyFleet);

        ctx.accounts.resources.settle(now);

        let slot = {
            let mut found = None;
            for (i, m) in ctx.accounts.fleet.missions.iter().enumerate() {
                if m.mission_type == 0 { found = Some(i); break; }
            }
            found.ok_or(LaunchError::NoSlot)?
        };

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

        f.missions[slot] = Mission {
            mission_type,
            destination: Pubkey::default(),
            depart_ts: now, arrive_ts: now + flight_seconds, return_ts: 0,
            s_light_fighter: lf, s_heavy_fighter: hf, s_cruiser: cr,
            s_battleship: bs, s_battlecruiser: bc, s_bomber: bm,
            s_destroyer: ds, s_deathstar: de, s_small_cargo: sc,
            s_large_cargo: lc, s_recycler: rec, s_espionage_probe: ep,
            s_colony_ship: col, cargo_metal, cargo_crystal, cargo_deuterium,
            applied: false,
        };
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
    #[msg("Unauthorized")]            Unauthorized,
    #[msg("Invalid mission")]         InvalidMission,
    #[msg("Empty fleet")]             EmptyFleet,
    #[msg("No fleet slot")]           NoSlot,
    #[msg("Insufficient ships")]      InsufficientShips,
    #[msg("Exceeds cargo")]           ExceedsCargo,
    #[msg("Insufficient resources")]  InsufficientResources,
    #[msg("Insufficient deuterium")]  InsufficientDeuterium,
}
