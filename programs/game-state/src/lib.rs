use anchor_lang::prelude::*;

declare_id!("7yKyjQ7m8tSqvqYnV65aVV9Jwdee7KqyELeDXf6Fxkt4");

pub const MAX_PLANET_NAME_LEN: usize = 32;
pub const MAX_MISSION_COLONY_NAME_LEN: usize = 32;
pub const MAX_MISSIONS: usize = 4;
pub const MISSION_TRANSPORT: u8 = 2;
pub const MISSION_COLONIZE: u8 = 5;
pub const PLANET_COORDS_SPACE: usize = 8 + PlanetCoordinates::INIT_SPACE;
pub const PLAYER_PROFILE_SPACE: usize = 8 + PlayerProfile::INIT_SPACE;
pub const PLANET_STATE_SPACE: usize = 8 + PlanetState::INIT_SPACE;
pub const AUTHORIZED_VAULT_SPACE: usize = 8 + AuthorizedVault::INIT_SPACE;
pub const VAULT_BACKUP_SPACE: usize = 8 + VaultBackup::INIT_SPACE;

// =============================================
// Helper Functions
// =============================================

fn validate_coordinates(galaxy: u16, system: u16, position: u8) -> Result<()> {
    require!((1..=9).contains(&galaxy), GameStateError::InvalidCoordinates);
    require!((1..=499).contains(&system), GameStateError::InvalidCoordinates);
    require!((1..=15).contains(&position), GameStateError::InvalidCoordinates);
    Ok(())
}

fn copy_name<const N: usize>(value: &str, fallback: &str) -> [u8; N] {
    let source = if value.is_empty() { fallback } else { value };
    let bytes = source.as_bytes();
    let mut out = [0u8; N];
    let copy_len = bytes.len().min(N);
    out[..copy_len].copy_from_slice(&bytes[..copy_len]);
    out
}

fn pow15(n: u64) -> u64 {
    let mut r: u64 = 1_000;
    for _ in 0..n { r = r * 3 / 2; }
    r
}

