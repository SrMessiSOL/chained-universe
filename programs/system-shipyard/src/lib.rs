use bolt_lang::*;
use component_fleet::Fleet;
use component_resources::Resources;

declare_id!("FTav8UK4RKawqyGWRakZhe1zhYV7PUJgPwHK7UnEqnN9");

fn ship_cost(ship_type: u8) -> (u64, u64, u64) {
    match ship_type {
        0  => (2000,    2000,    0),
        1  => (6000,    6000,    0),
        2  => (3000,    1000,    0),
        3  => (6000,    4000,    0),
        4  => (20000,   7000,    2000),
        5  => (45000,   15000,   0),
        6  => (30000,   40000,   15000),
        7  => (50000,   25000,   15000),
        8  => (60000,   50000,   15000),
        9  => (5000000, 4000000, 1000000),
        10 => (10000,   6000,    2000),
        11 => (0,       1000,    0),
        12 => (10000,   20000,   10000),
        13 => (0,       2000,    500),
        _  => (0,       0,       0),
    }
}

fn settle_resources(res: &mut Resources, now: i64) {
    if res.last_update_ts <= 0 || now <= res.last_update_ts {
        res.last_update_ts = now;
        return;
    }
    let dt = (now - res.last_update_ts) as u64;
    let eff = if res.energy_consumption == 0 {
        10_000u64
    } else {
        (res.energy_production * 10_000 / res.energy_consumption).min(10_000)
    };
    let produce = |current: u64, rate: u64, cap: u64| -> u64 {
        let gained = rate.saturating_mul(dt).saturating_mul(eff) / 3600 / 10_000;
        current.saturating_add(gained).min(cap)
    };
    res.metal     = produce(res.metal,     res.metal_hour,     res.metal_cap);
    res.crystal   = produce(res.crystal,   res.crystal_hour,   res.crystal_cap);
    res.deuterium = produce(res.deuterium, res.deuterium_hour, res.deuterium_cap);
    res.last_update_ts = now;
}

#[system]
pub mod system_shipyard {
    pub fn execute(ctx: Context<Components>, args: Vec<u8>) -> Result<Components> {
        // Only require is_signer — NOT bolt_metadata.authority.
        // During an ER session the burner keypair signs; its pubkey differs from
        // the wallet stored in bolt_metadata. The ER validator enforces ownership.
        // Arg layout:
        //   [0]     ship_type: u8
        //   [1..5]  quantity:  u32 (little-endian)
        //   [5..13] now:       i64 (little-endian)
        require!(args.len() >= 13, ShipyardError::InvalidArgs);
        let ship_type = args[0];
        let quantity  = u32::from_le_bytes(args[1..5].try_into().unwrap());
        let now       = i64::from_le_bytes(args[5..13].try_into().unwrap());
        require!(quantity > 0, ShipyardError::InvalidArgs);

        settle_resources(&mut ctx.accounts.resources, now);

        let (cm, cc, cd) = ship_cost(ship_type);
        let total_m = cm * quantity as u64;
        let total_c = cc * quantity as u64;
        let total_d = cd * quantity as u64;

        require!(ctx.accounts.resources.metal     >= total_m, ShipyardError::InsufficientMetal);
        require!(ctx.accounts.resources.crystal   >= total_c, ShipyardError::InsufficientCrystal);
        require!(ctx.accounts.resources.deuterium >= total_d, ShipyardError::InsufficientDeuterium);

        ctx.accounts.resources.metal     -= total_m;
        ctx.accounts.resources.crystal   -= total_c;
        ctx.accounts.resources.deuterium -= total_d;

        let f = &mut ctx.accounts.fleet;
        match ship_type {
            0  => f.small_cargo     = f.small_cargo.saturating_add(quantity),
            1  => f.large_cargo     = f.large_cargo.saturating_add(quantity),
            2  => f.light_fighter   = f.light_fighter.saturating_add(quantity),
            3  => f.heavy_fighter   = f.heavy_fighter.saturating_add(quantity),
            4  => f.cruiser         = f.cruiser.saturating_add(quantity),
            5  => f.battleship      = f.battleship.saturating_add(quantity),
            6  => f.battlecruiser   = f.battlecruiser.saturating_add(quantity),
            7  => f.bomber          = f.bomber.saturating_add(quantity),
            8  => f.destroyer       = f.destroyer.saturating_add(quantity),
            9  => f.deathstar       = f.deathstar.saturating_add(quantity),
            10 => f.recycler        = f.recycler.saturating_add(quantity),
            11 => f.espionage_probe = f.espionage_probe.saturating_add(quantity),
            12 => f.colony_ship     = f.colony_ship.saturating_add(quantity),
            13 => f.solar_satellite = f.solar_satellite.saturating_add(quantity),
            _  => return Err(ShipyardError::InvalidShipType.into()),
        }

        Ok(ctx.accounts)
    }

    #[system_input]
    pub struct Components {
        pub fleet:     Fleet,
        pub resources: Resources,
    }
}

#[error_code]
pub enum ShipyardError {
    #[msg("Invalid args")]           InvalidArgs,
    #[msg("Unauthorized")]           Unauthorized,
    #[msg("Invalid ship type")]      InvalidShipType,
    #[msg("Insufficient metal")]     InsufficientMetal,
    #[msg("Insufficient crystal")]   InsufficientCrystal,
    #[msg("Insufficient deuterium")] InsufficientDeuterium,
}