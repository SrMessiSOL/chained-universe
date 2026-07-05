use anchor_lang::prelude::*;

use crate::constants::*;
use crate::error::GameStateError;
use crate::state::*;
use anchor_spl::token::{self, Burn, Mint, Token, TokenAccount, Transfer};

// =============================================
// Helper Functions
// =============================================

pub(crate) fn validate_coordinates(galaxy: u16, system: u16, position: u8) -> Result<()> {
    require!(
        (1..=999).contains(&galaxy),
        GameStateError::InvalidCoordinates
    );
    require!(
        (1..=999).contains(&system),
        GameStateError::InvalidCoordinates
    );
    require!(
        (1..=15).contains(&position),
        GameStateError::InvalidCoordinates
    );
    Ok(())
}

pub(crate) fn copy_name<const N: usize>(value: &str, fallback: &str) -> [u8; N] {
    let source = if value.is_empty() { fallback } else { value };
    let bytes = source.as_bytes();
    let mut out = [0u8; N];
    let copy_len = bytes.len().min(N);
    out[..copy_len].copy_from_slice(&bytes[..copy_len]);
    out
}

pub(crate) fn pow15(n: u64) -> u64 {
    let mut r: u64 = 1_000;
    for _ in 0..n {
        r = r * 3 / 2;
    }
    r
}

pub(crate) fn base_cost(idx: u8) -> (u32, u32, u32) {
    match idx {
        0 => (60, 15, 0),
        1 => (48, 24, 0),
        2 => (225, 75, 0),
        3 => (75, 30, 0),
        4 => (900, 360, 900),
        5 => (400, 120, 200),
        6 => (1_000_000, 500_000, 100_000),
        7 => (400, 200, 100),
        8 => (1000, 0, 0),
        9 => (1000, 500, 0),
        10 => (1000, 1000, 0),
        11 => (200, 400, 200),
        12 => (20, 20, 0),
        _ => (0, 0, 0),
    }
}

pub(crate) fn upgrade_cost(idx: u8, level: u64) -> (u64, u64, u64) {
    let (bm, bc, bd) = base_cost(idx);
    let mult = pow15(level.saturating_sub(1));
    (
        (bm as u64 * mult) / 1_000,
        (bc as u64 * mult) / 1_000,
        (bd as u64 * mult) / 1_000,
    )
}

pub(crate) fn build_seconds(idx: u8, level: u64, robotics: u64) -> i64 {
    let (bm, bc, _) = base_cost(idx);
    let total = ((bm as u64 + bc as u64) * pow15(level.saturating_sub(1))) / 1_000;
    (total / (5u64 * (1 + robotics)).max(1)).max(1) as i64
}

pub(crate) fn research_base_cost(idx: u8) -> (u64, u64, u64) {
    match idx {
        0 => (0, 800, 400),
        1 => (400, 0, 600),
        2 => (2000, 4000, 600),
        3 => (10000, 20000, 6000),
        4 => (0, 400, 600),
        5 => (4000, 2000, 1000),
        6 => (240000, 400000, 160000),
        7 => (800, 200, 0),
        8 => (200, 600, 0),
        9 => (1000, 0, 0),
        _ => (0, 0, 0),
    }
}

pub(crate) fn research_lab_requirement(idx: u8) -> u8 {
    match idx {
        0 | 1 | 4 | 7 | 8 | 9 => 1,
        5 => 3,
        2 => 5,
        3 => 7,
        6 => 10,
        _ => 255,
    }
}

pub(crate) fn pow2(level: u8) -> u64 {
    1u64.checked_shl(level as u32).unwrap_or(u64::MAX)
}

pub(crate) fn research_cost_for_level(idx: u8, current: u8) -> (u64, u64, u64) {
    let (m, c, d) = research_base_cost(idx);
    let mult = pow2(current);
    (
        m.saturating_mul(mult),
        c.saturating_mul(mult),
        d.saturating_mul(mult),
    )
}

pub(crate) fn research_seconds(next_level: u8, lab_level: u8, igr_network: u8) -> i64 {
    let speed_bonus = 100u64.saturating_add(igr_network as u64 * 10);
    let effective_lab = (lab_level.max(1) as u64).saturating_mul(speed_bonus) / 100;
    ((next_level as u64 * 1800) / effective_lab.max(1)).max(1) as i64
}

pub(crate) fn ship_build_seconds(ship_type: u8, quantity: u32, shipyard: u8, nanite: u8) -> i64 {
    const SHIP_BASE_COST_FOR_FIVE_MINUTES: u64 = 1_000;
    const MIN_BUILD_SECONDS: u64 = 300;
    let (m, c, d) = ship_cost(ship_type);
    let total = m
        .saturating_add(c)
        .saturating_add(d)
        .saturating_mul(quantity as u64);

    let speed = (shipyard.max(1) as u64)
        .saturating_mul(2u64.pow(nanite as u32))
        .max(1);

    total
        .saturating_mul(MIN_BUILD_SECONDS)
        .saturating_div(SHIP_BASE_COST_FOR_FIVE_MINUTES.saturating_mul(speed))
        .max(MIN_BUILD_SECONDS) as i64
}

pub(crate) fn defense_cost(defense_type: u8) -> (u64, u64, u64) {
    match defense_type {
        0 => (2000, 0, 0),
        1 => (1500, 500, 0),
        2 => (6000, 2000, 0),
        3 => (20000, 15000, 2000),
        4 => (2000, 6000, 0),
        5 => (50000, 50000, 30000),
        6 => (10000, 10000, 0),
        7 => (50000, 50000, 0),
        8 => (8000, 0, 2000),
        9 => (12500, 2500, 10000),
        _ => (0, 0, 0),
    }
}

pub(crate) fn defense_build_seconds(
    defense_type: u8,
    quantity: u32,
    shipyard: u8,
    nanite: u8,
) -> i64 {
    const DEFENSE_BASE_COST_FOR_FIVE_MINUTES: u64 = 2_000;
    const MIN_BUILD_SECONDS: u64 = 300;
    let (m, c, d) = defense_cost(defense_type);
    let total = m
        .saturating_add(c)
        .saturating_add(d)
        .saturating_mul(quantity as u64);
    let speed = (shipyard.max(1) as u64)
        .saturating_mul(2u64.pow(nanite as u32))
        .max(1);
    total
        .saturating_mul(MIN_BUILD_SECONDS)
        .saturating_div(DEFENSE_BASE_COST_FOR_FIVE_MINUTES.saturating_mul(speed))
        .max(MIN_BUILD_SECONDS) as i64
}

pub(crate) fn ship_cost(ship_type: u8) -> (u64, u64, u64) {
    match ship_type {
        0 => (2000, 2000, 0),
        1 => (6000, 6000, 0),
        2 => (3000, 1000, 0),
        3 => (6000, 4000, 0),
        4 => (20000, 7000, 2000),
        5 => (45000, 15000, 0),
        6 => (30000, 40000, 15000),
        7 => (50000, 25000, 15000),
        8 => (60000, 50000, 15000),
        9 => (5000000, 4000000, 1000000),
        10 => (10000, 6000, 2000),
        11 => (0, 1000, 0),
        12 => (10000, 20000, 10000),
        13 => (0, 2000, 500),
        _ => (0, 0, 0),
    }
}

pub(crate) fn enforce_ship_research_gate(ship_type: u8, planet: &PlanetState) -> Result<()> {
    match ship_type {
        0 => require!(
            planet.shipyard >= 2 && planet.combustion_drive >= 2,
            GameStateError::TechLocked
        ),
        1 => require!(
            planet.shipyard >= 4 && planet.combustion_drive >= 6,
            GameStateError::TechLocked
        ),
        2 => require!(planet.shipyard >= 1, GameStateError::TechLocked),
        3 => require!(
            planet.shipyard >= 3 && planet.armor_technology >= 2 && planet.impulse_drive >= 2,
            GameStateError::TechLocked
        ),
        4 => require!(
            planet.shipyard >= 5 && planet.impulse_drive >= 4,
            GameStateError::TechLocked
        ),
        5 => require!(
            planet.shipyard >= 7 && planet.hyperspace_drive >= 4,
            GameStateError::TechLocked
        ),
        6 => require!(
            planet.shipyard >= 8
                && planet.hyperspace_drive >= 5
                && planet.computer_tech >= 5
                && planet.weapons_technology >= 5,
            GameStateError::TechLocked
        ),
        7 => require!(
            planet.shipyard >= 8
                && planet.impulse_drive >= 6
                && planet.hyperspace_drive >= 5
                && planet.weapons_technology >= 5,
            GameStateError::TechLocked
        ),
        8 => require!(
            planet.shipyard >= 9 && planet.hyperspace_drive >= 6 && planet.armor_technology >= 6,
            GameStateError::TechLocked
        ),
        9 => require!(
            planet.shipyard >= 12
                && planet.hyperspace_drive >= 7
                && planet.weapons_technology >= 10
                && planet.energy_tech >= 12,
            GameStateError::TechLocked
        ),
        10 => require!(
            planet.shipyard >= 4
                && planet.combustion_drive >= 6
                && planet.shielding_technology >= 2,
            GameStateError::TechLocked
        ),
        11 => require!(
            planet.shipyard >= 3 && planet.combustion_drive >= 3,
            GameStateError::TechLocked
        ),
        12 => require!(
            planet.shipyard >= 4 && planet.impulse_drive >= 3 && planet.astrophysics >= 4,
            GameStateError::TechLocked
        ),
        13 => require!(planet.shipyard >= 1, GameStateError::TechLocked),
        _ => return err!(GameStateError::InvalidShipType),
    }
    Ok(())
}

pub(crate) fn enforce_building_requirements(building_idx: u8, planet: &PlanetState) -> Result<()> {
    match building_idx {
        4 => require!(
            planet.deuterium_synthesizer >= 5 && planet.energy_tech >= 3,
            GameStateError::TechLocked
        ),
        6 => require!(
            planet.robotics_factory >= 10 && planet.computer_tech >= 10,
            GameStateError::TechLocked
        ),
        7 => require!(planet.robotics_factory >= 2, GameStateError::TechLocked),
        12 => require!(planet.shipyard >= 1, GameStateError::TechLocked),
        _ => {}
    }
    Ok(())
}

pub(crate) fn enforce_research_requirements(tech_idx: u8, planet: &PlanetState) -> Result<()> {
    match tech_idx {
        0 => require!(planet.research_lab >= 1, GameStateError::TechLocked),
        1 => require!(
            planet.research_lab >= 1 && planet.energy_tech >= 1,
            GameStateError::TechLocked
        ),
        2 => require!(
            planet.research_lab >= 2 && planet.energy_tech >= 1,
            GameStateError::TechLocked
        ),
        3 => require!(planet.research_lab >= 7, GameStateError::TechLocked),
        4 => require!(planet.research_lab >= 1, GameStateError::TechLocked),
        5 => require!(
            planet.research_lab >= 3 && planet.impulse_drive >= 3,
            GameStateError::TechLocked
        ),
        6 => require!(
            planet.research_lab >= 10 && planet.computer_tech >= 8,
            GameStateError::TechLocked
        ),
        7 => require!(planet.research_lab >= 4, GameStateError::TechLocked),
        8 => require!(
            planet.research_lab >= 6 && planet.energy_tech >= 3,
            GameStateError::TechLocked
        ),
        9 => require!(planet.research_lab >= 2, GameStateError::TechLocked),
        _ => {}
    }
    Ok(())
}

pub(crate) fn enforce_defense_requirements(defense_type: u8, planet: &PlanetState) -> Result<()> {
    require!(planet.shipyard >= 1, GameStateError::ShipyardTooLow);
    match defense_type {
        0 => {}
        1 => require!(planet.shipyard >= 2, GameStateError::TechLocked),
        2 => require!(planet.shipyard >= 4, GameStateError::TechLocked),
        3 => require!(
            planet.shipyard >= 6 && planet.weapons_technology >= 3,
            GameStateError::TechLocked
        ),
        4 => require!(
            planet.shipyard >= 4 && planet.shielding_technology >= 2,
            GameStateError::TechLocked
        ),
        5 => require!(
            planet.shipyard >= 8
                && planet.shielding_technology >= 8
                && planet.weapons_technology >= 10
                && planet.energy_tech >= 8,
            GameStateError::TechLocked
        ),
        6 => {
            require!(planet.shielding_technology >= 2, GameStateError::TechLocked);
            require!(planet.small_shield_dome == 0, GameStateError::TechLocked);
        }
        7 => {
            require!(planet.shielding_technology >= 6, GameStateError::TechLocked);
            require!(planet.large_shield_dome == 0, GameStateError::TechLocked);
        }
        8 | 9 => return err!(GameStateError::InvalidDefenseType),
        _ => return err!(GameStateError::InvalidDefenseType),
    }
    Ok(())
}

pub(crate) fn cargo_capacity(sc: u32, lc: u32, rec: u32, cr: u32, bs: u32) -> u64 {
    sc as u64 * 5_000
        + lc as u64 * 25_000
        + rec as u64 * 20_000
        + cr as u64 * 800
        + bs as u64 * 1_500
}

pub(crate) fn fleet_combat_points(
    lf: u32,
    hf: u32,
    cr: u32,
    bs: u32,
    bc: u32,
    bm: u32,
    ds: u32,
    de: u32,
    sc: u32,
    lc: u32,
    rec: u32,
    ep: u32,
    col: u32,
) -> u64 {
    lf as u64 * 50
        + hf as u64 * 150
        + cr as u64 * 400
        + bs as u64 * 1_000
        + bc as u64 * 700
        + bm as u64 * 1_000
        + ds as u64 * 2_000
        + de as u64 * 200_000
        + sc as u64 * 5
        + lc as u64 * 5
        + rec as u64
        + ep as u64
        + col as u64 * 50
}

pub(crate) fn launch_fuel_cost(
    lf: u32,
    hf: u32,
    cr: u32,
    bs: u32,
    bc: u32,
    bm: u32,
    ds: u32,
    _de: u32,
    sc: u32,
    lc: u32,
    rec: u32,
    ep: u32,
    col: u32,
    speed_factor: u8,
) -> u64 {
    (sc as u64 * 10
        + lc as u64 * 50
        + lf as u64 * 20
        + hf as u64 * 75
        + cr as u64 * 300
        + bs as u64 * 500
        + bc as u64 * 250
        + bm as u64 * 1_000
        + ds as u64 * 1_000
        + rec as u64 * 300
        + ep as u64
        + col as u64 * 1_000)
        * (speed_factor as u64).pow(2)
        / 10_000
}

pub(crate) fn mine_rate(level: u8, base: u64) -> u64 {
    if level == 0 {
        return 0;
    }
    base * (level as u64) * 11u64.pow(level as u32) / 10u64.pow(level as u32)
}

pub(crate) fn store_cap(level: u8) -> u64 {
    if level == 0 {
        BASE_STORAGE_CAP
    } else {
        BASE_STORAGE_CAP * 2u64.pow(level as u32)
    }
}

pub(crate) fn chain_now() -> Result<i64> {
    Ok(Clock::get()?.unix_timestamp)
}