fn base_cost(idx: u8) -> (u32, u32, u32) {
    match idx {
        0 => (60, 15, 0), 1 => (48, 24, 0), 2 => (225, 75, 0), 3 => (75, 30, 0),
        4 => (900, 360, 900), 5 => (400, 120, 200), 6 => (1_000_000, 500_000, 100_000),
        7 => (400, 200, 100), 8 => (1000, 0, 0), 9 => (1000, 500, 0),
        10 => (1000, 1000, 0), 11 => (200, 400, 200), 12 => (20, 20, 0),
        _ => (0, 0, 0),
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

fn research_base_cost(idx: u8) -> (u64, u64, u64) {
    match idx {
        0 => (0, 800, 400), 1 => (400, 0, 600), 2 => (2000, 4000, 600),
        3 => (10000, 20000, 6000), 4 => (0, 400, 600), 5 => (4000, 2000, 1000),
        6 => (240000, 400000, 160000), _ => (0, 0, 0),
    }
}

fn research_lab_requirement(idx: u8) -> u8 {
    match idx {
        0 | 1 | 4 => 1, 5 => 3, 2 => 5, 3 => 7, 6 => 10, _ => 255,
    }
}

fn pow2(level: u8) -> u64 {
    1u64.checked_shl(level as u32).unwrap_or(u64::MAX)
}

fn research_cost_for_level(idx: u8, current: u8) -> (u64, u64, u64) {
    let (m, c, d) = research_base_cost(idx);
    let mult = pow2(current);
    (m.saturating_mul(mult), c.saturating_mul(mult), d.saturating_mul(mult))
}

fn research_seconds(next_level: u8, lab_level: u8) -> i64 {
    ((next_level as u64 * 1800) / (lab_level.max(1) as u64)).max(1) as i64
}

fn ship_cost(ship_type: u8) -> (u64, u64, u64) {
    match ship_type {
        0 => (2000, 2000, 0), 1 => (6000, 6000, 0), 2 => (3000, 1000, 0),
        3 => (6000, 4000, 0), 4 => (20000, 7000, 2000), 5 => (45000, 15000, 0),
        6 => (30000, 40000, 15000), 7 => (50000, 25000, 15000),
        8 => (60000, 50000, 15000), 9 => (5000000, 4000000, 1000000),
        10 => (10000, 6000, 2000), 11 => (0, 1000, 0),
        12 => (10000, 20000, 10000), 13 => (0, 2000, 500),
        _ => (0, 0, 0),
    }
}

fn enforce_ship_research_gate(ship_type: u8, planet: &PlanetState) -> Result<()> {
    match ship_type {
        0 => require!(planet.combustion_drive >= 2, GameStateError::TechLocked),
        1 => require!(planet.combustion_drive >= 6, GameStateError::TechLocked),
        12 => require!(planet.impulse_drive >= 3 && planet.astrophysics >= 4, GameStateError::TechLocked),
        _ => {}
    }
    Ok(())
}

fn cargo_capacity(sc: u32, lc: u32, rec: u32, cr: u32, bs: u32) -> u64 {
    sc as u64 * 5_000 + lc as u64 * 25_000 + rec as u64 * 20_000 + cr as u64 * 800 + bs as u64 * 1_500
}

fn launch_fuel_cost(
    lf: u32, hf: u32, cr: u32, bs: u32, bc: u32, bm: u32, ds: u32, de: u32,
    sc: u32, lc: u32, rec: u32, ep: u32, col: u32, speed_factor: u8,
) -> u64 {
    (sc as u64 * 10 + lc as u64 * 50 + lf as u64 * 20 + hf as u64 * 75 +
     cr as u64 * 300 + bs as u64 * 500 + bc as u64 * 250 + bm as u64 * 1_000 +
     ds as u64 * 1_000 + rec as u64 * 300 + ep as u64 + col as u64 * 1_000)
        * (speed_factor as u64).pow(2) / 10_000
}

fn mine_rate(level: u8, base: u64) -> u64 {
    if level == 0 { return 0; }
    base * (level as u64) * 11u64.pow(level as u32) / 10u64.pow(level as u32)
}

fn store_cap(level: u8) -> u64 {
    if level == 0 { 1_000_000 } else { 1_000_000 * 2u64.pow(level as u32) }
}

fn settle_resources(planet: &mut PlanetState, now: i64) {
    if planet.last_update_ts <= 0 || now <= planet.last_update_ts {
        planet.last_update_ts = now;
        return;
    }
    let dt = (now - planet.last_update_ts) as u64;
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
    planet.deuterium = add_res(planet.deuterium, planet.deuterium_hour, planet.deuterium_cap);
    planet.last_update_ts = now;
}

fn recalculate_rates(planet: &mut PlanetState) {
    planet.metal_hour = mine_rate(planet.metal_mine, 30);
    planet.crystal_hour = mine_rate(planet.crystal_mine, 20);

    let temp_factor = (240i32 - planet.temperature as i32).max(0) as u64;
    planet.deuterium_hour = if planet.deuterium_synthesizer == 0 {
        0
    } else {
        mine_rate(planet.deuterium_synthesizer, 10) * temp_factor / 200
    };

    let solar_prod = mine_rate(planet.solar_plant, 20);
    let fusion_prod = if planet.fusion_reactor == 0 {
        0
    } else {
        mine_rate(planet.fusion_reactor, 30) * 180 / 100
    };

    planet.energy_production = solar_prod + fusion_prod;
    planet.energy_consumption = mine_rate(planet.metal_mine, 10)
        + mine_rate(planet.crystal_mine, 10)
        + mine_rate(planet.deuterium_synthesizer, 20);

    planet.metal_cap = store_cap(planet.metal_storage);
    planet.crystal_cap = store_cap(planet.crystal_storage);
    planet.deuterium_cap = store_cap(planet.deuterium_tank);
}

fn require_active_vault(
    vault_signer: Pubkey,
    authorized_vault: &Account<AuthorizedVault>,
    planet_authority: Pubkey,
) -> Result<()> {
    require_keys_eq!(authorized_vault.vault, vault_signer, GameStateError::InvalidVaultAuthorization);
    require_keys_eq!(authorized_vault.authority, planet_authority, GameStateError::InvalidVaultAuthorization);
    require!(!authorized_vault.revoked, GameStateError::VaultAuthorizationRevoked);

    if authorized_vault.expires_at > 0 {
        require!(
            Clock::get()?.unix_timestamp <= authorized_vault.expires_at,
            GameStateError::VaultAuthorizationExpired
        );
    }
    Ok(())
}

fn create_planet_state(
    authority: Pubkey,
    player_profile: &mut Account<PlayerProfile>,
    planet_state: &mut Account<PlanetState>,
    bump: u8,
    params: &InitializePlanetParams,
) -> Result<()> {
    validate_coordinates(params.galaxy, params.system, params.position)?;
    require_keys_eq!(player_profile.authority, authority, GameStateError::Unauthorized);

    let planet_index = player_profile.planet_count;
    player_profile.planet_count = player_profile
        .planet_count
        .checked_add(1)
        .ok_or(GameStateError::PlanetCountOverflow)?;

    planet_state.set_inner(PlanetState {
        authority,
        player: player_profile.key(),
        planet_index,
        galaxy: params.galaxy,
        system: params.system,
        position: params.position,
        name: copy_name::<MAX_PLANET_NAME_LEN>(&params.name, "Planet"),
        diameter: params.diameter,
        temperature: params.temperature,
        max_fields: params.max_fields,
        used_fields: params.used_fields,
        metal_mine: params.metal_mine,
        crystal_mine: params.crystal_mine,
        deuterium_synthesizer: params.deuterium_synthesizer,
        solar_plant: params.solar_plant,
        fusion_reactor: params.fusion_reactor,
        robotics_factory: params.robotics_factory,
        nanite_factory: params.nanite_factory,
        shipyard: params.shipyard,
        metal_storage: params.metal_storage,
        crystal_storage: params.crystal_storage,
        deuterium_tank: params.deuterium_tank,
        research_lab: params.research_lab,
        missile_silo: params.missile_silo,
        energy_tech: params.energy_tech,
        combustion_drive: params.combustion_drive,
        impulse_drive: params.impulse_drive,
        hyperspace_drive: params.hyperspace_drive,
        computer_tech: params.computer_tech,
        astrophysics: params.astrophysics,
        igr_network: params.igr_network,
        research_queue_item: params.research_queue_item,
        research_queue_target: params.research_queue_target,
        research_finish_ts: params.research_finish_ts,
        build_queue_item: params.build_queue_item,
        build_queue_target: params.build_queue_target,
        build_finish_ts: params.build_finish_ts,
        metal: params.metal,
        crystal: params.crystal,
        deuterium: params.deuterium,
        metal_hour: params.metal_hour,
        crystal_hour: params.crystal_hour,
        deuterium_hour: params.deuterium_hour,
        energy_production: params.energy_production,
        energy_consumption: params.energy_consumption,
        metal_cap: params.metal_cap,
        crystal_cap: params.crystal_cap,
        deuterium_cap: params.deuterium_cap,
        last_update_ts: params.last_update_ts,
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
        solar_satellite: params.solar_satellite,
        active_missions: 0,
        missions: [MissionState::default(); MAX_MISSIONS],
        bump,
    });

    Ok(())
}

fn produce_planet(planet: &mut PlanetState, now: i64) -> Result<()> {
    settle_resources(planet, now);
    Ok(())
}

fn start_build_planet(planet: &mut PlanetState, building_idx: u8, now: i64) -> Result<()> {
    settle_resources(planet, now);
    let current = planet.building_level(building_idx);
    let next = current.saturating_add(1);
    let (cm, cc, cd) = upgrade_cost(building_idx, next as u64);

    require!(planet.build_finish_ts == 0 || now >= planet.build_finish_ts, GameStateError::QueueBusy);
    require!(planet.used_fields < planet.max_fields, GameStateError::NoFields);
    require!(planet.metal >= cm, GameStateError::InsufficientMetal);
    require!(planet.crystal >= cc, GameStateError::InsufficientCrystal);
    require!(planet.deuterium >= cd, GameStateError::InsufficientDeuterium);

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

fn finish_build_planet(planet: &mut PlanetState, now: i64) -> Result<()> {
    settle_resources(planet, now);
    require!(planet.build_finish_ts > 0, GameStateError::NoBuild);
    require!(now >= planet.build_finish_ts, GameStateError::BuildNotFinished);

    let idx = planet.build_queue_item;
    let level = planet.build_queue_target;
    planet.set_building_level(idx, level);
    recalculate_rates(planet);

    planet.build_queue_item = 255;
    planet.build_queue_target = 0;
    planet.build_finish_ts = 0;
    Ok(())
}

fn start_research_planet(planet: &mut PlanetState, tech_idx: u8, now: i64) -> Result<()> {
    settle_resources(planet, now);
    require!(tech_idx <= 6, GameStateError::InvalidTech);
    require!(planet.research_lab >= 1, GameStateError::LabTooLow);
    require!(planet.research_queue_item == 255, GameStateError::ResearchQueueBusy);

    let lab_req = research_lab_requirement(tech_idx);
    require!(planet.research_lab >= lab_req, GameStateError::LabTooLow);

    let current = planet.research_level(tech_idx);
    let next = current.saturating_add(1);
    let (cm, cc, cd) = research_cost_for_level(tech_idx, current);

    require!(planet.metal >= cm, GameStateError::InsufficientMetal);
    require!(planet.crystal >= cc, GameStateError::InsufficientCrystal);
    require!(planet.deuterium >= cd, GameStateError::InsufficientDeuterium);

    planet.metal -= cm;
    planet.crystal -= cc;
    planet.deuterium -= cd;

    planet.research_queue_item = tech_idx;
    planet.research_queue_target = next;
    planet.research_finish_ts = now + research_seconds(next, planet.research_lab);
    Ok(())
}

fn finish_research_planet(planet: &mut PlanetState, now: i64) -> Result<()> {
    settle_resources(planet, now);
    require!(planet.research_queue_item != 255, GameStateError::NoResearch);
    require!(now >= planet.research_finish_ts, GameStateError::ResearchNotFinished);

    let idx = planet.research_queue_item;
    let target = planet.research_queue_target;
    planet.set_research_level(idx, target);

    planet.research_queue_item = 255;
    planet.research_queue_target = 0;
    planet.research_finish_ts = 0;
    Ok(())
}

fn distance(
    from_galaxy: u16,
    from_system: u16,
    from_position: u8,
    to_galaxy: u16,
    to_system: u16,
    to_position: u8,
) -> u64 {
    if from_galaxy != to_galaxy {
        return (from_galaxy as i64 - to_galaxy as i64).abs() as u64 * 20_000;
    }

    if from_system != to_system {
        return (from_system as i64 - to_system as i64).abs() as u64 * 2_000;
    }

    return (from_position as i64 - to_position as i64).abs() as u64 * 200 + 1_000;
}

fn mission_flight_seconds(distance: u64, speed_factor: u8) -> i64 {
    let sf = speed_factor.clamp(10, 100) as u64;
    ((distance * 100) / sf).max(1) as i64
}


fn build_ship_planet(planet: &mut PlanetState, ship_type: u8, quantity: u32, now: i64) -> Result<()> {
    require!(quantity > 0, GameStateError::InvalidArgs);
    settle_resources(planet, now);
    require!(planet.shipyard >= 1, GameStateError::ShipyardTooLow);
    enforce_ship_research_gate(ship_type, planet)?;

    let (cm, cc, cd) = ship_cost(ship_type);
    require!(cm != 0 || cc != 0 || cd != 0 || ship_type == 11, GameStateError::InvalidShipType);

    let total_m = cm.saturating_mul(quantity as u64);
    let total_c = cc.saturating_mul(quantity as u64);
    let total_d = cd.saturating_mul(quantity as u64);

    require!(planet.metal >= total_m, GameStateError::InsufficientMetal);
    require!(planet.crystal >= total_c, GameStateError::InsufficientCrystal);
    require!(planet.deuterium >= total_d, GameStateError::InsufficientDeuterium);

    planet.metal -= total_m;
    planet.crystal -= total_c;
    planet.deuterium -= total_d;
    planet.add_ship(ship_type, quantity)?;
    Ok(())
}

fn launch_fleet_planet(planet: &mut PlanetState, params: LaunchFleetParams) -> Result<()> {
    validate_coordinates(params.target_galaxy, params.target_system, params.target_position)?;
    require!(
        params.mission_type == MISSION_TRANSPORT || params.mission_type == MISSION_COLONIZE,
        GameStateError::InvalidMission
    );

    let total_ships =
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
        + params.colony_ship;

    require!(total_ships > 0, GameStateError::EmptyFleet);

    settle_resources(planet, params.now);
    let slot = planet
        .free_mission_slot()
        .ok_or(GameStateError::NoMissionSlot)?;

    require!(planet.light_fighter >= params.light_fighter, GameStateError::InsufficientShips);
    require!(planet.heavy_fighter >= params.heavy_fighter, GameStateError::InsufficientShips);
    require!(planet.cruiser >= params.cruiser, GameStateError::InsufficientShips);
    require!(planet.battleship >= params.battleship, GameStateError::InsufficientShips);
    require!(planet.battlecruiser >= params.battlecruiser, GameStateError::InsufficientShips);
    require!(planet.bomber >= params.bomber, GameStateError::InsufficientShips);
    require!(planet.destroyer >= params.destroyer, GameStateError::InsufficientShips);
    require!(planet.deathstar >= params.deathstar, GameStateError::InsufficientShips);
    require!(planet.small_cargo >= params.small_cargo, GameStateError::InsufficientShips);
    require!(planet.large_cargo >= params.large_cargo, GameStateError::InsufficientShips);
    require!(planet.recycler >= params.recycler, GameStateError::InsufficientShips);
    require!(planet.espionage_probe >= params.espionage_probe, GameStateError::InsufficientShips);
    require!(planet.colony_ship >= params.colony_ship, GameStateError::InsufficientShips);

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

    require!(planet.metal >= params.cargo_metal, GameStateError::InsufficientResources);
    require!(planet.crystal >= params.cargo_crystal, GameStateError::InsufficientResources);
    require!(
        planet.deuterium >= params.cargo_deuterium,
        GameStateError::InsufficientResources
    );

    let speed_factor = params.speed_factor.clamp(10, 100);

    let dist = distance(
        planet.galaxy,
        planet.system,
        planet.position,
        params.target_galaxy,
        params.target_system,
        params.target_position,
    );

    let flight_seconds = mission_flight_seconds(dist, speed_factor);
    require!(flight_seconds > 0, GameStateError::InvalidArgs);

    // One-way fuel only at launch.
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

    let arrive_ts = params.now.saturating_add(flight_seconds);

    let return_ts = if params.mission_type == MISSION_TRANSPORT {
        arrive_ts.saturating_add(flight_seconds)
    } else {
        0
    };

    planet.set_mission(slot, MissionState {
        mission_type: params.mission_type,
        target_galaxy: params.target_galaxy,
        target_system: params.target_system,
        target_position: params.target_position,
        colony_name: copy_name::<MAX_MISSION_COLONY_NAME_LEN>(&params.colony_name, ""),
        depart_ts: params.now,
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
        speed_factor, // <-- add this field to MissionState
    });

    planet.active_missions = planet.active_missions.saturating_add(1);
    Ok(())
}

fn resolve_transport_planets(
    source: &mut PlanetState,
    destination: &mut PlanetState,
    slot: usize,
    now: i64,
) -> Result<()> {
    require!(slot < MAX_MISSIONS, GameStateError::InvalidMissionSlot);

    let mission = source.mission(slot);
    require!(mission.mission_type == MISSION_TRANSPORT, GameStateError::InvalidMission);
    require!(
        mission.target_galaxy == destination.galaxy
            && mission.target_system == destination.system
            && mission.target_position == destination.position,
        GameStateError::InvalidDestination
    );

    if !mission.applied {
        require!(now >= mission.arrive_ts, GameStateError::MissionInFlight);

        settle_resources(source, now);
        settle_resources(destination, now);

        // Always deliver cargo on arrival.
        destination.metal = destination.metal.saturating_add(mission.cargo_metal);
        destination.crystal = destination.crystal.saturating_add(mission.cargo_crystal);
        destination.deuterium = destination.deuterium.saturating_add(mission.cargo_deuterium);

        if destination.authority == source.authority {
            // Same owner: ships stay at destination and mission ends here.
            destination.small_cargo = destination.small_cargo.saturating_add(mission.small_cargo);
            destination.large_cargo = destination.large_cargo.saturating_add(mission.large_cargo);
            destination.light_fighter = destination.light_fighter.saturating_add(mission.light_fighter);
            destination.heavy_fighter = destination.heavy_fighter.saturating_add(mission.heavy_fighter);
            destination.cruiser = destination.cruiser.saturating_add(mission.cruiser);
            destination.battleship = destination.battleship.saturating_add(mission.battleship);
            destination.battlecruiser = destination.battlecruiser.saturating_add(mission.battlecruiser);
            destination.bomber = destination.bomber.saturating_add(mission.bomber);
            destination.destroyer = destination.destroyer.saturating_add(mission.destroyer);
            destination.deathstar = destination.deathstar.saturating_add(mission.deathstar);
            destination.recycler = destination.recycler.saturating_add(mission.recycler);
            destination.espionage_probe = destination.espionage_probe.saturating_add(mission.espionage_probe);
            destination.colony_ship = destination.colony_ship.saturating_add(mission.colony_ship);

            source.clear_mission(slot);
            source.active_missions = source.active_missions.saturating_sub(1);
            return Ok(());
        }

        // Different owner: deduct extra return fuel now from source planet.
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

        // Ships return empty.
        source.missions[slot].cargo_metal = 0;
        source.missions[slot].cargo_crystal = 0;
        source.missions[slot].cargo_deuterium = 0;
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

fn resolve_colonize_planet(
    source: &mut PlanetState,
    player_profile: &mut Account<PlayerProfile>,
    colony_planet: &mut Account<PlanetState>,
    colony_bump: u8,
    slot: usize,
    now: i64,
) -> Result<()> {
    require!(slot < MAX_MISSIONS, GameStateError::InvalidMissionSlot);

    let mission = source.mission(slot);
    require!(mission.mission_type == MISSION_COLONIZE, GameStateError::InvalidMission);
    require!(!mission.applied, GameStateError::AlreadyResolved);
    require!(now >= mission.arrive_ts, GameStateError::MissionInFlight);
    require!(mission.colony_ship > 0, GameStateError::MissingColonyShip);

    validate_coordinates(
        mission.target_galaxy,
        mission.target_system,
        mission.target_position,
    )?;

    let coords_taken = source.galaxy == mission.target_galaxy
        && source.system == mission.target_system
        && source.position == mission.target_position;
    require!(!coords_taken, GameStateError::InvalidDestination);

    let params = InitializePlanetParams {
        name: String::from_utf8_lossy(&mission.colony_name)
            .trim_matches(char::from(0))
            .trim()
            .to_string(),
        galaxy: mission.target_galaxy,
        system: mission.target_system,
        position: mission.target_position,

        diameter: 8_000u32
            + ((mission.target_galaxy as u32 * 997
                + mission.target_system as u32 * 37
                + mission.target_position as u32 * 101)
                % 10_000),
        temperature: (120i16 - (mission.target_position as i16 * 12)).clamp(-60, 120),
        max_fields: 163u16
            + ((mission.target_galaxy + mission.target_system + mission.target_position as u16) % 40),

        used_fields: 3,
        metal_mine: 1,
        crystal_mine: 1,
        deuterium_synthesizer: 1,
        solar_plant: 1,
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
        combustion_drive: 0,
        impulse_drive: 0,
        hyperspace_drive: 0,
        computer_tech: 0,
        astrophysics: 0,
        igr_network: 0,

        research_queue_item: 255,
        research_queue_target: 0,
        research_finish_ts: 0,
        build_queue_item: 255,
        build_queue_target: 0,
        build_finish_ts: 0,

        metal: mission.cargo_metal,
        crystal: mission.cargo_crystal,
        deuterium: mission.cargo_deuterium,

        metal_hour: 33,
        crystal_hour: 22,
        deuterium_hour: 14,
        energy_production: 22,
        energy_consumption: 42,
        metal_cap: 1_000_000,
        crystal_cap: 1_000_000,
        deuterium_cap: 1_000_000,
        last_update_ts: now,

        small_cargo: mission.small_cargo,
        large_cargo: mission.large_cargo,
        light_fighter: mission.light_fighter,
        heavy_fighter: mission.heavy_fighter,
        cruiser: mission.cruiser,
        battleship: mission.battleship,
        battlecruiser: mission.battlecruiser,
        bomber: mission.bomber,
        destroyer: mission.destroyer,
        deathstar: mission.deathstar,
        recycler: mission.recycler,
        espionage_probe: mission.espionage_probe,

        // consumed to found the colony
        colony_ship: 0,
        solar_satellite: 0,
    };

    let authority = source.authority;
    create_planet_state(
        authority,
        player_profile,
        colony_planet,
        colony_bump,
        &params,
    )?;

    source.clear_mission(slot);
    source.active_missions = source.active_missions.saturating_sub(1);

    Ok(())
}

// =============================================
// Program
// =============================================

#[program]
pub mod game_state {
    use super::*;

    /// One-time wallet setup: creates player profile + authorizes vault + stores encrypted backup.
    /// After this, the wallet never needs to sign again unless explicitly managing the vault.
    pub fn initialize_player(
        ctx: Context<InitializePlayer>,
        vault: Pubkey,
        expires_at: i64,
        backup_version: u8,
        backup_ciphertext: Vec<u8>,
        backup_iv: [u8; 12],
        backup_salt: [u8; 16],
        backup_kdf_salt: [u8; 16],
    ) -> Result<()> {
        require!(!backup_ciphertext.is_empty(), GameStateError::InvalidArgs);
        require!(backup_ciphertext.len() <= 512, GameStateError::BackupTooLarge);

        let authority = ctx.accounts.authority.key();

        ctx.accounts.player_profile.set_inner(PlayerProfile {
            authority,
            planet_count: 0,
            bump: ctx.bumps.player_profile,
        });

        let now = Clock::get()?.unix_timestamp;
        require!(expires_at == 0 || expires_at > now, GameStateError::InvalidArgs);

        ctx.accounts.authorized_vault.set_inner(AuthorizedVault {
            authority,
            vault,
            expires_at,
            revoked: false,
            bump: ctx.bumps.authorized_vault,
        });

        ctx.accounts.vault_backup.set_inner(VaultBackup {
            authority,
            vault,
            version: backup_version,
            ciphertext: backup_ciphertext,
            iv: backup_iv,
            salt: backup_salt,
            kdf_salt: backup_kdf_salt,
            updated_at: now,
            bump: ctx.bumps.vault_backup,
        });

        Ok(())
    }

    /// Wallet-only: rotate vault key and update backup. Use when recovering on a new device.
    pub fn rotate_vault(
        ctx: Context<RotateVault>,
        new_vault: Pubkey,
        expires_at: i64,
        backup_version: u8,
        backup_ciphertext: Vec<u8>,
        backup_iv: [u8; 12],
        backup_salt: [u8; 16],
        backup_kdf_salt: [u8; 16],
    ) -> Result<()> {
        require!(!backup_ciphertext.is_empty(), GameStateError::InvalidArgs);
        require!(backup_ciphertext.len() <= 512, GameStateError::BackupTooLarge);

        let now = Clock::get()?.unix_timestamp;
        require!(expires_at == 0 || expires_at > now, GameStateError::InvalidArgs);

        ctx.accounts.authorized_vault.vault = new_vault;
        ctx.accounts.authorized_vault.expires_at = expires_at;
        ctx.accounts.authorized_vault.revoked = false;

        ctx.accounts.vault_backup.vault = new_vault;
        ctx.accounts.vault_backup.version = backup_version;
        ctx.accounts.vault_backup.ciphertext = backup_ciphertext;
        ctx.accounts.vault_backup.iv = backup_iv;
        ctx.accounts.vault_backup.salt = backup_salt;
        ctx.accounts.vault_backup.kdf_salt = backup_kdf_salt;
        ctx.accounts.vault_backup.updated_at = now;

        Ok(())
    }

    /// Wallet-only: revoke vault access (emergency lockout).
    pub fn revoke_vault(ctx: Context<ManageVault>) -> Result<()> {
        ctx.accounts.authorized_vault.revoked = true;
        Ok(())
    }

    /// Wallet-only: extend vault expiry.
    pub fn extend_vault(ctx: Context<ManageVault>, expires_at: i64) -> Result<()> {
        let now = Clock::get()?.unix_timestamp;
        require!(expires_at == 0 || expires_at > now, GameStateError::InvalidArgs);
        ctx.accounts.authorized_vault.expires_at = expires_at;
        ctx.accounts.authorized_vault.revoked = false;
        Ok(())
    }

    /// Vault-signed: initialize homeworld. Vault pays rent — no wallet popup.
    pub fn initialize_homeworld(
        ctx: Context<InitializePlanetVault>,
        params: InitializeHomeworldParams,
    ) -> Result<()> {
        require_active_vault(
            ctx.accounts.vault_signer.key(),
            &ctx.accounts.authorized_vault,
            ctx.accounts.player_profile.authority,
        )?;

        let authority = ctx.accounts.player_profile.authority;
        let auth_bytes = authority.to_bytes();
        let position = if params.position == 0 {
            (auth_bytes[3] % 15) + 1
        } else {
            params.position.clamp(1, 15)
        };
        let base_temp = 120i16 - (position as i16 * 12);

        let planet_params = InitializePlanetParams {
            name: if params.name.is_empty() { "Homeworld".to_string() } else { params.name },
            galaxy: if params.galaxy == 0 { ((auth_bytes[0] as u16) % 9) + 1 } else { params.galaxy.clamp(1, 9) },
            system: if params.system == 0 { (u16::from_le_bytes([auth_bytes[1], auth_bytes[2]]) % 499) + 1 } else { params.system.clamp(1, 499) },
            position,
            diameter: 8_000u32 + (u16::from_le_bytes([auth_bytes[4], auth_bytes[5]]) as u32 % 10_000),
            temperature: (base_temp + ((auth_bytes[6] as i16) % 40 - 20)).clamp(-60, 120),
            max_fields: 163u16 + (auth_bytes[7] as u16 % 40),
            used_fields: 3,
            metal_mine: 1, crystal_mine: 1, deuterium_synthesizer: 1, solar_plant: 1,
            fusion_reactor: 0, robotics_factory: 0, nanite_factory: 0, shipyard: 0,
            metal_storage: 0, crystal_storage: 0, deuterium_tank: 0, research_lab: 0, missile_silo: 0,
            energy_tech: 0, combustion_drive: 0, impulse_drive: 0, hyperspace_drive: 0,
            computer_tech: 0, astrophysics: 0, igr_network: 0,
            research_queue_item: 255, research_queue_target: 0, research_finish_ts: 0,
            build_queue_item: 255, build_queue_target: 0, build_finish_ts: 0,
            metal: 1_000_000, crystal: 1_000_000, deuterium: 1_000_000,
            metal_hour: 33, crystal_hour: 22, deuterium_hour: 14,
            energy_production: 22, energy_consumption: 42,
            metal_cap: 1_000_000, crystal_cap: 1_000_000, deuterium_cap: 1_000_000,
            last_update_ts: params.now,
            small_cargo: 0, large_cargo: 0, light_fighter: 0, heavy_fighter: 0,
            cruiser: 0, battleship: 0, battlecruiser: 0, bomber: 0, destroyer: 0,
            deathstar: 0, recycler: 0, espionage_probe: 0, colony_ship: 0, solar_satellite: 0,
        };

        create_planet_state(
            authority,
            &mut ctx.accounts.player_profile,
            &mut ctx.accounts.planet_state,
            ctx.bumps.planet_state,
            &planet_params,
        )
    }

    /// Vault-signed: initialize colony. Vault pays rent — no wallet popup.
    pub fn initialize_colony(
        ctx: Context<InitializePlanetVault>,
        params: InitializeColonyParams,
    ) -> Result<()> {
        require_active_vault(
            ctx.accounts.vault_signer.key(),
            &ctx.accounts.authorized_vault,
            ctx.accounts.player_profile.authority,
        )?;

        let authority = ctx.accounts.player_profile.authority;

        let planet_params = InitializePlanetParams {
            name: if params.name.is_empty() { "Colony".to_string() } else { params.name },
            galaxy: params.galaxy,
            system: params.system,
            position: params.position,
            diameter: 8_000u32 + ((params.galaxy as u32 * 997 + params.system as u32 * 37 + params.position as u32 * 101) % 10_000),
            temperature: (120i16 - (params.position as i16 * 12)).clamp(-60, 120),
            max_fields: 163u16 + ((params.galaxy + params.system + params.position as u16) % 40),
            used_fields: 3,
            metal_mine: 1, crystal_mine: 1, deuterium_synthesizer: 1, solar_plant: 1,
            fusion_reactor: 0, robotics_factory: 0, nanite_factory: 0, shipyard: 0,
            metal_storage: 0, crystal_storage: 0, deuterium_tank: 0, research_lab: 0, missile_silo: 0,
            energy_tech: 0, combustion_drive: 0, impulse_drive: 0, hyperspace_drive: 0,
            computer_tech: 0, astrophysics: 0, igr_network: 0,
            research_queue_item: 255, research_queue_target: 0, research_finish_ts: 0,
            build_queue_item: 255, build_queue_target: 0, build_finish_ts: 0,
            metal: params.cargo_metal, crystal: params.cargo_crystal, deuterium: params.cargo_deuterium,
            metal_hour: 33, crystal_hour: 22, deuterium_hour: 14,
            energy_production: 22, energy_consumption: 42,
            metal_cap: 1_000_000, crystal_cap: 1_000_000, deuterium_cap: 1_000_000,
            last_update_ts: params.now,
            small_cargo: params.small_cargo, large_cargo: params.large_cargo,
            light_fighter: params.light_fighter, heavy_fighter: params.heavy_fighter,
            cruiser: params.cruiser, battleship: params.battleship, battlecruiser: params.battlecruiser,
            bomber: params.bomber, destroyer: params.destroyer, deathstar: params.deathstar,
            recycler: params.recycler, espionage_probe: params.espionage_probe,
            colony_ship: 0, solar_satellite: params.solar_satellite,
        };

        create_planet_state(
            authority,
            &mut ctx.accounts.player_profile,
            &mut ctx.accounts.planet_state,
            ctx.bumps.planet_state,
            &planet_params,
        )
    }

    pub fn produce(ctx: Context<MutatePlanetState>, now: i64) -> Result<()> {
        produce_planet(&mut ctx.accounts.planet_state, now)
    }

    pub fn produce_vault(ctx: Context<MutatePlanetStateVault>, now: i64) -> Result<()> {
        require_active_vault(ctx.accounts.vault_signer.key(), &ctx.accounts.authorized_vault, ctx.accounts.planet_state.authority)?;
        produce_planet(&mut ctx.accounts.planet_state, now)
    }

    pub fn start_build(ctx: Context<MutatePlanetState>, building_idx: u8, now: i64) -> Result<()> {
        start_build_planet(&mut ctx.accounts.planet_state, building_idx, now)
    }

    pub fn start_build_vault(ctx: Context<MutatePlanetStateVault>, building_idx: u8, now: i64) -> Result<()> {
        require_active_vault(ctx.accounts.vault_signer.key(), &ctx.accounts.authorized_vault, ctx.accounts.planet_state.authority)?;
        start_build_planet(&mut ctx.accounts.planet_state, building_idx, now)
    }

    pub fn finish_build(ctx: Context<MutatePlanetState>, now: i64) -> Result<()> {
        finish_build_planet(&mut ctx.accounts.planet_state, now)
    }

    pub fn finish_build_vault(ctx: Context<MutatePlanetStateVault>, now: i64) -> Result<()> {
        require_active_vault(ctx.accounts.vault_signer.key(), &ctx.accounts.authorized_vault, ctx.accounts.planet_state.authority)?;
        finish_build_planet(&mut ctx.accounts.planet_state, now)
    }

    pub fn start_research(ctx: Context<MutatePlanetState>, tech_idx: u8, now: i64) -> Result<()> {
        start_research_planet(&mut ctx.accounts.planet_state, tech_idx, now)
    }

    pub fn start_research_vault(ctx: Context<MutatePlanetStateVault>, tech_idx: u8, now: i64) -> Result<()> {
        require_active_vault(ctx.accounts.vault_signer.key(), &ctx.accounts.authorized_vault, ctx.accounts.planet_state.authority)?;
        start_research_planet(&mut ctx.accounts.planet_state, tech_idx, now)
    }

    pub fn finish_research(ctx: Context<MutatePlanetState>, now: i64) -> Result<()> {
        finish_research_planet(&mut ctx.accounts.planet_state, now)
    }

    pub fn finish_research_vault(ctx: Context<MutatePlanetStateVault>, now: i64) -> Result<()> {
        require_active_vault(ctx.accounts.vault_signer.key(), &ctx.accounts.authorized_vault, ctx.accounts.planet_state.authority)?;
        finish_research_planet(&mut ctx.accounts.planet_state, now)
    }

    pub fn build_ship(ctx: Context<MutatePlanetState>, ship_type: u8, quantity: u32, now: i64) -> Result<()> {
        build_ship_planet(&mut ctx.accounts.planet_state, ship_type, quantity, now)
    }

    pub fn build_ship_vault(ctx: Context<MutatePlanetStateVault>, ship_type: u8, quantity: u32, now: i64) -> Result<()> {
        require_active_vault(ctx.accounts.vault_signer.key(), &ctx.accounts.authorized_vault, ctx.accounts.planet_state.authority)?;
        build_ship_planet(&mut ctx.accounts.planet_state, ship_type, quantity, now)
    }

    pub fn launch_fleet(ctx: Context<MutatePlanetState>, params: LaunchFleetParams) -> Result<()> {
        launch_fleet_planet(&mut ctx.accounts.planet_state, params)
    }

    pub fn launch_fleet_vault(ctx: Context<MutatePlanetStateVault>, params: LaunchFleetParams) -> Result<()> {
        require_active_vault(ctx.accounts.vault_signer.key(), &ctx.accounts.authorized_vault, ctx.accounts.planet_state.authority)?;
        launch_fleet_planet(&mut ctx.accounts.planet_state, params)
    }

    pub fn resolve_transport(ctx: Context<ResolveTransport>, slot: u8, now: i64) -> Result<()> {
        resolve_transport_planets(&mut ctx.accounts.source_planet, &mut ctx.accounts.destination_planet, slot as usize, now)
    }

    pub fn resolve_transport_vault(ctx: Context<ResolveTransportVault>, slot: u8, now: i64) -> Result<()> {
        require_active_vault(ctx.accounts.vault_signer.key(), &ctx.accounts.authorized_vault, ctx.accounts.source_planet.authority)?;
        resolve_transport_planets(&mut ctx.accounts.source_planet, &mut ctx.accounts.destination_planet, slot as usize, now)
    }

pub fn resolve_colonize(ctx: Context<ResolveColonize>, slot: u8, now: i64) -> Result<()> {
    resolve_colonize_planet(
        &mut ctx.accounts.source_planet,
        &mut ctx.accounts.player_profile,
        &mut ctx.accounts.colony_planet,
        ctx.bumps.colony_planet,
        slot as usize,
        now,
    )
}

pub fn resolve_colonize_vault(
    ctx: Context<ResolveColonizeVault>,
    slot: u8,
    now: i64,
) -> Result<()> {
    require_active_vault(
        ctx.accounts.vault_signer.key(),
        &ctx.accounts.authorized_vault,
        ctx.accounts.source_planet.authority,
    )?;

    resolve_colonize_planet(
        &mut ctx.accounts.source_planet,
        &mut ctx.accounts.player_profile,
        &mut ctx.accounts.colony_planet,
        ctx.bumps.colony_planet,
        slot as usize,
        now,
    )
}
}

// =============================================
// Account Contexts
// =============================================

/// One-time wallet setup: creates profile, authorizes vault, stores backup — all in one tx.
#[derive(Accounts)]
pub struct InitializePlayer<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(
        init,
        payer = authority,
        space = PLAYER_PROFILE_SPACE,
        seeds = [b"player_profile", authority.key().as_ref()],
        bump
    )]
    pub player_profile: Account<'info, PlayerProfile>,
    #[account(
        init,
        payer = authority,
        space = AUTHORIZED_VAULT_SPACE,
        seeds = [b"authorized_vault", authority.key().as_ref()],
        bump
    )]
    pub authorized_vault: Account<'info, AuthorizedVault>,
    #[account(
        init,
        payer = authority,
        space = VAULT_BACKUP_SPACE,
        seeds = [b"vault_backup", authority.key().as_ref()],
        bump
    )]
    pub vault_backup: Account<'info, VaultBackup>,
    pub system_program: Program<'info, System>,
}

/// Wallet-only: rotate vault keypair and update backup simultaneously.
#[derive(Accounts)]
pub struct RotateVault<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(
        mut,
        seeds = [b"authorized_vault", authority.key().as_ref()],
        bump = authorized_vault.bump,
        has_one = authority @ GameStateError::Unauthorized
    )]
    pub authorized_vault: Account<'info, AuthorizedVault>,
    #[account(
        mut,
        seeds = [b"vault_backup", authority.key().as_ref()],
        bump = vault_backup.bump,
        has_one = authority @ GameStateError::Unauthorized
    )]
    pub vault_backup: Account<'info, VaultBackup>,
}

/// Wallet-only: revoke or extend vault.
#[derive(Accounts)]
pub struct ManageVault<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(
        mut,
        seeds = [b"authorized_vault", authority.key().as_ref()],
        bump = authorized_vault.bump,
        has_one = authority @ GameStateError::Unauthorized
    )]
    pub authorized_vault: Account<'info, AuthorizedVault>,
}

/// Vault-signed planet creation. Vault is payer — no wallet popup.
#[derive(Accounts)]
pub struct InitializePlanetVault<'info> {
    /// The vault keypair signs and pays rent.
    #[account(mut)]
    pub vault_signer: Signer<'info>,
    /// CHECK: authority is read from player_profile.authority — not a signer.
    pub authority: UncheckedAccount<'info>,
    #[account(
        seeds = [b"authorized_vault", authority.key().as_ref()],
        bump = authorized_vault.bump,
    )]
    pub authorized_vault: Account<'info, AuthorizedVault>,
    #[account(
        mut,
        seeds = [b"player_profile", authority.key().as_ref()],
        bump = player_profile.bump,
        has_one = authority @ GameStateError::Unauthorized
    )]
    pub player_profile: Account<'info, PlayerProfile>,
    #[account(
        init,
        payer = vault_signer,
        space = PLANET_STATE_SPACE,
        seeds = [b"planet_state", authority.key().as_ref(), &player_profile.planet_count.to_le_bytes()],
        bump
    )]
    pub planet_state: Account<'info, PlanetState>,
    pub system_program: Program<'info, System>,
}