pub(crate) fn settle_resources(planet: &mut PlanetState, now: i64) -> Result<()> {
    require!(now >= 0, GameStateError::InvalidTimestamp);

    if planet.last_update_ts <= 0 {
        planet.last_update_ts = now;
        return Ok(());
    }

    require!(
        now >= planet.last_update_ts,
        GameStateError::InvalidTimestamp
    );

    if now == planet.last_update_ts {
        return Ok(());
    }

    let dt = (now - planet.last_update_ts).min(MAX_RESOURCE_SETTLEMENT_SECONDS) as u64;
    let eff_num = if planet.energy_consumption == 0 {
        10_000u64
    } else {
        (planet.energy_production * 10_000 / planet.energy_consumption).min(10_000)
    };

    let add_res = |current: u64, rate_per_hour: u64, cap: u64| -> u64 {
        let produced = rate_per_hour.saturating_mul(dt).saturating_mul(eff_num) / 3600 / 10_000;
        current.saturating_add(produced).min(cap)
    };

    planet.metal = add_res(planet.metal, planet.metal_hour, planet.metal_cap);
    planet.crystal = add_res(planet.crystal, planet.crystal_hour, planet.crystal_cap);
    planet.deuterium = add_res(
        planet.deuterium,
        planet.deuterium_hour,
        planet.deuterium_cap,
    );
    planet.last_update_ts = now;
    Ok(())
}

pub(crate) fn research_flight_bonus_pct(
    from_galaxy: u16,
    from_system: u16,
    to_galaxy: u16,
    to_system: u16,
    planet: &PlanetState,
) -> u64 {
    let mut bonus = 100u64;

    if from_galaxy == to_galaxy && from_system == to_system {
        if planet.astrophysics >= 1 {
            bonus = bonus.saturating_add(planet.astrophysics as u64 * 2);
        }

        return bonus;
    }

    if from_galaxy == to_galaxy {
        if planet.astrophysics >= 1 {
            bonus = bonus.saturating_add(planet.astrophysics as u64 * 3);
        }

        return bonus;
    }

    if planet.astrophysics >= 4 {
        bonus = bonus.saturating_add(planet.astrophysics as u64 * 10);
    }

    bonus
}

pub(crate) fn ship_speed(ship_type: u8, planet: &PlanetState) -> u64 {
    match ship_type {
        0 => 5_000u64.saturating_mul(100 + planet.combustion_drive as u64 * 10) / 100,
        1 => 7_500u64.saturating_mul(100 + planet.combustion_drive as u64 * 10) / 100,
        2 => 12_500u64.saturating_mul(100 + planet.combustion_drive as u64 * 10) / 100,
        3 => 10_000u64.saturating_mul(100 + planet.impulse_drive as u64 * 20) / 100,
        4 => 15_000u64.saturating_mul(100 + planet.impulse_drive as u64 * 20) / 100,
        5 => 10_000u64.saturating_mul(100 + planet.hyperspace_drive as u64 * 30) / 100,
        6 => 10_000u64.saturating_mul(100 + planet.hyperspace_drive as u64 * 30) / 100,
        7 => 4_000u64.saturating_mul(100 + planet.impulse_drive as u64 * 20) / 100,
        8 => 5_000u64.saturating_mul(100 + planet.hyperspace_drive as u64 * 30) / 100,
        9 => 100u64.saturating_mul(100 + planet.hyperspace_drive as u64 * 30) / 100,
        10 => 2_000u64.saturating_mul(100 + planet.combustion_drive as u64 * 10) / 100,
        11 => 100_000_000u64.saturating_mul(100 + planet.combustion_drive as u64 * 10) / 100,
        12 => 2_500u64.saturating_mul(100 + planet.impulse_drive as u64 * 20) / 100,
        _ => 0,
    }
}

pub(crate) fn slowest_fleet_speed(
    planet: &PlanetState,
    lf: u32,
    hf: u32,
    cr: u32,
    bs: u32,
    bc: u32,
    bm: u32,
    ds: u32,
    de: u32,
    sc: u32,
    lc: u32,
    rec: u32,
    ep: u32,
    col: u32,
) -> u64 {
    let mut speed = u64::MAX;

    if sc > 0 {
        speed = speed.min(ship_speed(0, planet));
    }
    if lc > 0 {
        speed = speed.min(ship_speed(1, planet));
    }
    if lf > 0 {
        speed = speed.min(ship_speed(2, planet));
    }
    if hf > 0 {
        speed = speed.min(ship_speed(3, planet));
    }
    if cr > 0 {
        speed = speed.min(ship_speed(4, planet));
    }
    if bs > 0 {
        speed = speed.min(ship_speed(5, planet));
    }
    if bc > 0 {
        speed = speed.min(ship_speed(6, planet));
    }
    if bm > 0 {
        speed = speed.min(ship_speed(7, planet));
    }
    if ds > 0 {
        speed = speed.min(ship_speed(8, planet));
    }
    if de > 0 {
        speed = speed.min(ship_speed(9, planet));
    }
    if rec > 0 {
        speed = speed.min(ship_speed(10, planet));
    }
    if ep > 0 {
        speed = speed.min(ship_speed(11, planet));
    }
    if col > 0 {
        speed = speed.min(ship_speed(12, planet));
    }

    if speed == u64::MAX {
        0
    } else {
        speed.max(1)
    }
}

pub(crate) fn recalculate_rates(planet: &mut PlanetState) {
    planet.metal_hour = mine_rate(planet.metal_mine, 30);
    planet.crystal_hour = mine_rate(planet.crystal_mine, 20);

    let temp_factor = (240i32 - planet.temperature as i32).max(0) as u64;
    planet.deuterium_hour = if planet.deuterium_synthesizer == 0 {
        0
    } else {
        mine_rate(planet.deuterium_synthesizer, 10) * temp_factor / 200
    };

    let solar_prod = mine_rate(planet.solar_plant, 20);
    let satellite_prod =
        solar_satellite_energy(planet.temperature).saturating_mul(planet.solar_satellite as u64);
    let fusion_prod = if planet.fusion_reactor == 0 {
        0
    } else {
        let base = mine_rate(planet.fusion_reactor, 30) * 180 / 100;
        base.saturating_mul(100 + planet.energy_tech as u64 * 10) / 100
    };

    planet.energy_production = solar_prod
        .saturating_add(satellite_prod)
        .saturating_add(fusion_prod);
    planet.energy_consumption = mine_rate(planet.metal_mine, 10)
        + mine_rate(planet.crystal_mine, 10)
        + mine_rate(planet.deuterium_synthesizer, 20);

    planet.metal_cap = store_cap(planet.metal_storage);
    planet.crystal_cap = store_cap(planet.crystal_storage);
    planet.deuterium_cap = store_cap(planet.deuterium_tank);
}

fn solar_satellite_energy(temperature: i16) -> u64 {
    ((temperature as i32 + 160).max(6) as u64 / 6).max(1)
}

pub(crate) fn require_active_vault(
    vault_signer: Pubkey,
    authorized_vault: &AuthorizedVault,
    planet_authority: Pubkey,
) -> Result<()> {
    require_keys_eq!(
        authorized_vault.vault,
        vault_signer,
        GameStateError::InvalidVaultAuthorization
    );
    require_keys_eq!(
        authorized_vault.authority,
        planet_authority,
        GameStateError::InvalidVaultAuthorization
    );
    require!(
        !authorized_vault.revoked,
        GameStateError::VaultAuthorizationRevoked
    );

    if authorized_vault.expires_at > 0 {
        require!(
            Clock::get()?.unix_timestamp <= authorized_vault.expires_at,
            GameStateError::VaultAuthorizationExpired
        );
    }
    Ok(())
}

pub(crate) fn require_market_authority(market_authority: &Signer<'_>) -> Result<()> {
    let (expected_pda, _) =
        Pubkey::find_program_address(&[b"market_authority"], &MARKET_PROGRAM_ID);
    require_keys_eq!(
        market_authority.key(),
        expected_pda,
        GameStateError::UnauthorizedMarket
    );
    Ok(())
}

pub(crate) fn create_planet_state<'info>(
    authority: Pubkey,
    player_profile: &mut Account<'info, PlayerProfile>,
    planet_state: &mut Account<'info, PlanetState>,
    planet_coords_info: &AccountInfo<'info>,
    payer_info: &AccountInfo<'info>,
    system_program_info: &AccountInfo<'info>,
    bump: u8,
    params: &InitializePlanetParams,
) -> Result<()> {
    validate_coordinates(params.galaxy, params.system, params.position)?;
    require_keys_eq!(
        player_profile.authority,
        authority,
        GameStateError::Unauthorized
    );

    msg!("create_planet_state: validated coords");

    let galaxy_bytes = params.galaxy.to_le_bytes();
    let system_bytes = params.system.to_le_bytes();
    let position_bytes = [params.position];
    let seeds: &[&[u8]] = &[
        b"planet_coords",
        &galaxy_bytes,
        &system_bytes,
        &position_bytes,
    ];

    let (expected_pda, coords_bump) = Pubkey::find_program_address(seeds, &crate::ID);

    require_keys_eq!(
        planet_coords_info.key(),
        expected_pda,
        GameStateError::InvalidCoordinates
    );

    msg!("create_planet_state: derived coords PDA");

    let planet_index = player_profile.planet_count;
    player_profile.planet_count = player_profile
        .planet_count
        .checked_add(1)
        .ok_or(GameStateError::PlanetCountOverflow)?;

    msg!("create_planet_state: incremented planet_count");

    let rent = Rent::get()?;
    let space = PLANET_COORDS_SPACE;
    let lamports = rent.minimum_balance(space);

    let signer_seeds: &[&[&[u8]]] = &[&[
        b"planet_coords",
        &galaxy_bytes,
        &system_bytes,
        &position_bytes,
        &[coords_bump],
    ]];

    anchor_lang::system_program::create_account(
        CpiContext::new_with_signer(
            system_program_info.clone(),
            anchor_lang::system_program::CreateAccount {
                from: payer_info.clone(),
                to: planet_coords_info.clone(),
            },
            signer_seeds,
        ),
        lamports,
        space as u64,
        &crate::ID,
    )?;

    msg!("create_planet_state: created coords account");

    let coords_data = PlanetCoordinates {
        galaxy: params.galaxy,
        system: params.system,
        position: params.position,
        planet: planet_state.key(),
        authority,
        debris_metal: 0,
        debris_crystal: 0,
        bump: coords_bump,
    };

    let mut encoded = Vec::with_capacity(PLANET_COORDS_SPACE);
    let disc = <PlanetCoordinates as anchor_lang::Discriminator>::DISCRIMINATOR;
    encoded.extend_from_slice(&disc);
    coords_data.serialize(&mut encoded)?;
    require!(
        encoded.len() <= PLANET_COORDS_SPACE,
        GameStateError::InvalidArgs
    );
    let mut data = planet_coords_info.try_borrow_mut_data()?;
    data[..encoded.len()].copy_from_slice(&encoded);

    msg!("create_planet_state: wrote coords data");
    msg!("create_planet_state: about to write planet_state fields");

    // IMPORTANT:
    // Write directly into the account fields instead of constructing one huge
    // PlanetState value on the stack with set_inner(...).
    let planet = &mut **planet_state;

    planet.authority = authority;
    planet.player = player_profile.key();
    planet.planet_index = planet_index;

    planet.galaxy = params.galaxy;
    planet.system = params.system;
    planet.position = params.position;
    planet.name = copy_name::<MAX_PLANET_NAME_LEN>(&params.name, "Planet");

    planet.diameter = params.diameter;
    planet.temperature = params.temperature;
    planet.max_fields = params.max_fields;
    planet.used_fields = params.used_fields;

    planet.metal_mine = params.metal_mine;
    planet.crystal_mine = params.crystal_mine;
    planet.deuterium_synthesizer = params.deuterium_synthesizer;
    planet.solar_plant = params.solar_plant;
    planet.fusion_reactor = params.fusion_reactor;
    planet.robotics_factory = params.robotics_factory;
    planet.nanite_factory = params.nanite_factory;
    planet.shipyard = params.shipyard;
    planet.metal_storage = params.metal_storage;
    planet.crystal_storage = params.crystal_storage;
    planet.deuterium_tank = params.deuterium_tank;
    planet.research_lab = params.research_lab;
    planet.missile_silo = params.missile_silo;

    planet.energy_tech = params.energy_tech;
    planet.combustion_drive = params.combustion_drive;
    planet.impulse_drive = params.impulse_drive;
    planet.hyperspace_drive = params.hyperspace_drive;
    planet.computer_tech = params.computer_tech;
    planet.astrophysics = params.astrophysics;
    planet.igr_network = params.igr_network;
    planet.weapons_technology = params.weapons_technology;
    planet.shielding_technology = params.shielding_technology;
    planet.armor_technology = params.armor_technology;

    planet.research_queue_item = params.research_queue_item;
    planet.research_queue_target = params.research_queue_target;
    planet.research_finish_ts = params.research_finish_ts;

    planet.build_queue_item = params.build_queue_item;
    planet.build_queue_target = params.build_queue_target;
    planet.build_finish_ts = params.build_finish_ts;

    planet.metal = params.metal;
    planet.crystal = params.crystal;
    planet.deuterium = params.deuterium;

    planet.metal_hour = params.metal_hour;
    planet.crystal_hour = params.crystal_hour;
    planet.deuterium_hour = params.deuterium_hour;
    planet.energy_production = params.energy_production;
    planet.energy_consumption = params.energy_consumption;

    planet.metal_cap = params.metal_cap;
    planet.crystal_cap = params.crystal_cap;
    planet.deuterium_cap = params.deuterium_cap;
    planet.last_update_ts = params.last_update_ts;
    planet.created_at = params.created_at;
    planet.protection_until_ts = params.protection_until_ts;
    planet.market_unlocked_at = params.market_unlocked_at;
    planet.attack_unlocked_at = params.attack_unlocked_at;
    planet.last_attack_launch_ts = params.last_attack_launch_ts;
    planet.last_attacked_ts = params.last_attacked_ts;

    planet.small_cargo = params.small_cargo;
    planet.large_cargo = params.large_cargo;
    planet.light_fighter = params.light_fighter;
    planet.heavy_fighter = params.heavy_fighter;
    planet.cruiser = params.cruiser;
    planet.battleship = params.battleship;
    planet.battlecruiser = params.battlecruiser;
    planet.bomber = params.bomber;
    planet.destroyer = params.destroyer;
    planet.deathstar = params.deathstar;
    planet.recycler = params.recycler;
    planet.espionage_probe = params.espionage_probe;
    planet.colony_ship = params.colony_ship;
    planet.solar_satellite = params.solar_satellite;
    planet.rocket_launcher = params.rocket_launcher;
    planet.light_laser = params.light_laser;
    planet.heavy_laser = params.heavy_laser;
    planet.gauss_cannon = params.gauss_cannon;
    planet.ion_cannon = params.ion_cannon;
    planet.plasma_turret = params.plasma_turret;
    planet.small_shield_dome = params.small_shield_dome;
    planet.large_shield_dome = params.large_shield_dome;
    planet.anti_ballistic_missile = params.anti_ballistic_missile;
    planet.interplanetary_missile = params.interplanetary_missile;

    planet.active_missions = 0;

    for i in 0..MAX_MISSIONS {
        planet.missions[i] = MissionState::default();
    }

    planet.bump = bump;
    planet.ship_build_item = params.ship_build_item;
    planet.ship_build_qty = params.ship_build_qty;
    planet.ship_build_finish_ts = params.ship_build_finish_ts;
    planet.defense_build_item = params.defense_build_item;
    planet.defense_build_qty = params.defense_build_qty;
    planet.defense_build_finish_ts = params.defense_build_finish_ts;

    msg!("create_planet_state: finished writing planet_state fields");

    Ok(())
}