/// Wallet-signed planet mutation (fallback / recovery path).
#[derive(Accounts)]
pub struct MutatePlanetState<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(
        mut,
        seeds = [b"planet_state", authority.key().as_ref(), &planet_state.planet_index.to_le_bytes()],
        bump = planet_state.bump,
        has_one = authority @ GameStateError::Unauthorized
    )]
    pub planet_state: Account<'info, PlanetState>,
}

/// Vault-signed planet mutation — normal gameplay path, no wallet popup.
#[derive(Accounts)]
pub struct MutatePlanetStateVault<'info> {
    #[account(mut)]
    pub vault_signer: Signer<'info>,
    #[account(
        seeds = [b"authorized_vault", planet_state.authority.as_ref()],
        bump = authorized_vault.bump,
    )]
    pub authorized_vault: Account<'info, AuthorizedVault>,
    #[account(mut)]
    pub planet_state: Account<'info, PlanetState>,
}

/// Wallet-signed transport resolution (fallback).
#[derive(Accounts)]
pub struct ResolveTransport<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(
        mut,
        seeds = [b"planet_state", authority.key().as_ref(), &source_planet.planet_index.to_le_bytes()],
        bump = source_planet.bump,
        has_one = authority @ GameStateError::Unauthorized
    )]
    pub source_planet: Account<'info, PlanetState>,
    #[account(mut)]
    pub destination_planet: Account<'info, PlanetState>,
}

/// Vault-signed transport resolution — no wallet popup.
#[derive(Accounts)]
pub struct ResolveTransportVault<'info> {
    #[account(mut)]
    pub vault_signer: Signer<'info>,
    #[account(
        seeds = [b"authorized_vault", source_planet.authority.as_ref()],
        bump = authorized_vault.bump,
    )]
    pub authorized_vault: Account<'info, AuthorizedVault>,
    #[account(mut)]
    pub source_planet: Account<'info, PlanetState>,
    #[account(mut)]
    pub destination_planet: Account<'info, PlanetState>,
}

#[derive(Accounts)]
pub struct ResolveColonize<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(
        mut,
        seeds = [b"player_profile", authority.key().as_ref()],
        bump = player_profile.bump,
        has_one = authority @ GameStateError::Unauthorized
    )]
    pub player_profile: Account<'info, PlayerProfile>,

    #[account(
        mut,
        seeds = [b"planet_state", authority.key().as_ref(), &source_planet.planet_index.to_le_bytes()],
        bump = source_planet.bump,
        has_one = authority @ GameStateError::Unauthorized
    )]
    pub source_planet: Account<'info, PlanetState>,

    #[account(
        init,
        payer = authority,
        space = PLANET_STATE_SPACE,
        seeds = [b"planet_state", authority.key().as_ref(), &player_profile.planet_count.to_le_bytes()],
        bump
    )]
    pub colony_planet: Account<'info, PlanetState>,

    pub system_program: Program<'info, System>,
}