pub(crate) fn create_public_planet_state<'info>(
    authority: Pubkey,
    player_profile: &mut Account<'info, PlayerProfile>,
    public_planet_state: &mut Account<'info, PublicPlanetState>,
    public_planet_coords_info: &AccountInfo<'info>,
    payer_info: &AccountInfo<'info>,
    system_program_info: &AccountInfo<'info>,
    bump: u8,
    name: &str,
    galaxy: u16,
    system: u16,
    position: u8,
    created_at: i64,
) -> Result<()> {
    validate_coordinates(galaxy, system, position)?;
    require_keys_eq!(
        player_profile.authority,
        authority,
        GameStateError::Unauthorized
    );

    let galaxy_bytes = galaxy.to_le_bytes();
    let system_bytes = system.to_le_bytes();
    let position_bytes = [position];
    let seeds: &[&[u8]] = &[
        b"public_planet_coords",
        &galaxy_bytes,
        &system_bytes,
        &position_bytes,
    ];
    let (expected_pda, coords_bump) = Pubkey::find_program_address(seeds, &crate::ID);

    require_keys_eq!(
        public_planet_coords_info.key(),
        expected_pda,
        GameStateError::InvalidCoordinates
    );

    let planet_index = player_profile.planet_count;
    player_profile.planet_count = player_profile
        .planet_count
        .checked_add(1)
        .ok_or(GameStateError::PlanetCountOverflow)?;

    let rent = Rent::get()?;
    let space = PUBLIC_PLANET_COORDS_SPACE;
    let lamports = rent.minimum_balance(space);
    let signer_seeds: &[&[&[u8]]] = &[&[
        b"public_planet_coords",
        &galaxy_bytes,
        &system_bytes,
        &position_bytes,
        &[coords_bump],
    ]];

    anchor_lang::system_program::create_account(
        CpiContext::new_with_signer(
            system_program_info.clone(),
            anchor_lang::system_program::CreateAccount {
                from: payer_info.clone(),
                to: public_planet_coords_info.clone(),
            },
            signer_seeds,
        ),
        lamports,
        space as u64,
        &crate::ID,
    )?;

    let coords_data = PublicPlanetCoordinates {
        galaxy,
        system,
        position,
        public_planet: public_planet_state.key(),
        authority,
        bump: coords_bump,
    };
    let mut encoded = Vec::with_capacity(PUBLIC_PLANET_COORDS_SPACE);
    let disc = <PublicPlanetCoordinates as anchor_lang::Discriminator>::DISCRIMINATOR;
    encoded.extend_from_slice(&disc);
    coords_data.serialize(&mut encoded)?;
    require!(
        encoded.len() <= PUBLIC_PLANET_COORDS_SPACE,
        GameStateError::InvalidArgs
    );
    let mut data = public_planet_coords_info.try_borrow_mut_data()?;
    data[..encoded.len()].copy_from_slice(&encoded);

    public_planet_state.authority = authority;
    public_planet_state.player = player_profile.key();
    public_planet_state.planet_index = planet_index;
    public_planet_state.galaxy = galaxy;
    public_planet_state.system = system;
    public_planet_state.position = position;
    public_planet_state.version = 2;
    public_planet_state.name = copy_name::<MAX_PLANET_NAME_LEN>(name, "Planet");
    public_planet_state.created_at = created_at;
    public_planet_state.bump = bump;

    Ok(())
}

pub(crate) fn produce_planet(planet: &mut PlanetState, now: i64) -> Result<()> {
    settle_resources(planet, now)?;
    Ok(())
}

pub(crate) fn finish_build_now(planet: &mut PlanetState, now: i64) -> Result<()> {
    settle_resources(planet, now)?;
    require!(planet.build_finish_ts > 0, GameStateError::NoBuild);

    let idx = planet.build_queue_item;
    let level = planet.build_queue_target;
    planet.set_building_level(idx, level);
    recalculate_rates(planet);

    planet.build_queue_item = 255;
    planet.build_queue_target = 0;
    planet.build_finish_ts = 0;
    Ok(())
}

pub(crate) fn start_build_planet(
    planet: &mut PlanetState,
    building_idx: u8,
    now: i64,
) -> Result<()> {
    settle_resources(planet, now)?;
    let current = planet.building_level(building_idx);
    let next = current.saturating_add(1);
    let (cm, cc, cd) = upgrade_cost(building_idx, next as u64);

    require!(
        planet.build_finish_ts == 0 || now >= planet.build_finish_ts,
        GameStateError::QueueBusy
    );
    require!(
        planet.used_fields < planet.max_fields,
        GameStateError::NoFields
    );
    enforce_building_requirements(building_idx, planet)?;
    require!(planet.metal >= cm, GameStateError::InsufficientMetal);
    require!(planet.crystal >= cc, GameStateError::InsufficientCrystal);
    require!(
        planet.deuterium >= cd,
        GameStateError::InsufficientDeuterium
    );

    planet.metal -= cm;
    planet.crystal -= cc;
    planet.deuterium -= cd;

    let dur = build_seconds(building_idx, next as u64, planet.robotics_factory as u64);
    planet.build_queue_item = building_idx;
    planet.build_queue_target = next;
    planet.build_finish_ts = now + dur;
    planet.used_fields = planet.used_fields.saturating_add(1);
    Ok(())
}

pub(crate) fn finish_build_planet(planet: &mut PlanetState, now: i64) -> Result<()> {
    require!(
        now >= planet.build_finish_ts,
        GameStateError::BuildNotFinished
    );
    finish_build_now(planet, now)
}

pub(crate) fn start_research_planet(
    planet: &mut PlanetState,
    tech_idx: u8,
    now: i64,
) -> Result<()> {
    settle_resources(planet, now)?;
    require!(tech_idx <= 9, GameStateError::InvalidTech);
    require!(planet.research_lab >= 1, GameStateError::LabTooLow);
    require!(
        !(planet.build_queue_item == 11 && planet.build_finish_ts > 0),
        GameStateError::ResearchQueueBusy
    );
    require!(
        planet.research_queue_item == 255,
        GameStateError::ResearchQueueBusy
    );

    let lab_req = research_lab_requirement(tech_idx);
    require!(planet.research_lab >= lab_req, GameStateError::LabTooLow);
    enforce_research_requirements(tech_idx, planet)?;

    let current = planet.research_level(tech_idx);
    let next = current.saturating_add(1);
    let (cm, cc, cd) = research_cost_for_level(tech_idx, current);

    require!(planet.metal >= cm, GameStateError::InsufficientMetal);
    require!(planet.crystal >= cc, GameStateError::InsufficientCrystal);
    require!(
        planet.deuterium >= cd,
        GameStateError::InsufficientDeuterium
    );

    planet.metal -= cm;
    planet.crystal -= cc;
    planet.deuterium -= cd;

    planet.research_queue_item = tech_idx;
    planet.research_queue_target = next;
    planet.research_finish_ts =
        now + research_seconds(next, planet.research_lab, planet.igr_network);
    Ok(())
}

pub(crate) fn build_defense_planet(
    planet: &mut PlanetState,
    defense_type: u8,
    quantity: u32,
    now: i64,
) -> Result<()> {
    require!(quantity > 0, GameStateError::InvalidArgs);
    require!(
        defense_type != 6 && defense_type != 7 || quantity == 1,
        GameStateError::InvalidDefenseType
    );
    settle_resources(planet, now)?;
    require!(
        !(planet.build_queue_item == 7 && planet.build_finish_ts > 0),
        GameStateError::ShipyardQueueBusy
    );
    require!(
        planet.ship_build_item == 255,
        GameStateError::ShipyardQueueBusy
    );
    require!(
        planet.defense_build_item == 255,
        GameStateError::ShipyardQueueBusy
    );

    enforce_defense_requirements(defense_type, planet)?;

    let (cm, cc, cd) = defense_cost(defense_type);
    require!(
        cm != 0 || cc != 0 || cd != 0,
        GameStateError::InvalidDefenseType
    );

    let total_m = cm.saturating_mul(quantity as u64);
    let total_c = cc.saturating_mul(quantity as u64);
    let total_d = cd.saturating_mul(quantity as u64);

    require!(planet.metal >= total_m, GameStateError::InsufficientMetal);
    require!(
        planet.crystal >= total_c,
        GameStateError::InsufficientCrystal
    );
    require!(
        planet.deuterium >= total_d,
        GameStateError::InsufficientDeuterium
    );

    planet.metal -= total_m;
    planet.crystal -= total_c;
    planet.deuterium -= total_d;

    planet.defense_build_item = defense_type;
    planet.defense_build_qty = quantity;
    planet.defense_build_finish_ts = now
        + defense_build_seconds(
            defense_type,
            quantity,
            planet.shipyard,
            planet.nanite_factory,
        );
    Ok(())
}

pub(crate) fn finish_defense_build_planet(planet: &mut PlanetState, now: i64) -> Result<()> {
    require!(
        now >= planet.defense_build_finish_ts,
        GameStateError::DefenseBuildNotFinished
    );
    settle_resources(planet, now)?;
    require!(
        planet.defense_build_item != 255,
        GameStateError::NoDefenseBuild
    );
    require!(
        planet.defense_build_finish_ts > 0,
        GameStateError::NoDefenseBuild
    );

    let defense_type = planet.defense_build_item;
    let quantity = planet.defense_build_qty;
    planet.add_defense(defense_type, quantity)?;

    planet.defense_build_item = 255;
    planet.defense_build_qty = 0;
    planet.defense_build_finish_ts = 0;
    Ok(())
}

pub(crate) fn finish_research_now(planet: &mut PlanetState, now: i64) -> Result<()> {
    settle_resources(planet, now)?;
    require!(
        planet.research_queue_item != 255,
        GameStateError::NoResearch
    );

    let idx = planet.research_queue_item;
    let target = planet.research_queue_target;
    planet.set_research_level(idx, target);
    recalculate_rates(planet);

    planet.research_queue_item = 255;
    planet.research_queue_target = 0;
    planet.research_finish_ts = 0;
    Ok(())
}

pub(crate) fn finish_research_planet(planet: &mut PlanetState, now: i64) -> Result<()> {
    require!(
        now >= planet.research_finish_ts,
        GameStateError::ResearchNotFinished
    );
    finish_research_now(planet, now)
}

pub(crate) fn distance(
    from_galaxy: u16,
    from_system: u16,
    from_position: u8,
    to_galaxy: u16,
    to_system: u16,
    to_position: u8,
) -> u64 {
    let galaxy_jump = (from_galaxy as i64 - to_galaxy as i64).abs() as u64 * 20_000;
    let system_jump = (from_system as i64 - to_system as i64).abs() as u64 * 2_000;
    let position_jump = (from_position as i64 - to_position as i64).abs() as u64 * 200;

    galaxy_jump
        .saturating_add(system_jump)
        .saturating_add(position_jump)
        .saturating_add(1_000)
}

pub(crate) fn base_flight_seconds(
    from_galaxy: u16,
    from_system: u16,
    from_position: u8,
    to_galaxy: u16,
    to_system: u16,
    to_position: u8,
) -> u64 {
    let galaxy_delta = (from_galaxy as i64 - to_galaxy as i64).abs() as u64;
    let system_delta = (from_system as i64 - to_system as i64).abs() as u64;
    let position_delta = (from_position as i64 - to_position as i64).abs() as u64;

    if galaxy_delta > 0 {
        86_400u64
            .saturating_mul(galaxy_delta)
            .saturating_add(3_600u64.saturating_mul(system_delta))
            .saturating_add(300u64.saturating_mul(position_delta))
            .max(86_400)
    } else if system_delta > 0 {
        3_600u64
            .saturating_mul(system_delta)
            .saturating_add(300u64.saturating_mul(position_delta))
            .max(3_600)
    } else {
        300u64.saturating_mul(position_delta.max(1))
    }
}

pub(crate) fn mission_flight_seconds(
    from_galaxy: u16,
    from_system: u16,
    from_position: u8,
    to_galaxy: u16,
    to_system: u16,
    to_position: u8,
    speed_factor: u8,
    fleet_speed: u64,
    planet: &PlanetState,
) -> i64 {
    const REFERENCE_EFFECTIVE_SPEED: u64 = 500_000;
    let sf = speed_factor.clamp(10, 100) as u64;
    let base_seconds = base_flight_seconds(
        from_galaxy,
        from_system,
        from_position,
        to_galaxy,
        to_system,
        to_position,
    );
    let effective_fleet_speed = fleet_speed.max(1);
    let tech_bonus =
        research_flight_bonus_pct(from_galaxy, from_system, to_galaxy, to_system, planet);
    let effective_speed = effective_fleet_speed
        .saturating_mul(sf)
        .saturating_mul(tech_bonus.max(100))
        / 100;

    base_seconds
        .saturating_mul(REFERENCE_EFFECTIVE_SPEED)
        .checked_div(effective_speed.max(1))
        .unwrap_or(1)
        .max(1) as i64
}

pub(crate) fn build_ship_planet(
    planet: &mut PlanetState,
    ship_type: u8,
    quantity: u32,
    now: i64,
) -> Result<()> {
    require!(quantity > 0, GameStateError::InvalidArgs);
    settle_resources(planet, now)?;
    require!(planet.shipyard >= 1, GameStateError::ShipyardTooLow);
    require!(
        !(planet.build_queue_item == 7 && planet.build_finish_ts > 0),
        GameStateError::ShipyardQueueBusy
    );
    require!(
        planet.ship_build_item == 255,
        GameStateError::ShipyardQueueBusy
    );
    require!(
        planet.defense_build_item == 255,
        GameStateError::ShipyardQueueBusy
    );

    enforce_ship_research_gate(ship_type, planet)?;

    let (cm, cc, cd) = ship_cost(ship_type);
    require!(
        cm != 0 || cc != 0 || cd != 0 || ship_type == 11,
        GameStateError::InvalidShipType
    );

    let total_m = cm.saturating_mul(quantity as u64);
    let total_c = cc.saturating_mul(quantity as u64);
    let total_d = cd.saturating_mul(quantity as u64);

    require!(planet.metal >= total_m, GameStateError::InsufficientMetal);
    require!(
        planet.crystal >= total_c,
        GameStateError::InsufficientCrystal
    );
    require!(
        planet.deuterium >= total_d,
        GameStateError::InsufficientDeuterium
    );

    planet.metal -= total_m;
    planet.crystal -= total_c;
    planet.deuterium -= total_d;

    let dur = ship_build_seconds(ship_type, quantity, planet.shipyard, planet.nanite_factory);

    planet.ship_build_item = ship_type;
    planet.ship_build_qty = quantity;
    planet.ship_build_finish_ts = now + dur;

    Ok(())
}

pub(crate) fn finish_ship_build_planet(planet: &mut PlanetState, now: i64) -> Result<()> {
    require!(
        now >= planet.ship_build_finish_ts,
        GameStateError::ShipBuildNotFinished
    );
    finish_ship_build_now(planet, now)
}

pub(crate) fn finish_ship_build_now(planet: &mut PlanetState, now: i64) -> Result<()> {
    settle_resources(planet, now)?;

    require!(planet.ship_build_item != 255, GameStateError::NoShipBuild);
    require!(planet.ship_build_finish_ts > 0, GameStateError::NoShipBuild);

    let ship_type = planet.ship_build_item;
    let quantity = planet.ship_build_qty;

    planet.add_ship(ship_type, quantity)?;
    if ship_type == 13 {
        recalculate_rates(planet);
    }

    planet.ship_build_item = 255;
    planet.ship_build_qty = 0;
    planet.ship_build_finish_ts = 0;

    Ok(())
}

pub(crate) fn burn_antimatter<'info>(
    antimatter_mint: &Account<'info, Mint>,
    user_antimatter_account: &Account<'info, TokenAccount>,
    authority: &Signer<'info>,
    token_program: &Program<'info, Token>,
    amount: u64,
) -> Result<()> {
    require!(amount > 0, GameStateError::NoAccelerationNeeded);
    require!(
        antimatter_mint.decimals == ANTIMATTER_DECIMALS,
        GameStateError::InvalidAntimatterMintDecimals
    );
    require_keys_eq!(
        user_antimatter_account.owner,
        authority.key(),
        GameStateError::InvalidAntimatterAccount
    );
    require_keys_eq!(
        user_antimatter_account.mint,
        antimatter_mint.key(),
        GameStateError::InvalidAntimatterMint
    );
    require!(
        user_antimatter_account.amount >= amount,
        GameStateError::InsufficientAntimatter
    );

    token::burn(
        CpiContext::new(
            token_program.to_account_info(),
            Burn {
                mint: antimatter_mint.to_account_info(),
                from: user_antimatter_account.to_account_info(),
                authority: authority.to_account_info(),
            },
        ),
        amount,
    )
}

pub(crate) fn transfer_antimatter<'info>(
    antimatter_mint: &Account<'info, Mint>,
    user_antimatter_account: &Account<'info, TokenAccount>,
    treasury_antimatter_account: &Account<'info, TokenAccount>,
    authority: &Signer<'info>,
    token_program: &Program<'info, Token>,
    amount: u64,
) -> Result<()> {
    require!(amount > 0, GameStateError::InvalidArgs);
    require!(
        antimatter_mint.decimals == ANTIMATTER_DECIMALS,
        GameStateError::InvalidAntimatterMint
    );
    require_keys_eq!(
        user_antimatter_account.owner,
        authority.key(),
        GameStateError::InvalidAntimatterAccount
    );
    require_keys_eq!(
        user_antimatter_account.mint,
        antimatter_mint.key(),
        GameStateError::InvalidAntimatterMint
    );
    require_keys_eq!(
        treasury_antimatter_account.mint,
        antimatter_mint.key(),
        GameStateError::InvalidAntimatterMint
    );
    require!(
        user_antimatter_account.amount >= amount,
        GameStateError::InsufficientAntimatter
    );

    token::transfer(
        CpiContext::new(
            token_program.to_account_info(),
            Transfer {
                from: user_antimatter_account.to_account_info(),
                to: treasury_antimatter_account.to_account_info(),
                authority: authority.to_account_info(),
            },
        ),
        amount,
    )
}

pub(crate) fn transfer_usdc<'info>(
    usdc_mint: &Account<'info, Mint>,
    user_usdc_account: &Account<'info, TokenAccount>,
    treasury_usdc_account: &Account<'info, TokenAccount>,
    authority: &Signer<'info>,
    token_program: &Program<'info, Token>,
    amount: u64,
) -> Result<()> {
    require!(amount > 0, GameStateError::InvalidStorePack);
    require!(usdc_mint.decimals == 6, GameStateError::InvalidUsdcMint);
    require_keys_eq!(
        user_usdc_account.owner,
        authority.key(),
        GameStateError::InvalidUsdcAccount
    );
    require_keys_eq!(
        user_usdc_account.mint,
        usdc_mint.key(),
        GameStateError::InvalidUsdcMint
    );
    require_keys_eq!(
        treasury_usdc_account.mint,
        usdc_mint.key(),
        GameStateError::InvalidUsdcMint
    );
    require!(
        user_usdc_account.amount >= amount,
        GameStateError::InsufficientUsdc
    );

    token::transfer(
        CpiContext::new(
            token_program.to_account_info(),
            Transfer {
                from: user_usdc_account.to_account_info(),
                to: treasury_usdc_account.to_account_info(),
                authority: authority.to_account_info(),
            },
        ),
        amount,
    )
}

pub(crate) fn accelerate_mission_with_antimatter_inner<'info>(
    planet: &mut Account<'info, PlanetState>,
    antimatter_mint: &Account<'info, Mint>,
    user_antimatter_account: &Account<'info, TokenAccount>,
    authority: &Signer<'info>,
    token_program: &Program<'info, Token>,
    slot: u8,
    leg: u8,
) -> Result<u64> {
    let now = Clock::get()?.unix_timestamp;
    let slot_idx = slot as usize;
    require!(slot_idx < MAX_MISSIONS, GameStateError::InvalidMissionSlot);

    let mut mission = planet.mission(slot_idx);
    require!(mission.mission_type != 0, GameStateError::InvalidMission);

    let target_ts = match leg {
        0 => {
            require!(!mission.applied, GameStateError::AlreadyResolved);
            mission.arrive_ts
        }
        1 => {
            require!(mission.applied, GameStateError::ReturnInFlight);
            require!(mission.return_ts > 0, GameStateError::ReturnInFlight);
            mission.return_ts
        }
        _ => return Err(GameStateError::InvalidArgs.into()),
    };

    let seconds_left = target_ts.saturating_sub(now);
    require!(seconds_left > 0, GameStateError::NoAccelerationNeeded);
    let amount = (seconds_left as u64)
        .checked_mul(ANTIMATTER_SCALE)
        .ok_or(GameStateError::AntimatterAmountOverflow)?;

    burn_antimatter(
        antimatter_mint,
        user_antimatter_account,
        authority,
        token_program,
        amount,
    )?;

    if leg == 0 {
        mission.arrive_ts = now;
    } else {
        mission.return_ts = now;
    }
    planet.set_mission(slot_idx, mission);
    Ok(amount)
}

pub(crate) fn accelerate_build_with_antimatter_inner<'info>(
    planet: &mut Account<'info, PlanetState>,
    antimatter_mint: &Account<'info, Mint>,
    user_antimatter_account: &Account<'info, TokenAccount>,
    authority: &Signer<'info>,
    token_program: &Program<'info, Token>,
) -> Result<u64> {
    let now = Clock::get()?.unix_timestamp;
    require!(planet.build_finish_ts > 0, GameStateError::NoBuild);
    require!(planet.build_queue_item != 255, GameStateError::NoBuild);

    let seconds_left = planet.build_finish_ts.saturating_sub(now);
    require!(seconds_left > 0, GameStateError::NoAccelerationNeeded);

    let amount = (seconds_left as u64)
        .checked_mul(ANTIMATTER_SCALE)
        .ok_or(GameStateError::AntimatterAmountOverflow)?;
    burn_antimatter(
        antimatter_mint,
        user_antimatter_account,
        authority,
        token_program,
        amount,
    )?;
    finish_build_now(planet, now)?;
    Ok(amount)
}

pub(crate) fn accelerate_research_with_antimatter_inner<'info>(
    planet: &mut Account<'info, PlanetState>,
    antimatter_mint: &Account<'info, Mint>,
    user_antimatter_account: &Account<'info, TokenAccount>,
    authority: &Signer<'info>,
    token_program: &Program<'info, Token>,
) -> Result<u64> {
    let now = Clock::get()?.unix_timestamp;
    require!(
        planet.research_queue_item != 255,
        GameStateError::NoResearch
    );
    require!(planet.research_finish_ts > 0, GameStateError::NoResearch);

    let seconds_left = planet.research_finish_ts.saturating_sub(now);
    require!(seconds_left > 0, GameStateError::NoAccelerationNeeded);

    let amount = (seconds_left as u64)
        .checked_mul(ANTIMATTER_SCALE)
        .ok_or(GameStateError::AntimatterAmountOverflow)?;
    burn_antimatter(
        antimatter_mint,
        user_antimatter_account,
        authority,
        token_program,
        amount,
    )?;
    finish_research_now(planet, now)?;
    Ok(amount)
}

pub(crate) fn accelerate_ship_build_with_antimatter_inner<'info>(
    planet: &mut Account<'info, PlanetState>,
    antimatter_mint: &Account<'info, Mint>,
    user_antimatter_account: &Account<'info, TokenAccount>,
    authority: &Signer<'info>,
    token_program: &Program<'info, Token>,
) -> Result<u64> {
    let now = Clock::get()?.unix_timestamp;
    require!(planet.ship_build_item != 255, GameStateError::NoShipBuild);
    require!(planet.ship_build_finish_ts > 0, GameStateError::NoShipBuild);

    let seconds_left = planet.ship_build_finish_ts.saturating_sub(now);
    require!(seconds_left > 0, GameStateError::NoAccelerationNeeded);

    let amount = (seconds_left as u64)
        .checked_mul(ANTIMATTER_SCALE)
        .ok_or(GameStateError::AntimatterAmountOverflow)?;
    burn_antimatter(
        antimatter_mint,
        user_antimatter_account,
        authority,
        token_program,
        amount,
    )?;
    finish_ship_build_now(planet, now)?;
    Ok(amount)
}

pub(crate) fn accelerate_defense_build_with_antimatter_inner<'info>(
    planet: &mut Account<'info, PlanetState>,
    antimatter_mint: &Account<'info, Mint>,
    user_antimatter_account: &Account<'info, TokenAccount>,
    authority: &Signer<'info>,
    token_program: &Program<'info, Token>,
) -> Result<u64> {
    let now = Clock::get()?.unix_timestamp;
    require!(
        planet.defense_build_item != 255,
        GameStateError::NoDefenseBuild
    );
    require!(
        planet.defense_build_finish_ts > 0,
        GameStateError::NoDefenseBuild
    );

    let seconds_left = planet.defense_build_finish_ts.saturating_sub(now);
    require!(seconds_left > 0, GameStateError::NoAccelerationNeeded);

    let amount = (seconds_left as u64)
        .checked_mul(ANTIMATTER_SCALE)
        .ok_or(GameStateError::AntimatterAmountOverflow)?;
    burn_antimatter(
        antimatter_mint,
        user_antimatter_account,
        authority,
        token_program,
        amount,
    )?;
    planet.defense_build_finish_ts = now;
    finish_defense_build_planet(planet, now)?;
    Ok(amount)
}