#[derive(Accounts)]
pub struct ResolveColonizeVault<'info> {
    #[account(mut)]
    pub vault_signer: Signer<'info>,

    /// CHECK: authority comes from player_profile / source_planet checks
    pub authority: UncheckedAccount<'info>,

    #[account(
        seeds = [b"authorized_vault", authority.key().as_ref()],
        bump = authorized_vault.bump,
    )]
    pub authorized_vault: Account<'info, AuthorizedVault>,

    #[account(
        mut,
        seeds = [b"player_profile", authority.key().as_ref()],
        bump = player_profile.bump,
        has_one = authority @ GameStateError::Unauthorized
    )]
    pub player_profile: Account<'info, PlayerProfile>,

    #[account(
        mut,
        seeds = [b"planet_state", authority.key().as_ref(), &source_planet.planet_index.to_le_bytes()],
        bump = source_planet.bump,
        has_one = authority @ GameStateError::Unauthorized
    )]
    pub source_planet: Account<'info, PlanetState>,

    #[account(
        init,
        payer = vault_signer,
        space = PLANET_STATE_SPACE,
        seeds = [b"planet_state", authority.key().as_ref(), &player_profile.planet_count.to_le_bytes()],
        bump
    )]
    pub colony_planet: Account<'info, PlanetState>,

    pub system_program: Program<'info, System>,
}



// =============================================
// Account Data
// =============================================

#[account]
#[derive(InitSpace)]
pub struct PlayerProfile {
    pub authority: Pubkey,
    pub planet_count: u32,
    pub bump: u8,
}