pub(crate) fn launch_fleet_planet(
    planet: &mut PlanetState,
    params: LaunchFleetParams,
) -> Result<()> {
    let now = chain_now()?;
    validate_coordinates(
        params.target_galaxy,
        params.target_system,
        params.target_position,
    )?;
    require!(
        params.mission_type == MISSION_ATTACK
            || params.mission_type == MISSION_TRANSPORT
            || params.mission_type == MISSION_COLONIZE
            || params.mission_type == MISSION_ESPIONAGE,
        GameStateError::InvalidMission
    );

    let total_ships = params.light_fighter
        + params.heavy_fighter
        + params.cruiser
        + params.battleship
        + params.battlecruiser
        + params.bomber
        + params.destroyer
        + params.deathstar
        + params.small_cargo
        + params.large_cargo
        + params.recycler
        + params.espionage_probe
        + params.colony_ship;

    require!(total_ships > 0, GameStateError::EmptyFleet);
    if params.mission_type == MISSION_ESPIONAGE {
        require!(params.espionage_probe > 0, GameStateError::EmptyFleet);
        require!(
            params.light_fighter
                + params.heavy_fighter
                + params.cruiser
                + params.battleship
                + params.battlecruiser
                + params.bomber
                + params.destroyer
                + params.deathstar
                + params.small_cargo
                + params.large_cargo
                + params.recycler
                + params.colony_ship
                == 0,
            GameStateError::InvalidMission
        );
        require!(
            params.cargo_metal == 0 && params.cargo_crystal == 0 && params.cargo_deuterium == 0,
            GameStateError::InvalidMission
        );
    }
    if params.mission_type == MISSION_COLONIZE {
        require!(params.colony_ship == 1, GameStateError::MissingColonyShip);
        require!(
            params.light_fighter
                + params.heavy_fighter
                + params.cruiser
                + params.battleship
                + params.battlecruiser
                + params.bomber
                + params.destroyer
                + params.deathstar
                + params.small_cargo
                + params.large_cargo
                + params.recycler
                + params.espionage_probe
                == 0,
            GameStateError::InvalidMission
        );
        require!(
            params.cargo_metal == 0 && params.cargo_crystal == 0 && params.cargo_deuterium == 0,
            GameStateError::InvalidMission
        );
    }

    settle_resources(planet, now)?;
    let slot = planet
        .free_mission_slot()
        .ok_or(GameStateError::NoMissionSlot)?;

    if params.mission_type == MISSION_ATTACK {
        require!(
            now >= planet.attack_unlocked_at,
            GameStateError::GameplayLocked
        );
        require!(
            planet.last_attack_launch_ts == 0
                || now
                    >= planet
                        .last_attack_launch_ts
                        .saturating_add(ATTACK_LAUNCH_COOLDOWN_SECONDS),
            GameStateError::AttackCooldown
        );
        let attack_points = fleet_combat_points(
            params.light_fighter,
            params.heavy_fighter,
            params.cruiser,
            params.battleship,
            params.battlecruiser,
            params.bomber,
            params.destroyer,
            params.deathstar,
            params.small_cargo,
            params.large_cargo,
            params.recycler,
            params.espionage_probe,
            params.colony_ship,
        );
        require!(
            attack_points >= MIN_ATTACK_COMBAT_POINTS,
            GameStateError::AttackPowerTooLow
        );
    }

    require!(
        planet.light_fighter >= params.light_fighter,
        GameStateError::InsufficientShips
    );
    require!(
        planet.heavy_fighter >= params.heavy_fighter,
        GameStateError::InsufficientShips
    );
    require!(
        planet.cruiser >= params.cruiser,
        GameStateError::InsufficientShips
    );
    require!(
        planet.battleship >= params.battleship,
        GameStateError::InsufficientShips
    );
    require!(
        planet.battlecruiser >= params.battlecruiser,
        GameStateError::InsufficientShips
    );
    require!(
        planet.bomber >= params.bomber,
        GameStateError::InsufficientShips
    );
    require!(
        planet.destroyer >= params.destroyer,
        GameStateError::InsufficientShips
    );
    require!(
        planet.deathstar >= params.deathstar,
        GameStateError::InsufficientShips
    );
    require!(
        planet.small_cargo >= params.small_cargo,
        GameStateError::InsufficientShips
    );
    require!(
        planet.large_cargo >= params.large_cargo,
        GameStateError::InsufficientShips
    );
    require!(
        planet.recycler >= params.recycler,
        GameStateError::InsufficientShips
    );
    require!(
        planet.espionage_probe >= params.espionage_probe,
        GameStateError::InsufficientShips
    );
    require!(
        planet.colony_ship >= params.colony_ship,
        GameStateError::InsufficientShips
    );

    let cap = cargo_capacity(
        params.small_cargo,
        params.large_cargo,
        params.recycler,
        params.cruiser,
        params.battleship,
    );
    require!(
        params.cargo_metal + params.cargo_crystal + params.cargo_deuterium <= cap,
        GameStateError::ExceedsCargo
    );

    require!(
        planet.metal >= params.cargo_metal,
        GameStateError::InsufficientResources
    );
    require!(
        planet.crystal >= params.cargo_crystal,
        GameStateError::InsufficientResources
    );
    require!(
        planet.deuterium >= params.cargo_deuterium,
        GameStateError::InsufficientResources
    );

    let speed_factor = params.speed_factor.clamp(10, 100);
    let fleet_speed = slowest_fleet_speed(
        planet,
        params.light_fighter,
        params.heavy_fighter,
        params.cruiser,
        params.battleship,
        params.battlecruiser,
        params.bomber,
        params.destroyer,
        params.deathstar,
        params.small_cargo,
        params.large_cargo,
        params.recycler,
        params.espionage_probe,
        params.colony_ship,
    );
    require!(fleet_speed > 0, GameStateError::InvalidArgs);

    let flight_seconds = mission_flight_seconds(
        planet.galaxy,
        planet.system,
        planet.position,
        params.target_galaxy,
        params.target_system,
        params.target_position,
        speed_factor,
        fleet_speed,
        planet,
    );
    require!(flight_seconds > 0, GameStateError::InvalidArgs);

    let launch_fuel = launch_fuel_cost(
        params.light_fighter,
        params.heavy_fighter,
        params.cruiser,
        params.battleship,
        params.battlecruiser,
        params.bomber,
        params.destroyer,
        params.deathstar,
        params.small_cargo,
        params.large_cargo,
        params.recycler,
        params.espionage_probe,
        params.colony_ship,
        speed_factor,
    );

    require!(
        planet.deuterium >= params.cargo_deuterium + launch_fuel,
        GameStateError::InsufficientDeuterium
    );

    planet.metal -= params.cargo_metal;
    planet.crystal -= params.cargo_crystal;
    planet.deuterium -= params.cargo_deuterium + launch_fuel;

    planet.light_fighter -= params.light_fighter;
    planet.heavy_fighter -= params.heavy_fighter;
    planet.cruiser -= params.cruiser;
    planet.battleship -= params.battleship;
    planet.battlecruiser -= params.battlecruiser;
    planet.bomber -= params.bomber;
    planet.destroyer -= params.destroyer;
    planet.deathstar -= params.deathstar;
    planet.small_cargo -= params.small_cargo;
    planet.large_cargo -= params.large_cargo;
    planet.recycler -= params.recycler;
    planet.espionage_probe -= params.espionage_probe;
    planet.colony_ship -= params.colony_ship;

    let arrive_ts = now.saturating_add(flight_seconds);

    let return_ts = 0;

    planet.set_mission(
        slot,
        MissionState {
            mission_type: params.mission_type,
            target_galaxy: params.target_galaxy,
            target_system: params.target_system,
            target_position: params.target_position,
            colony_name: copy_name::<MAX_MISSION_COLONY_NAME_LEN>(&params.colony_name, ""),
            depart_ts: now,
            arrive_ts,
            return_ts,
            small_cargo: params.small_cargo,
            large_cargo: params.large_cargo,
            light_fighter: params.light_fighter,
            heavy_fighter: params.heavy_fighter,
            cruiser: params.cruiser,
            battleship: params.battleship,
            battlecruiser: params.battlecruiser,
            bomber: params.bomber,
            destroyer: params.destroyer,
            deathstar: params.deathstar,
            recycler: params.recycler,
            espionage_probe: params.espionage_probe,
            colony_ship: params.colony_ship,
            cargo_metal: params.cargo_metal,
            cargo_crystal: params.cargo_crystal,
            cargo_deuterium: params.cargo_deuterium,
            applied: false,
            speed_factor,
            combat_rounds: 0,
            attacker_won: false,
        },
    );

    planet.active_missions = planet.active_missions.saturating_add(1);
    if params.mission_type == MISSION_ATTACK {
        planet.last_attack_launch_ts = now;
    }
    Ok(())
}

#[derive(Clone, Copy)]
struct CombatStack {
    kind: u8,
    count: u32,
    attack: u64,
    shield: u64,
    hull: u64,
    metal_cost: u64,
    crystal_cost: u64,
    is_defense: bool,
    repairable: bool,
}

fn ship_stats(ship_type: u8) -> (u64, u64, u64) {
    match ship_type {
        0 => (5, 10, 400),
        1 => (5, 25, 1200),
        2 => (50, 10, 400),
        3 => (150, 25, 1000),
        4 => (400, 50, 2700),
        5 => (1000, 200, 6000),
        6 => (700, 400, 7000),
        7 => (1000, 500, 7500),
        8 => (2000, 500, 11000),
        9 => (200000, 50000, 900000),
        10 => (1, 10, 1600),
        11 => (1, 1, 100),
        12 => (50, 100, 3000),
        13 => (1, 1, 200),
        _ => (0, 0, 0),
    }
}

fn defense_stats(defense_type: u8) -> (u64, u64, u64) {
    match defense_type {
        0 => (80, 20, 200),
        1 => (100, 25, 200),
        2 => (250, 100, 800),
        3 => (1100, 200, 3500),
        4 => (250, 500, 800),
        5 => (3000, 300, 10000),
        6 => (1, 2000, 2000),
        7 => (1, 10000, 10000),
        8 => (1, 1, 800),
        9 => (12000, 1, 1500),
        _ => (0, 0, 0),
    }
}

fn rapid_fire_multiplier(attacker_kind: u8, defender_kind: u8) -> u64 {
    match (attacker_kind, defender_kind) {
        (6, 2) | (6, 3) => 4,
        (6, 4) => 7,
        (6, 5) => 2,
        (6, 0) | (6, 1) | (6, 10) => 3,
        (6, 11) | (6, 13) => 5,
        (7, 0) | (7, 1) | (7, 2) | (7, 3) | (7, 4) | (7, 5) => 2,
        (8, 6) => 2,
        (8, 7) => 3,
        _ => 1,
    }
}

fn apply_weapons_bonus(base_attack: u64, tech_level: u8) -> u64 {
    base_attack.saturating_mul(100 + tech_level as u64 * 10) / 100
}

fn apply_shield_bonus(base_shield: u64, tech_level: u8) -> u64 {
    base_shield.saturating_mul(100 + tech_level as u64 * 10) / 100
}

fn apply_armor_bonus(base_hull: u64, tech_level: u8) -> u64 {
    base_hull.saturating_mul(100 + tech_level as u64 * 10) / 100
}

fn ship_stack(count: u32, ship_type: u8, tech_source: &PlanetState) -> Option<CombatStack> {
    if count == 0 {
        return None;
    }
    let (attack, shield, hull) = ship_stats(ship_type);
    let (metal_cost, crystal_cost, _) = ship_cost(ship_type);
    Some(CombatStack {
        kind: ship_type,
        count,
        attack: apply_weapons_bonus(attack, tech_source.weapons_technology),
        shield: apply_shield_bonus(shield, tech_source.shielding_technology),
        hull: apply_armor_bonus(hull, tech_source.armor_technology),
        metal_cost,
        crystal_cost,
        is_defense: false,
        repairable: false,
    })
}

fn defense_stack(count: u32, defense_type: u8, tech_source: &PlanetState) -> Option<CombatStack> {
    if count == 0 {
        return None;
    }
    let (attack, shield, hull) = defense_stats(defense_type);
    let (metal_cost, crystal_cost, _) = defense_cost(defense_type);
    Some(CombatStack {
        kind: defense_type,
        count,
        attack: apply_weapons_bonus(attack, tech_source.weapons_technology),
        shield: apply_shield_bonus(shield, tech_source.shielding_technology),
        hull: apply_armor_bonus(hull, tech_source.armor_technology),
        metal_cost,
        crystal_cost,
        is_defense: true,
        repairable: defense_type <= 7,
    })
}

fn total_stack_units(stacks: &[CombatStack]) -> u64 {
    stacks.iter().map(|stack| stack.count as u64).sum()
}

fn apply_volley(attacking: &[CombatStack], defending: &mut [CombatStack]) -> u64 {
    let mut total_kills = 0u64;

    for attacker in attacking
        .iter()
        .filter(|stack| stack.count > 0 && stack.attack > 0)
    {
        let total_targets = total_stack_units(defending);
        if total_targets == 0 {
            break;
        }

        let mut remaining_shots = attacker.count as u64;
        for idx in 0..defending.len() {
            let target_count = defending[idx].count as u64;
            if target_count == 0 {
                continue;
            }

            let mut allocated = attacker.count as u64 * target_count / total_targets.max(1);
            if allocated == 0 && remaining_shots > 0 {
                allocated = 1;
            }
            allocated = allocated.min(remaining_shots);
            if allocated == 0 {
                continue;
            }
            remaining_shots = remaining_shots.saturating_sub(allocated);

            let multiplier = rapid_fire_multiplier(attacker.kind, defending[idx].kind);
            let effective_shots = allocated.saturating_mul(multiplier);
            let damage_per_shot = attacker.attack.saturating_sub(defending[idx].shield);
            if damage_per_shot == 0 {
                continue;
            }

            let possible_kills = effective_shots
                .saturating_mul(damage_per_shot)
                .checked_div(defending[idx].hull.max(1))
                .unwrap_or(0);
            let kills = possible_kills.min(defending[idx].count as u64) as u32;
            if kills > 0 {
                defending[idx].count = defending[idx].count.saturating_sub(kills);
                total_kills = total_kills.saturating_add(kills as u64);
            }
        }
    }

    total_kills
}

fn build_attacker_stacks(mission: &MissionState, source: &PlanetState) -> Vec<CombatStack> {
    let mut stacks = Vec::with_capacity(13);
    for maybe_stack in [
        ship_stack(mission.small_cargo, 0, source),
        ship_stack(mission.large_cargo, 1, source),
        ship_stack(mission.light_fighter, 2, source),
        ship_stack(mission.heavy_fighter, 3, source),
        ship_stack(mission.cruiser, 4, source),
        ship_stack(mission.battleship, 5, source),
        ship_stack(mission.battlecruiser, 6, source),
        ship_stack(mission.bomber, 7, source),
        ship_stack(mission.destroyer, 8, source),
        ship_stack(mission.deathstar, 9, source),
        ship_stack(mission.recycler, 10, source),
        ship_stack(mission.espionage_probe, 11, source),
        ship_stack(mission.colony_ship, 12, source),
    ] {
        if let Some(stack) = maybe_stack {
            stacks.push(stack);
        }
    }
    stacks
}

fn build_defender_stacks(destination: &PlanetState) -> Vec<CombatStack> {
    let mut stacks = Vec::with_capacity(24);
    for maybe_stack in [
        ship_stack(destination.small_cargo, 0, destination),
        ship_stack(destination.large_cargo, 1, destination),
        ship_stack(destination.light_fighter, 2, destination),
        ship_stack(destination.heavy_fighter, 3, destination),
        ship_stack(destination.cruiser, 4, destination),
        ship_stack(destination.battleship, 5, destination),
        ship_stack(destination.battlecruiser, 6, destination),
        ship_stack(destination.bomber, 7, destination),
        ship_stack(destination.destroyer, 8, destination),
        ship_stack(destination.deathstar, 9, destination),
        ship_stack(destination.recycler, 10, destination),
        ship_stack(destination.espionage_probe, 11, destination),
        ship_stack(destination.colony_ship, 12, destination),
        ship_stack(destination.solar_satellite, 13, destination),
        defense_stack(destination.rocket_launcher, 0, destination),
        defense_stack(destination.light_laser, 1, destination),
        defense_stack(destination.heavy_laser, 2, destination),
        defense_stack(destination.gauss_cannon, 3, destination),
        defense_stack(destination.ion_cannon, 4, destination),
        defense_stack(destination.plasma_turret, 5, destination),
        defense_stack(destination.small_shield_dome, 6, destination),
        defense_stack(destination.large_shield_dome, 7, destination),
    ] {
        if let Some(stack) = maybe_stack {
            stacks.push(stack);
        }
    }
    stacks
}

fn fleet_counts_from_stacks(stacks: &[CombatStack]) -> [u32; 14] {
    let mut counts = [0u32; 14];
    for stack in stacks.iter().filter(|stack| !stack.is_defense) {
        let idx = stack.kind as usize;
        if idx < counts.len() {
            counts[idx] = stack.count;
        }
    }
    counts
}

fn defense_counts_from_stacks(stacks: &[CombatStack]) -> [u32; 10] {
    let mut counts = [0u32; 10];
    for stack in stacks.iter().filter(|stack| stack.is_defense) {
        let idx = stack.kind as usize;
        if idx < counts.len() {
            counts[idx] = stack.count;
        }
    }
    counts
}

fn apply_defender_survivors(
    destination: &mut PlanetState,
    fleet_counts: [u32; 14],
    defense_counts: [u32; 10],
) {
    destination.small_cargo = fleet_counts[0];
    destination.large_cargo = fleet_counts[1];
    destination.light_fighter = fleet_counts[2];
    destination.heavy_fighter = fleet_counts[3];
    destination.cruiser = fleet_counts[4];
    destination.battleship = fleet_counts[5];
    destination.battlecruiser = fleet_counts[6];
    destination.bomber = fleet_counts[7];
    destination.destroyer = fleet_counts[8];
    destination.deathstar = fleet_counts[9];
    destination.recycler = fleet_counts[10];
    destination.espionage_probe = fleet_counts[11];
    destination.colony_ship = fleet_counts[12];
    destination.solar_satellite = fleet_counts[13];

    destination.rocket_launcher = defense_counts[0];
    destination.light_laser = defense_counts[1];
    destination.heavy_laser = defense_counts[2];
    destination.gauss_cannon = defense_counts[3];
    destination.ion_cannon = defense_counts[4];
    destination.plasma_turret = defense_counts[5];
    destination.small_shield_dome = defense_counts[6];
    destination.large_shield_dome = defense_counts[7];
}

fn fleet_debris(destroyed_count: u32, metal_cost: u64, crystal_cost: u64) -> (u64, u64) {
    let total_metal = metal_cost.saturating_mul(destroyed_count as u64);
    let total_crystal = crystal_cost.saturating_mul(destroyed_count as u64);
    (
        total_metal.saturating_mul(3) / 10,
        total_crystal.saturating_mul(3) / 10,
    )
}