#[account]
#[derive(InitSpace)]
pub struct AuthorizedVault {
    pub authority: Pubkey,
    pub vault: Pubkey,
    pub expires_at: i64,
    pub revoked: bool,
    pub bump: u8,
}

#[account]
#[derive(InitSpace)]
pub struct VaultBackup {
    pub authority: Pubkey,
    pub vault: Pubkey,
    pub version: u8,
    #[max_len(512)]
    pub ciphertext: Vec<u8>,
    pub iv: [u8; 12],
    pub salt: [u8; 16],
    pub kdf_salt: [u8; 16],
    pub updated_at: i64,
    pub bump: u8,
}

#[account]
#[derive(InitSpace)]
pub struct PlanetState {
    pub authority: Pubkey,
    pub player: Pubkey,
    pub planet_index: u32,
    pub galaxy: u16,
    pub system: u16,
    pub position: u8,
    pub name: [u8; MAX_PLANET_NAME_LEN],
    pub diameter: u32,
    pub temperature: i16,
    pub max_fields: u16,
    pub used_fields: u16,
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
    pub energy_tech: u8,
    pub combustion_drive: u8,
    pub impulse_drive: u8,
    pub hyperspace_drive: u8,
    pub computer_tech: u8,
    pub astrophysics: u8,
    pub igr_network: u8,
    pub research_queue_item: u8,
    pub research_queue_target: u8,
    pub research_finish_ts: i64,
    pub build_queue_item: u8,
    pub build_queue_target: u8,
    pub build_finish_ts: i64,
    pub metal: u64,
    pub crystal: u64,
    pub deuterium: u64,
    pub metal_hour: u64,
    pub crystal_hour: u64,
    pub deuterium_hour: u64,
    pub energy_production: u64,
    pub energy_consumption: u64,
    pub metal_cap: u64,
    pub crystal_cap: u64,
    pub deuterium_cap: u64,
    pub last_update_ts: i64,
    pub small_cargo: u32,
    pub large_cargo: u32,
    pub light_fighter: u32,
    pub heavy_fighter: u32,
    pub cruiser: u32,
    pub battleship: u32,
    pub battlecruiser: u32,
    pub bomber: u32,
    pub destroyer: u32,
    pub deathstar: u32,
    pub recycler: u32,
    pub espionage_probe: u32,
    pub colony_ship: u32,
    pub solar_satellite: u32,
    pub active_missions: u8,
    pub missions: [MissionState; MAX_MISSIONS],
    pub bump: u8,
}


#[account]
#[derive(InitSpace)]
pub struct PlanetCoordinates {
    pub galaxy: u16,
    pub system: u16,
    pub position: u8,
    pub planet: Pubkey,
    pub authority: Pubkey,
    pub bump: u8,
}

impl PlanetState {
    pub fn building_level(&self, idx: u8) -> u8 {
        match idx {
            0 => self.metal_mine, 1 => self.crystal_mine, 2 => self.deuterium_synthesizer,
            3 => self.solar_plant, 4 => self.fusion_reactor, 5 => self.robotics_factory,
            6 => self.nanite_factory, 7 => self.shipyard, 8 => self.metal_storage,
            9 => self.crystal_storage, 10 => self.deuterium_tank, 11 => self.research_lab,
            12 => self.missile_silo, _ => 0,
        }
    }

    pub fn set_building_level(&mut self, idx: u8, level: u8) {
        match idx {
            0 => self.metal_mine = level, 1 => self.crystal_mine = level,
            2 => self.deuterium_synthesizer = level, 3 => self.solar_plant = level,
            4 => self.fusion_reactor = level, 5 => self.robotics_factory = level,
            6 => self.nanite_factory = level, 7 => self.shipyard = level,
            8 => self.metal_storage = level, 9 => self.crystal_storage = level,
            10 => self.deuterium_tank = level, 11 => self.research_lab = level,
            12 => self.missile_silo = level, _ => {}
        }
    }

    pub fn research_level(&self, idx: u8) -> u8 {
        match idx {
            0 => self.energy_tech, 1 => self.combustion_drive, 2 => self.impulse_drive,
            3 => self.hyperspace_drive, 4 => self.computer_tech, 5 => self.astrophysics,
            6 => self.igr_network, _ => 0,
        }
    }

    fn planet_coords_seeds(galaxy: u16, system: u16, position: u8) -> [Vec<u8>; 4] {
    [
        b"planet_coords".to_vec(),
        galaxy.to_le_bytes().to_vec(),
        system.to_le_bytes().to_vec(),
        vec![position],
    ]
}

    pub fn set_research_level(&mut self, idx: u8, level: u8) {
        match idx {
            0 => self.energy_tech = level, 1 => self.combustion_drive = level,
            2 => self.impulse_drive = level, 3 => self.hyperspace_drive = level,
            4 => self.computer_tech = level, 5 => self.astrophysics = level,
            6 => self.igr_network = level, _ => {}
        }
    }

    pub fn free_mission_slot(&self) -> Option<usize> {
        (0..MAX_MISSIONS).find(|&i| self.missions[i].mission_type == 0)
    }

    pub fn mission(&self, slot: usize) -> MissionState { self.missions[slot] }
    pub fn set_mission(&mut self, slot: usize, m: MissionState) { self.missions[slot] = m; }
    pub fn set_mission_applied(&mut self, slot: usize, applied: bool) { self.missions[slot].applied = applied; }
    pub fn clear_mission(&mut self, slot: usize) { self.missions[slot] = MissionState::default(); }

    pub fn return_mission_assets(&mut self, slot: usize) {
        let m = self.missions[slot];
        self.light_fighter = self.light_fighter.saturating_add(m.light_fighter);
        self.heavy_fighter = self.heavy_fighter.saturating_add(m.heavy_fighter);
        self.cruiser = self.cruiser.saturating_add(m.cruiser);
        self.battleship = self.battleship.saturating_add(m.battleship);
        self.battlecruiser = self.battlecruiser.saturating_add(m.battlecruiser);
        self.bomber = self.bomber.saturating_add(m.bomber);
        self.destroyer = self.destroyer.saturating_add(m.destroyer);
        self.deathstar = self.deathstar.saturating_add(m.deathstar);
        self.small_cargo = self.small_cargo.saturating_add(m.small_cargo);
        self.large_cargo = self.large_cargo.saturating_add(m.large_cargo);
        self.recycler = self.recycler.saturating_add(m.recycler);
        self.espionage_probe = self.espionage_probe.saturating_add(m.espionage_probe);
        self.colony_ship = self.colony_ship.saturating_add(m.colony_ship);
        self.metal = self.metal.saturating_add(m.cargo_metal);
        self.crystal = self.crystal.saturating_add(m.cargo_crystal);
        self.deuterium = self.deuterium.saturating_add(m.cargo_deuterium);
    }

    pub fn return_mission_ships_only(&mut self, slot: usize) {
    let m = self.missions[slot];
    self.light_fighter = self.light_fighter.saturating_add(m.light_fighter);
    self.heavy_fighter = self.heavy_fighter.saturating_add(m.heavy_fighter);
    self.cruiser = self.cruiser.saturating_add(m.cruiser);
    self.battleship = self.battleship.saturating_add(m.battleship);
    self.battlecruiser = self.battlecruiser.saturating_add(m.battlecruiser);
    self.bomber = self.bomber.saturating_add(m.bomber);
    self.destroyer = self.destroyer.saturating_add(m.destroyer);
    self.deathstar = self.deathstar.saturating_add(m.deathstar);
    self.small_cargo = self.small_cargo.saturating_add(m.small_cargo);
    self.large_cargo = self.large_cargo.saturating_add(m.large_cargo);
    self.recycler = self.recycler.saturating_add(m.recycler);
    self.espionage_probe = self.espionage_probe.saturating_add(m.espionage_probe);
    self.colony_ship = self.colony_ship.saturating_add(m.colony_ship);
}