fn plunder_resources(destination: &mut PlanetState, cargo_room: u64) -> (u64, u64, u64) {
    if cargo_room == 0 {
        return (0, 0, 0);
    }

    let desired_metal = destination.metal / 2;
    let desired_crystal = destination.crystal / 4;
    let desired_deuterium = destination.deuterium / 4;

    let metal = desired_metal.min(cargo_room);
    let after_metal = cargo_room.saturating_sub(metal);
    let crystal = desired_crystal.min(after_metal);
    let after_crystal = after_metal.saturating_sub(crystal);
    let deuterium = desired_deuterium.min(after_crystal);

    destination.metal = destination.metal.saturating_sub(metal);
    destination.crystal = destination.crystal.saturating_sub(crystal);
    destination.deuterium = destination.deuterium.saturating_sub(deuterium);

    (metal, crystal, deuterium)
}

fn collect_debris(
    coords: &mut PlanetCoordinates,
    cargo_room: u64,
    recyclers: u32,
) -> (u64, u64) {
    if cargo_room == 0 || recyclers == 0 {
        return (0, 0);
    }

    let metal = coords.debris_metal.min(cargo_room);
    let after_metal = cargo_room.saturating_sub(metal);
    let crystal = coords.debris_crystal.min(after_metal);

    coords.debris_metal = coords.debris_metal.saturating_sub(metal);
    coords.debris_crystal = coords.debris_crystal.saturating_sub(crystal);

    (metal, crystal)
}

fn emit_battle_resolved(
    source_planet_key: Pubkey,
    destination_planet_key: Pubkey,
    source: &PlanetState,
    destination: &PlanetState,
    attacker_survivors: [u32; 14],
    slot: usize,
    now: i64,
    combat_rounds: u8,
    attacker_won: bool,
    attacker_destroyed: bool,
    defender_survived: bool,
    loot_metal: u64,
    loot_crystal: u64,
    loot_deuterium: u64,
    debris_metal: u64,
    debris_crystal: u64,
    recycled_metal: u64,
    recycled_crystal: u64,
) {
    emit!(BattleResolvedEvent {
        source_planet: source_planet_key,
        destination_planet: destination_planet_key,
        attacker: source.authority,
        defender: destination.authority,
        source_galaxy: source.galaxy,
        source_system: source.system,
        source_position: source.position,
        target_galaxy: destination.galaxy,
        target_system: destination.system,
        target_position: destination.position,
        resolved_at: now,
        mission_slot: slot as u8,
        combat_rounds,
        attacker_won,
        attacker_destroyed,
        defender_survived,
        loot_metal,
        loot_crystal,
        loot_deuterium,
        debris_metal,
        debris_crystal,
        recycled_metal,
        recycled_crystal,
        attacker_small_cargo: attacker_survivors[0],
        attacker_large_cargo: attacker_survivors[1],
        attacker_light_fighter: attacker_survivors[2],
        attacker_heavy_fighter: attacker_survivors[3],
        attacker_cruiser: attacker_survivors[4],
        attacker_battleship: attacker_survivors[5],
        attacker_battlecruiser: attacker_survivors[6],
        attacker_bomber: attacker_survivors[7],
        attacker_destroyer: attacker_survivors[8],
        attacker_deathstar: attacker_survivors[9],
        attacker_recycler: attacker_survivors[10],
        attacker_espionage_probe: attacker_survivors[11],
        attacker_colony_ship: attacker_survivors[12],
    });
}

fn planet_building_score(planet: &PlanetState) -> u64 {
    planet.metal_mine as u64
        + planet.crystal_mine as u64
        + planet.deuterium_synthesizer as u64
        + planet.solar_plant as u64
        + planet.fusion_reactor as u64
        + planet.robotics_factory as u64
        + planet.nanite_factory as u64
        + planet.shipyard as u64
        + planet.metal_storage as u64
        + planet.crystal_storage as u64
        + planet.deuterium_tank as u64
        + planet.research_lab as u64
        + planet.missile_silo as u64
}

fn planet_fleet_points(planet: &PlanetState) -> u64 {
    fleet_combat_points(
        planet.light_fighter,
        planet.heavy_fighter,
        planet.cruiser,
        planet.battleship,
        planet.battlecruiser,
        planet.bomber,
        planet.destroyer,
        planet.deathstar,
        planet.small_cargo,
        planet.large_cargo,
        planet.recycler,
        planet.espionage_probe,
        planet.colony_ship,
    )
}

fn planet_defense_points(planet: &PlanetState) -> u64 {
    planet.rocket_launcher as u64 * 80
        + planet.light_laser as u64 * 100
        + planet.heavy_laser as u64 * 250
        + planet.gauss_cannon as u64 * 1_100
        + planet.ion_cannon as u64 * 250
        + planet.plasma_turret as u64 * 3_000
        + planet.small_shield_dome as u64 * 2_000
        + planet.large_shield_dome as u64 * 10_000
}

fn effective_attack_protection_until(planet: &PlanetState) -> i64 {
    let beginner_until = if planet.created_at > 0 {
        planet
            .created_at
            .saturating_add(NEW_PLAYER_PROTECTION_SECONDS)
    } else {
        0
    };
    planet.protection_until_ts.max(beginner_until)
}

fn espionage_sensor_score(source: &PlanetState, probes: u32) -> u64 {
    probes as u64 * 25 + source.computer_tech as u64 * 100 + source.astrophysics as u64 * 100
}

fn espionage_counter_score(destination: &PlanetState) -> u64 {
    destination.computer_tech as u64 * 100
        + destination.shielding_technology as u64 * 50
        + destination.armor_technology as u64 * 50
        + planet_defense_points(destination) / 100
        + planet_fleet_points(destination) / 250
}

fn espionage_reveal_level(sensor_score: u64, counter_score: u64) -> u8 {
    if sensor_score >= counter_score.saturating_add(600) {
        4
    } else if sensor_score >= counter_score.saturating_add(200) {
        3
    } else if sensor_score >= counter_score.saturating_mul(75) / 100 {
        2
    } else if sensor_score >= counter_score / 2 {
        1
    } else {
        0
    }
}

fn rounded_resource_report(value: u64, reveal_level: u8) -> u64 {
    match reveal_level {
        0 => 0,
        1 => value / 1_000 * 1_000,
        _ => value,
    }
}

fn espionage_probe_losses(
    probes: u32,
    sensor_score: u64,
    counter_score: u64,
    reveal_level: u8,
) -> u32 {
    if probes == 0 {
        return 0;
    }
    if reveal_level == 0 {
        return probes;
    }
    let pressure = counter_score.saturating_sub(sensor_score);
    let loss = pressure
        .saturating_mul(probes as u64)
        .checked_div(counter_score.max(1))
        .unwrap_or(0) as u32;
    loss.min(probes.saturating_sub(1))
}

fn emit_espionage_report(
    source_planet_key: Pubkey,
    destination_planet_key: Pubkey,
    source: &PlanetState,
    destination: &PlanetState,
    slot: usize,
    now: i64,
    probes_sent: u32,
    probes_survived: u32,
    sensor_score: u64,
    counter_score: u64,
    reveal_level: u8,
) {
    emit!(EspionageReportEvent {
        source_planet: source_planet_key,
        destination_planet: destination_planet_key,
        attacker: source.authority,
        defender: destination.authority,
        source_galaxy: source.galaxy,
        source_system: source.system,
        source_position: source.position,
        target_galaxy: destination.galaxy,
        target_system: destination.system,
        target_position: destination.position,
        resolved_at: now,
        mission_slot: slot as u8,
        reveal_level,
        probes_sent,
        probes_survived,
        probes_lost: probes_sent.saturating_sub(probes_survived),
        sensor_score,
        counter_score,
        reported_metal: rounded_resource_report(destination.metal, reveal_level),
        reported_crystal: rounded_resource_report(destination.crystal, reveal_level),
        reported_deuterium: rounded_resource_report(destination.deuterium, reveal_level),
        reported_building_score: if reveal_level >= 2 {
            planet_building_score(destination)
        } else {
            0
        },
        reported_fleet_points: if reveal_level >= 3 {
            planet_fleet_points(destination)
        } else {
            0
        },
        reported_defense_points: if reveal_level >= 3 {
            planet_defense_points(destination)
        } else {
            0
        },
        reported_weapons_technology: if reveal_level >= 4 {
            destination.weapons_technology
        } else {
            0
        },
        reported_shielding_technology: if reveal_level >= 4 {
            destination.shielding_technology
        } else {
            0
        },
        reported_armor_technology: if reveal_level >= 4 {
            destination.armor_technology
        } else {
            0
        },
    });
}

pub(crate) fn resolve_espionage_planets(
    source: &mut PlanetState,
    destination: &mut PlanetState,
    source_planet_key: Pubkey,
    destination_planet_key: Pubkey,
    slot: usize,
    now: i64,
) -> Result<()> {
    require!(slot < MAX_MISSIONS, GameStateError::InvalidMissionSlot);
    let mut mission = source.mission(slot);
    require!(
        mission.mission_type == MISSION_ESPIONAGE,
        GameStateError::InvalidMission
    );
    require!(
        mission.target_galaxy == destination.galaxy
            && mission.target_system == destination.system
            && mission.target_position == destination.position,
        GameStateError::InvalidDestination
    );
    require!(
        source.authority != destination.authority,
        GameStateError::CannotAttackOwnPlanet
    );
    require!(now >= mission.arrive_ts, GameStateError::MissionInFlight);

    if mission.applied {
        require!(mission.return_ts > 0, GameStateError::ReturnInFlight);
        require!(now >= mission.return_ts, GameStateError::ReturnInFlight);
        settle_resources(source, now)?;
        source.return_mission_assets(slot)?;
        source.clear_mission(slot);
        source.active_missions = source.active_missions.saturating_sub(1);
        return Ok(());
    }

    settle_resources(source, now)?;
    settle_resources(destination, now)?;

    let probes_sent = mission.espionage_probe;
    require!(probes_sent > 0, GameStateError::EmptyFleet);
    let sensor_score = espionage_sensor_score(source, probes_sent);
    let counter_score = espionage_counter_score(destination);
    let reveal_level = espionage_reveal_level(sensor_score, counter_score);
    let probes_lost =
        espionage_probe_losses(probes_sent, sensor_score, counter_score, reveal_level);
    let probes_survived = probes_sent.saturating_sub(probes_lost);

    emit_espionage_report(
        source_planet_key,
        destination_planet_key,
        source,
        destination,
        slot,
        now,
        probes_sent,
        probes_survived,
        sensor_score,
        counter_score,
        reveal_level,
    );

    if probes_survived == 0 {
        source.clear_mission(slot);
        source.active_missions = source.active_missions.saturating_sub(1);
        return Ok(());
    }

    mission.espionage_probe = probes_survived;
    mission.applied = true;
    mission.return_ts =
        now.saturating_add(mission.arrive_ts.saturating_sub(mission.depart_ts).max(1));
    source.set_mission(slot, mission);
    Ok(())
}

pub(crate) fn resolve_attack_planets(
    source: &mut PlanetState,
    destination: &mut PlanetState,
    destination_coords: &mut PlanetCoordinates,
    source_planet_key: Pubkey,
    destination_planet_key: Pubkey,
    slot: usize,
    now: i64,
) -> Result<()> {
    require!(slot < MAX_MISSIONS, GameStateError::InvalidMissionSlot);

    let mission = source.mission(slot);
    require!(
        mission.mission_type == MISSION_ATTACK,
        GameStateError::InvalidMission
    );

    if mission.applied {
        require!(
            mission.return_ts > 0 && now >= mission.return_ts,
            GameStateError::ReturnInFlight
        );
        settle_resources(source, now)?;
        source.return_mission_assets(slot)?;
        source.clear_mission(slot);
        source.active_missions = source.active_missions.saturating_sub(1);
        return Ok(());
    }

    require!(
        destination.authority != source.authority,
        GameStateError::CannotAttackOwnPlanet
    );
    require!(
        destination.galaxy == mission.target_galaxy
            && destination.system == mission.target_system
            && destination.position == mission.target_position,
        GameStateError::InvalidDestination
    );
    require!(
        destination_coords.galaxy == destination.galaxy
            && destination_coords.system == destination.system
            && destination_coords.position == destination.position,
        GameStateError::InvalidDestination
    );
    require!(now >= mission.arrive_ts, GameStateError::MissionInFlight);
    require!(
        now >= effective_attack_protection_until(destination),
        GameStateError::NewPlayerProtected
    );
    require!(
        destination.last_attacked_ts == 0
            || now
                >= destination
                    .last_attacked_ts
                    .saturating_add(TARGET_ATTACK_COOLDOWN_SECONDS),
        GameStateError::TargetCooldown
    );

    settle_resources(source, now)?;
    settle_resources(destination, now)?;
    destination.last_attacked_ts = now;

    let mut attacker = build_attacker_stacks(&mission, source);
    let mut defender = build_defender_stacks(destination);
    let initial_attacker = attacker.clone();
    let initial_defender = defender.clone();

    let mut combat_rounds = 0u8;
    for _ in 0..MAX_COMBAT_ROUNDS {
        if total_stack_units(&attacker) == 0 || total_stack_units(&defender) == 0 {
            break;
        }
        combat_rounds = combat_rounds.saturating_add(1);

        let defender_kills = apply_volley(&attacker, &mut defender);
        let attacker_kills = apply_volley(&defender, &mut attacker);

        if defender_kills == 0 && attacker_kills == 0 {
            break;
        }
    }

    let attacker_survivors = fleet_counts_from_stacks(&attacker);
    let defender_survived_combat = total_stack_units(&defender) > 0;
    let mut defender_survivors = fleet_counts_from_stacks(&defender);
    let mut defense_survivors = defense_counts_from_stacks(&defender);

    let mut debris_metal = 0u64;
    let mut debris_crystal = 0u64;

    for (before, after) in initial_attacker.iter().zip(attacker.iter()) {
        let destroyed = before.count.saturating_sub(after.count);
        let (metal, crystal) = fleet_debris(destroyed, before.metal_cost, before.crystal_cost);
        debris_metal = debris_metal.saturating_add(metal);
        debris_crystal = debris_crystal.saturating_add(crystal);
    }

    for (before, after) in initial_defender.iter().zip(defender.iter()) {
        let destroyed = before.count.saturating_sub(after.count);
        if before.is_defense && before.repairable {
            let repaired = destroyed.saturating_mul(7) / 10;
            let permanently_destroyed = destroyed.saturating_sub(repaired);
            let (metal, crystal) = fleet_debris(
                permanently_destroyed,
                before.metal_cost,
                before.crystal_cost,
            );
            debris_metal = debris_metal.saturating_add(metal);
            debris_crystal = debris_crystal.saturating_add(crystal);
            let repaired_total = after.count.saturating_add(repaired);
            defense_survivors[before.kind as usize] = repaired_total;
        } else if before.is_defense {
            defense_survivors[before.kind as usize] = after.count;
        } else {
            defender_survivors[before.kind as usize] = after.count;
            let (metal, crystal) = fleet_debris(destroyed, before.metal_cost, before.crystal_cost);
            debris_metal = debris_metal.saturating_add(metal);
            debris_crystal = debris_crystal.saturating_add(crystal);
        }
    }

    destination_coords.debris_metal = destination_coords.debris_metal.saturating_add(debris_metal);
    destination_coords.debris_crystal = destination_coords
        .debris_crystal
        .saturating_add(debris_crystal);
    apply_defender_survivors(destination, defender_survivors, defense_survivors);

    let attacker_alive = attacker_survivors
        .iter()
        .copied()
        .map(u64::from)
        .sum::<u64>()
        > 0;
    let attacker_won = !defender_survived_combat;

    if !attacker_alive {
        emit_battle_resolved(
            source_planet_key,
            destination_planet_key,
            source,
            destination,
            attacker_survivors,
            slot,
            now,
            combat_rounds,
            false,
            true,
            defender_survived_combat,
            0,
            0,
            0,
            debris_metal,
            debris_crystal,
            0,
            0,
        );
        source.clear_mission(slot);
        source.active_missions = source.active_missions.saturating_sub(1);
        return Ok(());
    }

    let return_flight_seconds = mission.arrive_ts.saturating_sub(mission.depart_ts).max(1);
    let return_fuel = launch_fuel_cost(
        attacker_survivors[2],
        attacker_survivors[3],
        attacker_survivors[4],
        attacker_survivors[5],
        attacker_survivors[6],
        attacker_survivors[7],
        attacker_survivors[8],
        attacker_survivors[9],
        attacker_survivors[0],
        attacker_survivors[1],
        attacker_survivors[10],
        attacker_survivors[11],
        attacker_survivors[12],
        mission.speed_factor,
    );
    require!(
        source.deuterium >= return_fuel,
        GameStateError::InsufficientDeuterium
    );
    source.deuterium = source.deuterium.saturating_sub(return_fuel);

    let mut cargo_metal = 0u64;
    let mut cargo_crystal = 0u64;
    let mut cargo_deuterium = 0u64;
    let mut loot_metal = 0u64;
    let mut loot_crystal = 0u64;
    let mut loot_deuterium = 0u64;
    let mut recycled_metal = 0u64;
    let mut recycled_crystal = 0u64;
    if attacker_won {
        let total_cargo_capacity = cargo_capacity(
            attacker_survivors[0],
            attacker_survivors[1],
            attacker_survivors[10],
            attacker_survivors[4],
            attacker_survivors[5],
        );
        let launched_cargo = mission
            .cargo_metal
            .saturating_add(mission.cargo_crystal)
            .saturating_add(mission.cargo_deuterium);
        let cargo_room = total_cargo_capacity.saturating_sub(launched_cargo);
        (loot_metal, loot_crystal, loot_deuterium) = plunder_resources(destination, cargo_room);
        cargo_metal = mission.cargo_metal.saturating_add(loot_metal);
        cargo_crystal = mission.cargo_crystal.saturating_add(loot_crystal);
        cargo_deuterium = mission.cargo_deuterium.saturating_add(loot_deuterium);

        let cargo_used = cargo_metal
            .saturating_add(cargo_crystal)
            .saturating_add(cargo_deuterium);
        let debris_room = total_cargo_capacity.saturating_sub(cargo_used);
        (recycled_metal, recycled_crystal) =
            collect_debris(destination_coords, debris_room, attacker_survivors[10]);
        cargo_metal = cargo_metal.saturating_add(recycled_metal);
        cargo_crystal = cargo_crystal.saturating_add(recycled_crystal);
    }

    source.missions[slot].small_cargo = attacker_survivors[0];
    source.missions[slot].large_cargo = attacker_survivors[1];
    source.missions[slot].light_fighter = attacker_survivors[2];
    source.missions[slot].heavy_fighter = attacker_survivors[3];
    source.missions[slot].cruiser = attacker_survivors[4];
    source.missions[slot].battleship = attacker_survivors[5];
    source.missions[slot].battlecruiser = attacker_survivors[6];
    source.missions[slot].bomber = attacker_survivors[7];
    source.missions[slot].destroyer = attacker_survivors[8];
    source.missions[slot].deathstar = attacker_survivors[9];
    source.missions[slot].recycler = attacker_survivors[10];
    source.missions[slot].espionage_probe = attacker_survivors[11];
    source.missions[slot].colony_ship = attacker_survivors[12];
    source.missions[slot].cargo_metal = cargo_metal;
    source.missions[slot].cargo_crystal = cargo_crystal;
    source.missions[slot].cargo_deuterium = cargo_deuterium;
    source.missions[slot].return_ts = now.saturating_add(return_flight_seconds);
    source.missions[slot].combat_rounds = combat_rounds;
    source.missions[slot].attacker_won = attacker_won;
    source.set_mission_applied(slot, true);

    emit_battle_resolved(
        source_planet_key,
        destination_planet_key,
        source,
        destination,
        attacker_survivors,
        slot,
        now,
        combat_rounds,
        attacker_won,
        false,
        defender_survived_combat,
        loot_metal,
        loot_crystal,
        loot_deuterium,
        debris_metal,
        debris_crystal,
        recycled_metal,
        recycled_crystal,
    );

    Ok(())
}

pub(crate) fn resolve_transport_planets(
    source: &mut PlanetState,
    destination: &mut PlanetState,
    slot: usize,
    now: i64,
) -> Result<()> {
    require!(slot < MAX_MISSIONS, GameStateError::InvalidMissionSlot);

    let mission = source.mission(slot);
    require!(
        mission.mission_type == MISSION_TRANSPORT,
        GameStateError::InvalidMission
    );
    require!(
        mission.target_galaxy == destination.galaxy
            && mission.target_system == destination.system
            && mission.target_position == destination.position,
        GameStateError::InvalidDestination
    );

    if !mission.applied {
        require!(now >= mission.arrive_ts, GameStateError::MissionInFlight);

        settle_resources(source, now)?;
        settle_resources(destination, now)?;

        destination.credit_resources(
            mission.cargo_metal,
            mission.cargo_crystal,
            mission.cargo_deuterium,
        )?;

        if destination.authority == source.authority {
            // Same owner: ships stay at destination and mission ends here.
            destination.small_cargo = destination.small_cargo.saturating_add(mission.small_cargo);
            destination.large_cargo = destination.large_cargo.saturating_add(mission.large_cargo);
            destination.light_fighter = destination
                .light_fighter
                .saturating_add(mission.light_fighter);
            destination.heavy_fighter = destination
                .heavy_fighter
                .saturating_add(mission.heavy_fighter);
            destination.cruiser = destination.cruiser.saturating_add(mission.cruiser);
            destination.battleship = destination.battleship.saturating_add(mission.battleship);
            destination.battlecruiser = destination
                .battlecruiser
                .saturating_add(mission.battlecruiser);
            destination.bomber = destination.bomber.saturating_add(mission.bomber);
            destination.destroyer = destination.destroyer.saturating_add(mission.destroyer);
            destination.deathstar = destination.deathstar.saturating_add(mission.deathstar);
            destination.recycler = destination.recycler.saturating_add(mission.recycler);
            destination.espionage_probe = destination
                .espionage_probe
                .saturating_add(mission.espionage_probe);
            destination.colony_ship = destination.colony_ship.saturating_add(mission.colony_ship);

            source.clear_mission(slot);
            source.active_missions = source.active_missions.saturating_sub(1);
            return Ok(());
        }

        // Different owner: deduct return fuel from source planet.
        let return_fuel = launch_fuel_cost(
            mission.light_fighter,
            mission.heavy_fighter,
            mission.cruiser,
            mission.battleship,
            mission.battlecruiser,
            mission.bomber,
            mission.destroyer,
            mission.deathstar,
            mission.small_cargo,
            mission.large_cargo,
            mission.recycler,
            mission.espionage_probe,
            mission.colony_ship,
            mission.speed_factor,
        );

        require!(
            source.deuterium >= return_fuel,
            GameStateError::InsufficientDeuterium
        );

        source.deuterium -= return_fuel;

        let return_flight_seconds = mission.arrive_ts.saturating_sub(mission.depart_ts).max(1);

        source.missions[slot].cargo_metal = 0;
        source.missions[slot].cargo_crystal = 0;
        source.missions[slot].cargo_deuterium = 0;
        source.missions[slot].return_ts = now.saturating_add(return_flight_seconds);
        source.set_mission_applied(slot, true);

        return Ok(());
    }

    require!(
        mission.return_ts > 0 && now >= mission.return_ts,
        GameStateError::ReturnInFlight
    );

    source.return_mission_ships_only(slot);
    source.clear_mission(slot);
    source.active_missions = source.active_missions.saturating_sub(1);
    Ok(())
}

pub(crate) fn resolve_transport_empty_slot(
    source: &mut PlanetState,
    slot: usize,
    now: i64,
) -> Result<()> {
    require!(slot < MAX_MISSIONS, GameStateError::InvalidMissionSlot);

    let mission = source.mission(slot);
    require!(
        mission.mission_type == MISSION_TRANSPORT,
        GameStateError::InvalidMission
    );

    if !mission.applied {
        require!(now >= mission.arrive_ts, GameStateError::MissionInFlight);

        settle_resources(source, now)?;

        let return_fuel = launch_fuel_cost(
            mission.light_fighter,
            mission.heavy_fighter,
            mission.cruiser,
            mission.battleship,
            mission.battlecruiser,
            mission.bomber,
            mission.destroyer,
            mission.deathstar,
            mission.small_cargo,
            mission.large_cargo,
            mission.recycler,
            mission.espionage_probe,
            mission.colony_ship,
            mission.speed_factor,
        );

        require!(
            source.deuterium >= return_fuel,
            GameStateError::InsufficientDeuterium
        );
        source.deuterium -= return_fuel;

        let return_flight_seconds = mission.arrive_ts.saturating_sub(mission.depart_ts).max(1);

        source.missions[slot].return_ts = now.saturating_add(return_flight_seconds);
        source.set_mission_applied(slot, true);

        return Ok(());
    }

    require!(
        mission.return_ts > 0 && now >= mission.return_ts,
        GameStateError::ReturnInFlight
    );

    settle_resources(source, now)?;
    source.return_mission_assets(slot)?;
    source.clear_mission(slot);
    source.active_missions = source.active_missions.saturating_sub(1);
    Ok(())
}