    pub fn add_ship(&mut self, ship_type: u8, quantity: u32) -> Result<()> {
        match ship_type {
            0 => self.small_cargo = self.small_cargo.saturating_add(quantity),
            1 => self.large_cargo = self.large_cargo.saturating_add(quantity),
            2 => self.light_fighter = self.light_fighter.saturating_add(quantity),
            3 => self.heavy_fighter = self.heavy_fighter.saturating_add(quantity),
            4 => self.cruiser = self.cruiser.saturating_add(quantity),
            5 => self.battleship = self.battleship.saturating_add(quantity),
            6 => self.battlecruiser = self.battlecruiser.saturating_add(quantity),
            7 => self.bomber = self.bomber.saturating_add(quantity),
            8 => self.destroyer = self.destroyer.saturating_add(quantity),
            9 => self.deathstar = self.deathstar.saturating_add(quantity),
            10 => self.recycler = self.recycler.saturating_add(quantity),
            11 => self.espionage_probe = self.espionage_probe.saturating_add(quantity),
            12 => self.colony_ship = self.colony_ship.saturating_add(quantity),
            13 => self.solar_satellite = self.solar_satellite.saturating_add(quantity),
            _ => return Err(GameStateError::InvalidShipType.into()),
        }
        Ok(())
    }
}

#[derive(AnchorSerialize, AnchorDeserialize, InitSpace, Copy, Clone, Default)]
pub struct MissionState {
    pub mission_type: u8,
    pub target_galaxy: u16,
    pub target_system: u16,
    pub target_position: u8,
    pub colony_name: [u8; MAX_MISSION_COLONY_NAME_LEN],
    pub depart_ts: i64,
    pub arrive_ts: i64,
    pub return_ts: i64,
    pub small_cargo: u32,
    pub large_cargo: u32,
    pub light_fighter: u32,
    pub heavy_fighter: u32,
    pub cruiser: u32,
    pub battleship: u32,
    pub battlecruiser: u32,
    pub bomber: u32,
    pub destroyer: u32,
    pub deathstar: u32,
    pub recycler: u32,
    pub espionage_probe: u32,
    pub colony_ship: u32,
    pub cargo_metal: u64,
    pub cargo_crystal: u64,
    pub cargo_deuterium: u64,
    pub applied: bool,
    pub speed_factor: u8
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct InitializePlanetParams {
    pub name: String,
    pub galaxy: u16,
    pub system: u16,
    pub position: u8,
    pub diameter: u32,
    pub temperature: i16,
    pub max_fields: u16,
    pub used_fields: u16,
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
    pub energy_tech: u8,
    pub combustion_drive: u8,
    pub impulse_drive: u8,
    pub hyperspace_drive: u8,
    pub computer_tech: u8,
    pub astrophysics: u8,
    pub igr_network: u8,
    pub research_queue_item: u8,
    pub research_queue_target: u8,
    pub research_finish_ts: i64,
    pub build_queue_item: u8,
    pub build_queue_target: u8,
    pub build_finish_ts: i64,
    pub metal: u64,
    pub crystal: u64,
    pub deuterium: u64,
    pub metal_hour: u64,
    pub crystal_hour: u64,
    pub deuterium_hour: u64,
    pub energy_production: u64,
    pub energy_consumption: u64,
    pub metal_cap: u64,
    pub crystal_cap: u64,
    pub deuterium_cap: u64,
    pub last_update_ts: i64,
    pub small_cargo: u32,
    pub large_cargo: u32,
    pub light_fighter: u32,
    pub heavy_fighter: u32,
    pub cruiser: u32,
    pub battleship: u32,
    pub battlecruiser: u32,
    pub bomber: u32,
    pub destroyer: u32,
    pub deathstar: u32,
    pub recycler: u32,
    pub espionage_probe: u32,
    pub colony_ship: u32,
    pub solar_satellite: u32,
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct InitializeHomeworldParams {
    pub now: i64,
    pub name: String,
    pub galaxy: u16,
    pub system: u16,
    pub position: u8,
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct InitializeColonyParams {
    pub now: i64,
    pub name: String,
    pub galaxy: u16,
    pub system: u16,
    pub position: u8,
    pub cargo_metal: u64,
    pub cargo_crystal: u64,
    pub cargo_deuterium: u64,
    pub small_cargo: u32,
    pub large_cargo: u32,
    pub light_fighter: u32,
    pub heavy_fighter: u32,
    pub cruiser: u32,
    pub battleship: u32,
    pub battlecruiser: u32,
    pub bomber: u32,
    pub destroyer: u32,
    pub deathstar: u32,
    pub recycler: u32,
    pub espionage_probe: u32,
    pub solar_satellite: u32,
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct LaunchFleetParams {
    pub mission_type: u8,
    pub light_fighter: u32,
    pub heavy_fighter: u32,
    pub cruiser: u32,
    pub battleship: u32,
    pub battlecruiser: u32,
    pub bomber: u32,
    pub destroyer: u32,
    pub deathstar: u32,
    pub small_cargo: u32,
    pub large_cargo: u32,
    pub recycler: u32,
    pub espionage_probe: u32,
    pub colony_ship: u32,
    pub cargo_metal: u64,
    pub cargo_crystal: u64,
    pub cargo_deuterium: u64,
    pub speed_factor: u8,
    pub now: i64,
    pub target_galaxy: u16,
    pub target_system: u16,
    pub target_position: u8,
    pub colony_name: String,
}

#[error_code]
pub enum GameStateError {
    #[msg("The caller is not authorized to modify this account.")]
    Unauthorized,
    #[msg("Planet coordinates are out of range.")]
    InvalidCoordinates,
    #[msg("Planet count overflowed.")]
    PlanetCountOverflow,
    #[msg("Build queue is busy.")]
    QueueBusy,
    #[msg("No free building fields are available.")]
    NoFields,
    #[msg("Insufficient metal.")]
    InsufficientMetal,
    #[msg("Insufficient crystal.")]
    InsufficientCrystal,
    #[msg("Insufficient deuterium.")]
    InsufficientDeuterium,
    #[msg("No build is currently queued.")]
    NoBuild,
    #[msg("The queued build has not finished yet.")]
    BuildNotFinished,
    #[msg("Invalid research technology.")]
    InvalidTech,
    #[msg("Research lab level is too low.")]
    LabTooLow,
    #[msg("Research queue is busy.")]
    ResearchQueueBusy,
    #[msg("No research is currently queued.")]
    NoResearch,
    #[msg("The queued research has not finished yet.")]
    ResearchNotFinished,
    #[msg("Mission is invalid for this instruction.")]
    InvalidMission,
    #[msg("Mission arguments are invalid.")]
    InvalidArgs,
    #[msg("Invalid ship type.")]
    InvalidShipType,
    #[msg("Ship is locked by research requirements.")]
    TechLocked,
    #[msg("Shipyard level is too low.")]
    ShipyardTooLow,
    #[msg("The selected fleet is empty.")]
    EmptyFleet,
    #[msg("No free mission slot is available.")]
    NoMissionSlot,
    #[msg("Insufficient ships are available.")]
    InsufficientShips,
    #[msg("Cargo exceeds the selected fleet capacity.")]
    ExceedsCargo,
    #[msg("Insufficient resources are available.")]
    InsufficientResources,
    #[msg("Mission slot is invalid.")]
    InvalidMissionSlot,
    #[msg("Mission destination does not match the provided destination planet.")]
    InvalidDestination,
    #[msg("Mission is still in flight.")]
    MissionInFlight,
    #[msg("Return trip has not completed yet.")]
    ReturnInFlight,
    #[msg("Mission was already resolved.")]
    AlreadyResolved,
    #[msg("Colonize mission is missing a colony ship.")]
    MissingColonyShip,
    #[msg("The provided vault authorization is invalid.")]
    InvalidVaultAuthorization,
    #[msg("The provided vault authorization has expired.")]
    VaultAuthorizationExpired,
    #[msg("The provided vault authorization was revoked.")]
    VaultAuthorizationRevoked,
    #[msg("Encrypted vault backup is too large.")]
    BackupTooLarge,
}