/// Resolve a colonize mission.
///
/// The `colony_planet` and `colony_coords` accounts must ALREADY be initialized
/// (by `initialize_colony` / `initialize_colony_vault` in the same tx, or by a
/// preceding tx). This instruction only clears the mission slot on the source
/// planet; it does NOT create any accounts.
pub(crate) fn resolve_colonize_planet(
    source: &mut PlanetState,
    slot: usize,
    now: i64,
) -> Result<()> {
    require!(slot < MAX_MISSIONS, GameStateError::InvalidMissionSlot);

    let mission = source.mission(slot);
    require!(
        mission.mission_type == MISSION_COLONIZE,
        GameStateError::InvalidMission
    );
    require!(!mission.applied, GameStateError::AlreadyResolved);
    require!(now >= mission.arrive_ts, GameStateError::MissionInFlight);
    require!(mission.colony_ship > 0, GameStateError::MissingColonyShip);

    validate_coordinates(
        mission.target_galaxy,
        mission.target_system,
        mission.target_position,
    )?;

    // Guard: source planet must not be at the target coords
    let coords_taken = source.galaxy == mission.target_galaxy
        && source.system == mission.target_system
        && source.position == mission.target_position;
    require!(!coords_taken, GameStateError::InvalidDestination);

    source.clear_mission(slot);
    source.active_missions = source.active_missions.saturating_sub(1);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_planet() -> PlanetState {
        PlanetState {
            authority: Pubkey::default(),
            player: Pubkey::default(),
            planet_index: 0,
            galaxy: 1,
            system: 1,
            position: 1,
            name: [0u8; MAX_PLANET_NAME_LEN],
            diameter: 0,
            temperature: 0,
            max_fields: 0,
            used_fields: 0,
            metal_mine: 0,
            crystal_mine: 0,
            deuterium_synthesizer: 0,
            solar_plant: 0,
            fusion_reactor: 0,
            robotics_factory: 0,
            nanite_factory: 0,
            shipyard: 0,
            metal_storage: 0,
            crystal_storage: 0,
            deuterium_tank: 0,
            research_lab: 0,
            missile_silo: 0,
            energy_tech: 0,
            combustion_drive: 6,
            impulse_drive: 6,
            hyperspace_drive: 6,
            computer_tech: 0,
            astrophysics: 6,
            igr_network: 0,
            weapons_technology: 0,
            shielding_technology: 0,
            armor_technology: 0,
            research_queue_item: 0,
            research_queue_target: 0,
            research_finish_ts: 0,
            build_queue_item: 0,
            build_queue_target: 0,
            build_finish_ts: 0,
            metal: 0,
            crystal: 0,
            deuterium: 0,
            metal_hour: 0,
            crystal_hour: 0,
            deuterium_hour: 0,
            energy_production: 0,
            energy_consumption: 0,
            metal_cap: 1_000_000,
            crystal_cap: 1_000_000,
            deuterium_cap: 1_000_000,
            last_update_ts: 0,
            created_at: 0,
            protection_until_ts: 0,
            market_unlocked_at: 0,
            attack_unlocked_at: 0,
            last_attack_launch_ts: 0,
            last_attacked_ts: 0,
            small_cargo: 0,
            large_cargo: 0,
            light_fighter: 0,
            heavy_fighter: 0,
            cruiser: 0,
            battleship: 0,
            battlecruiser: 0,
            bomber: 0,
            destroyer: 0,
            deathstar: 0,
            recycler: 0,
            espionage_probe: 0,
            colony_ship: 0,
            solar_satellite: 0,
            rocket_launcher: 0,
            light_laser: 0,
            heavy_laser: 0,
            gauss_cannon: 0,
            ion_cannon: 0,
            plasma_turret: 0,
            small_shield_dome: 0,
            large_shield_dome: 0,
            anti_ballistic_missile: 0,
            interplanetary_missile: 0,
            active_missions: 0,
            missions: [MissionState::default(); MAX_MISSIONS],
            bump: 0,
            ship_build_item: 0,
            ship_build_qty: 0,
            ship_build_finish_ts: 0,
            defense_build_item: 0,
            defense_build_qty: 0,
            defense_build_finish_ts: 0,
        }
    }

    fn attack_mission(target: &PlanetState) -> MissionState {
        MissionState {
            mission_type: MISSION_ATTACK,
            target_galaxy: target.galaxy,
            target_system: target.system,
            target_position: target.position,
            depart_ts: 10,
            arrive_ts: 20,
            speed_factor: 100,
            ..MissionState::default()
        }
    }

    fn espionage_mission(target: &PlanetState, probes: u32) -> MissionState {
        MissionState {
            mission_type: MISSION_ESPIONAGE,
            target_galaxy: target.galaxy,
            target_system: target.system,
            target_position: target.position,
            depart_ts: 10,
            arrive_ts: 20,
            espionage_probe: probes,
            speed_factor: 100,
            ..MissionState::default()
        }
    }

    fn coords_for(planet: &PlanetState) -> PlanetCoordinates {
        PlanetCoordinates {
            galaxy: planet.galaxy,
            system: planet.system,
            position: planet.position,
            planet: Pubkey::new_unique(),
            authority: planet.authority,
            debris_metal: 0,
            debris_crystal: 0,
            bump: 0,
        }
    }

    #[test]
    fn distance_scales_with_system_and_galaxy_jumps() {
        assert!(distance(1, 1, 1, 1, 2, 1) < distance(1, 1, 1, 1, 300, 1));
        assert!(distance(1, 1, 1, 2, 1, 1) < distance(1, 1, 1, 300, 1, 1));
        assert!(distance(1, 1, 1, 2, 3, 4) < distance(1, 1, 1, 300, 400, 10));
    }

    #[test]
    fn flight_time_scales_with_larger_galaxy_jumps() {
        let planet = test_planet();
        let fleet_speed = slowest_fleet_speed(&planet, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0);

        let one_galaxy_jump = mission_flight_seconds(1, 1, 1, 2, 1, 1, 100, fleet_speed, &planet);
        let many_galaxy_jumps =
            mission_flight_seconds(1, 1, 1, 300, 1, 1, 100, fleet_speed, &planet);

        assert!(one_galaxy_jump < many_galaxy_jumps);
    }

    #[test]
    fn flight_time_has_slowed_tier_baselines() {
        let mut planet = test_planet();
        planet.astrophysics = 0;
        let reference_speed = 5_000;

        assert_eq!(
            mission_flight_seconds(1, 1, 1, 1, 1, 2, 100, reference_speed, &planet),
            300
        );
        assert_eq!(
            mission_flight_seconds(1, 1, 1, 1, 2, 1, 100, reference_speed, &planet),
            3_600
        );
        assert_eq!(
            mission_flight_seconds(1, 1, 1, 2, 1, 1, 100, reference_speed, &planet),
            86_400
        );
    }

    #[test]
    fn slowest_ship_sets_fleet_travel_time() {
        let planet = test_planet();
        let fast_fleet_speed = slowest_fleet_speed(&planet, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0);
        let mixed_fleet_speed = slowest_fleet_speed(&planet, 1, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0);

        let fast_time = mission_flight_seconds(1, 1, 1, 1, 50, 1, 100, fast_fleet_speed, &planet);
        let mixed_time = mission_flight_seconds(1, 1, 1, 1, 50, 1, 100, mixed_fleet_speed, &planet);

        assert!(mixed_fleet_speed < fast_fleet_speed);
        assert!(mixed_time > fast_time);
    }

    #[test]
    fn launch_espionage_requires_probe_only_fleet() {
        let mut planet = test_planet();
        planet.espionage_probe = 3;
        planet.light_fighter = 1;
        planet.deuterium = 1_000;

        let mixed = launch_fleet_planet(
            &mut planet,
            LaunchFleetParams {
                mission_type: MISSION_ESPIONAGE,
                light_fighter: 1,
                heavy_fighter: 0,
                cruiser: 0,
                battleship: 0,
                battlecruiser: 0,
                bomber: 0,
                destroyer: 0,
                deathstar: 0,
                small_cargo: 0,
                large_cargo: 0,
                recycler: 0,
                espionage_probe: 1,
                colony_ship: 0,
                cargo_metal: 0,
                cargo_crystal: 0,
                cargo_deuterium: 0,
                speed_factor: 100,
                now: 0,
                target_galaxy: 1,
                target_system: 2,
                target_position: 1,
                colony_name: String::new(),
            },
        );

        assert!(mixed.is_err());
        assert_eq!(planet.espionage_probe, 3);
        assert_eq!(planet.light_fighter, 1);
    }

    #[test]
    fn espionage_success_returns_surviving_probes() {
        let mut source = test_planet();
        source.authority = Pubkey::new_unique();
        source.computer_tech = 6;
        source.astrophysics = 6;

        let mut destination = test_planet();
        destination.authority = Pubkey::new_unique();
        destination.galaxy = 2;
        destination.system = 2;
        destination.position = 2;
        destination.metal = 12_345;

        source.missions[0] = espionage_mission(&destination, 5);
        source.active_missions = 1;

        resolve_espionage_planets(
            &mut source,
            &mut destination,
            Pubkey::new_unique(),
            Pubkey::new_unique(),
            0,
            20,
        )
        .unwrap();

        assert!(source.missions[0].applied);
        assert!(source.missions[0].return_ts > 20);
        assert_eq!(source.missions[0].espionage_probe, 5);

        let return_ts = source.missions[0].return_ts;
        resolve_espionage_planets(
            &mut source,
            &mut destination,
            Pubkey::new_unique(),
            Pubkey::new_unique(),
            0,
            return_ts,
        )
        .unwrap();

        assert_eq!(source.active_missions, 0);
        assert_eq!(source.missions[0].mission_type, 0);
        assert_eq!(source.espionage_probe, 5);
    }

    #[test]
    fn espionage_blind_report_destroys_probes() {
        let mut source = test_planet();
        source.authority = Pubkey::new_unique();

        let mut destination = test_planet();
        destination.authority = Pubkey::new_unique();
        destination.galaxy = 2;
        destination.system = 2;
        destination.position = 2;
        destination.computer_tech = 10;
        destination.shielding_technology = 10;
        destination.armor_technology = 10;
        destination.plasma_turret = 20;

        source.missions[0] = espionage_mission(&destination, 1);
        source.active_missions = 1;

        resolve_espionage_planets(
            &mut source,
            &mut destination,
            Pubkey::new_unique(),
            Pubkey::new_unique(),
            0,
            20,
        )
        .unwrap();

        assert_eq!(source.active_missions, 0);
        assert_eq!(source.missions[0].mission_type, 0);
        assert_eq!(source.espionage_probe, 0);
    }

    #[test]
    fn attack_ship_requirements_block_locked_ships() {
        let mut planet = test_planet();
        planet.shipyard = 1;
        planet.combustion_drive = 0;

        assert!(enforce_ship_research_gate(0, &planet).is_err());
        assert!(enforce_ship_research_gate(2, &planet).is_ok());
        assert!(enforce_ship_research_gate(6, &planet).is_err());
        assert!(enforce_ship_research_gate(9, &planet).is_err());
    }

    #[test]
    fn attack_ship_requirements_allow_unlocked_ships() {
        let mut planet = test_planet();
        planet.shipyard = 12;
        planet.combustion_drive = 6;
        planet.impulse_drive = 6;
        planet.hyperspace_drive = 7;
        planet.computer_tech = 5;
        planet.weapons_technology = 10;
        planet.shielding_technology = 2;
        planet.armor_technology = 6;
        planet.energy_tech = 12;
        planet.astrophysics = 4;

        for ship_type in 0..=13 {
            assert!(enforce_ship_research_gate(ship_type, &planet).is_ok());
        }
    }

    #[test]
    fn attack_resolution_generates_return_trip_and_debris() {
        let mut source = test_planet();
        source.authority = Pubkey::new_unique();
        source.deuterium = 10_000;
        source.light_fighter = 10;

        let mut destination = test_planet();
        destination.authority = Pubkey::new_unique();
        destination.galaxy = 2;
        destination.system = 2;
        destination.position = 2;
        destination.rocket_launcher = 4;

        let mut coords = PlanetCoordinates {
            galaxy: 2,
            system: 2,
            position: 2,
            planet: Pubkey::new_unique(),
            authority: destination.authority,
            debris_metal: 0,
            debris_crystal: 0,
            bump: 0,
        };
        let source_key = Pubkey::new_unique();
        let destination_key = Pubkey::new_unique();

        source.missions[0] = MissionState {
            mission_type: MISSION_ATTACK,
            target_galaxy: 2,
            target_system: 2,
            target_position: 2,
            depart_ts: 10,
            arrive_ts: 20,
            light_fighter: 10,
            speed_factor: 100,
            ..MissionState::default()
        };
        source.active_missions = 1;

        resolve_attack_planets(
            &mut source,
            &mut destination,
            &mut coords,
            source_key,
            destination_key,
            0,
            20,
        )
        .unwrap();

        assert!(source.missions[0].applied);
        assert!(source.missions[0].return_ts > 20);
        assert!(coords.debris_metal + coords.debris_crystal > 0);
    }

    #[test]
    fn attack_resolution_rejects_own_planet() {
        let owner = Pubkey::new_unique();
        let mut source = test_planet();
        source.authority = owner;

        let mut destination = test_planet();
        destination.authority = owner;
        destination.galaxy = 2;
        destination.system = 2;
        destination.position = 2;

        let mut coords = coords_for(&destination);
        source.missions[0] = MissionState {
            light_fighter: 10,
            ..attack_mission(&destination)
        };
        source.active_missions = 1;

        let result = resolve_attack_planets(
            &mut source,
            &mut destination,
            &mut coords,
            Pubkey::new_unique(),
            Pubkey::new_unique(),
            0,
            20,
        );

        assert!(result.is_err());
        assert_eq!(source.active_missions, 1);
        assert!(!source.missions[0].applied);
    }

    #[test]
    fn attack_resolution_respects_target_protection_and_cooldown() {
        let mut source = test_planet();
        source.authority = Pubkey::new_unique();

        let mut protected_target = test_planet();
        protected_target.authority = Pubkey::new_unique();
        protected_target.galaxy = 2;
        protected_target.system = 2;
        protected_target.position = 2;
        protected_target.protection_until_ts = 100;

        let mut protected_coords = coords_for(&protected_target);
        source.missions[0] = MissionState {
            light_fighter: 10,
            ..attack_mission(&protected_target)
        };
        source.active_missions = 1;

        let protected_result = resolve_attack_planets(
            &mut source,
            &mut protected_target,
            &mut protected_coords,
            Pubkey::new_unique(),
            Pubkey::new_unique(),
            0,
            20,
        );
        assert!(protected_result.is_err());
        assert_eq!(protected_target.last_attacked_ts, 0);

        let mut cooled_target = test_planet();
        cooled_target.authority = Pubkey::new_unique();
        cooled_target.galaxy = 2;
        cooled_target.system = 2;
        cooled_target.position = 2;
        cooled_target.last_attacked_ts = 100;

        let mut cooled_coords = coords_for(&cooled_target);
        source.missions[0] = MissionState {
            light_fighter: 10,
            ..attack_mission(&cooled_target)
        };

        let cooldown_result = resolve_attack_planets(
            &mut source,
            &mut cooled_target,
            &mut cooled_coords,
            Pubkey::new_unique(),
            Pubkey::new_unique(),
            0,
            150,
        );
        assert!(cooldown_result.is_err());
        assert_eq!(cooled_target.last_attacked_ts, 100);
    }

    #[test]
    fn attacker_wipeout_clears_mission_and_leaves_debris() {
        let mut source = test_planet();
        source.authority = Pubkey::new_unique();
        source.deuterium = 10_000;

        let mut destination = test_planet();
        destination.authority = Pubkey::new_unique();
        destination.galaxy = 2;
        destination.system = 2;
        destination.position = 2;
        destination.deathstar = 1;

        let mut coords = coords_for(&destination);
        source.missions[0] = MissionState {
            light_fighter: 1,
            ..attack_mission(&destination)
        };
        source.active_missions = 1;

        resolve_attack_planets(
            &mut source,
            &mut destination,
            &mut coords,
            Pubkey::new_unique(),
            Pubkey::new_unique(),
            0,
            20,
        )
        .unwrap();

        assert_eq!(source.active_missions, 0);
        assert_eq!(source.missions[0].mission_type, 0);
        assert_eq!(source.light_fighter, 0);
        assert!(destination.deathstar > 0);
        assert!(coords.debris_metal + coords.debris_crystal > 0);
    }

    #[test]
    fn returning_attack_restores_survivors_and_loot() {
        let mut source = test_planet();
        source.authority = Pubkey::new_unique();
        source.deuterium = 20_000;

        let mut destination = test_planet();
        destination.authority = Pubkey::new_unique();
        destination.galaxy = 2;
        destination.system = 2;
        destination.position = 2;
        destination.metal = 10_000;
        destination.crystal = 10_000;
        destination.deuterium = 10_000;

        let mut coords = coords_for(&destination);
        source.missions[0] = MissionState {
            small_cargo: 1,
            battlecruiser: 1,
            ..attack_mission(&destination)
        };
        source.active_missions = 1;

        resolve_attack_planets(
            &mut source,
            &mut destination,
            &mut coords,
            Pubkey::new_unique(),
            Pubkey::new_unique(),
            0,
            20,
        )
        .unwrap();

        let return_ts = source.missions[0].return_ts;
        let returning_metal = source.missions[0].cargo_metal;
        assert!(source.missions[0].applied);
        assert!(return_ts > 20);
        assert!(returning_metal > 0);

        resolve_attack_planets(
            &mut source,
            &mut destination,
            &mut coords,
            Pubkey::new_unique(),
            Pubkey::new_unique(),
            0,
            return_ts,
        )
        .unwrap();

        assert_eq!(source.active_missions, 0);
        assert_eq!(source.missions[0].mission_type, 0);
        assert_eq!(source.small_cargo, 1);
        assert_eq!(source.battlecruiser, 1);
        assert!(source.metal >= returning_metal);
    }

    #[test]
    fn surviving_recyclers_collect_attack_debris() {
        let mut source = test_planet();
        source.authority = Pubkey::new_unique();
        source.deuterium = 20_000;

        let mut destination = test_planet();
        destination.authority = Pubkey::new_unique();
        destination.galaxy = 2;
        destination.system = 2;
        destination.position = 2;
        destination.rocket_launcher = 1;

        let mut coords = coords_for(&destination);
        source.missions[0] = MissionState {
            battlecruiser: 1,
            recycler: 1,
            ..attack_mission(&destination)
        };
        source.active_missions = 1;

        resolve_attack_planets(
            &mut source,
            &mut destination,
            &mut coords,
            Pubkey::new_unique(),
            Pubkey::new_unique(),
            0,
            20,
        )
        .unwrap();

        assert!(source.missions[0].attacker_won);
        assert_eq!(source.missions[0].recycler, 1);
        assert!(source.missions[0].cargo_metal + source.missions[0].cargo_crystal > 0);
        assert_eq!(coords.debris_metal + coords.debris_crystal, 0);
    }

    #[test]
    fn repaired_defenses_do_not_block_loot_after_attacker_wins() {
        let mut source = test_planet();
        source.authority = Pubkey::new_unique();
        source.deuterium = 10_000;

        let mut destination = test_planet();
        destination.authority = Pubkey::new_unique();
        destination.galaxy = 2;
        destination.system = 2;
        destination.position = 2;
        destination.metal = 10_000;
        destination.crystal = 10_000;
        destination.deuterium = 10_000;
        destination.rocket_launcher = 1;

        let mut coords = PlanetCoordinates {
            galaxy: 2,
            system: 2,
            position: 2,
            planet: Pubkey::new_unique(),
            authority: destination.authority,
            debris_metal: 0,
            debris_crystal: 0,
            bump: 0,
        };

        source.missions[0] = MissionState {
            mission_type: MISSION_ATTACK,
            target_galaxy: 2,
            target_system: 2,
            target_position: 2,
            depart_ts: 10,
            arrive_ts: 20,
            small_cargo: 1,
            battlecruiser: 1,
            speed_factor: 100,
            ..MissionState::default()
        };
        source.active_missions = 1;

        resolve_attack_planets(
            &mut source,
            &mut destination,
            &mut coords,
            Pubkey::new_unique(),
            Pubkey::new_unique(),
            0,
            20,
        )
        .unwrap();

        assert!(source.missions[0].attacker_won);
        assert!(source.missions[0].cargo_metal > 0);
        assert!(destination.metal < 10_000);
    }
}
