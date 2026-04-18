use anchor_lang::prelude::*;
use anchor_spl::token::{self, Burn, Mint, Token, TokenAccount};

use crate::constants::*;
use crate::error::GameStateError;
use crate::state::*;

    // =============================================    
    // Helper Functions
    // =============================================

pub(crate) fn validate_coordinates(galaxy: u16, system: u16, position: u8) -> Result<()> {
        require!((1..=499).contains(&galaxy), GameStateError::InvalidCoordinates);
        require!((1..=999).contains(&system), GameStateError::InvalidCoordinates);
        require!((1..=15).contains(&position), GameStateError::InvalidCoordinates);
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
        for _ in 0..n { r = r * 3 / 2; }
        r
    }

pub(crate) fn base_cost(idx: u8) -> (u32, u32, u32) {
        match idx {
            0 => (60, 15, 0), 1 => (48, 24, 0), 2 => (225, 75, 0), 3 => (75, 30, 0),
            4 => (900, 360, 900), 5 => (400, 120, 200), 6 => (1_000_000, 500_000, 100_000),
            7 => (400, 200, 100), 8 => (1000, 0, 0), 9 => (1000, 500, 0),
            10 => (1000, 1000, 0), 11 => (200, 400, 200), 12 => (20, 20, 0),
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
            0 => (0, 800, 400), 1 => (400, 0, 600), 2 => (2000, 4000, 600),
            3 => (10000, 20000, 6000), 4 => (0, 400, 600), 5 => (4000, 2000, 1000),
            6 => (240000, 400000, 160000), _ => (0, 0, 0),
        }
    }

pub(crate) fn research_lab_requirement(idx: u8) -> u8 {
        match idx {
            0 | 1 | 4 => 1, 5 => 3, 2 => 5, 3 => 7, 6 => 10, _ => 255,
        }
    }

pub(crate) fn pow2(level: u8) -> u64 {
        1u64.checked_shl(level as u32).unwrap_or(u64::MAX)
    }

pub(crate) fn research_cost_for_level(idx: u8, current: u8) -> (u64, u64, u64) {
        let (m, c, d) = research_base_cost(idx);
        let mult = pow2(current);
        (m.saturating_mul(mult), c.saturating_mul(mult), d.saturating_mul(mult))
    }

pub(crate) fn research_seconds(next_level: u8, lab_level: u8, igr_network: u8) -> i64 {
        let speed_bonus = 100u64.saturating_add(igr_network as u64 * 10);
        let effective_lab = (lab_level.max(1) as u64)
            .saturating_mul(speed_bonus)
            / 100;
        ((next_level as u64 * 1800) / effective_lab.max(1)).max(1) as i64
    }

pub(crate) fn ship_build_seconds(ship_type: u8, quantity: u32, shipyard: u8, nanite: u8) -> i64 {
        let (m, c, d) = ship_cost(ship_type);
        let total = m
            .saturating_add(c)
            .saturating_add(d)
            .saturating_mul(quantity as u64);

        let speed = (shipyard.max(1) as u64)
            .saturating_mul(2u64.pow(nanite as u32))
            .max(1);

        (total / (1_000 * speed)).max(1) as i64
    }

pub(crate) fn ship_cost(ship_type: u8) -> (u64, u64, u64) {
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

pub(crate) fn enforce_ship_research_gate(ship_type: u8, planet: &PlanetState) -> Result<()> {
        match ship_type {
            0 => require!(planet.combustion_drive >= 2, GameStateError::TechLocked),
            1 => require!(planet.combustion_drive >= 6, GameStateError::TechLocked),
            12 => require!(planet.impulse_drive >= 3 && planet.astrophysics >= 4, GameStateError::TechLocked),
            _ => {}
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
            _ => {}
        }
        Ok(())
    }

pub(crate) fn cargo_capacity(sc: u32, lc: u32, rec: u32, cr: u32, bs: u32) -> u64 {
        sc as u64 * 5_000 + lc as u64 * 25_000 + rec as u64 * 20_000 + cr as u64 * 800 + bs as u64 * 1_500
    }

pub(crate) fn launch_fuel_cost(
        lf: u32, hf: u32, cr: u32, bs: u32, bc: u32, bm: u32, ds: u32, de: u32,
        sc: u32, lc: u32, rec: u32, ep: u32, col: u32, speed_factor: u8,
    ) -> u64 {
        (sc as u64 * 10 + lc as u64 * 50 + lf as u64 * 20 + hf as u64 * 75 +
        cr as u64 * 300 + bs as u64 * 500 + bc as u64 * 250 + bm as u64 * 1_000 +
        ds as u64 * 1_000 + rec as u64 * 300 + ep as u64 + col as u64 * 1_000)
            * (speed_factor as u64).pow(2) / 10_000
    }

pub(crate) fn mine_rate(level: u8, base: u64) -> u64 {
        if level == 0 { return 0; }
        base * (level as u64) * 11u64.pow(level as u32) / 10u64.pow(level as u32)
    }

pub(crate) fn store_cap(level: u8) -> u64 {
        if level == 0 { 1_000_000 } else { 1_000_000 * 2u64.pow(level as u32) }
    }

pub(crate) fn settle_resources(planet: &mut PlanetState, now: i64) {
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

pub(crate) fn research_flight_bonus_pct(
        from_galaxy: u16,
        from_system: u16,
        to_galaxy: u16,
        to_system: u16,
        planet: &PlanetState,
    ) -> u64 {
        let mut bonus = 100u64;

        if from_galaxy == to_galaxy && from_system == to_system {
            return bonus.saturating_add(planet.combustion_drive as u64 * 5);
        }

        if from_galaxy == to_galaxy {
            bonus = bonus.saturating_add(planet.impulse_drive as u64 * 10);

            if planet.hyperspace_drive >= 1 {
                bonus = bonus.saturating_add(planet.hyperspace_drive as u64 * 5);
            }

            if planet.astrophysics >= 1 {
                bonus = bonus.saturating_add(planet.astrophysics as u64 * 3);
            }

            return bonus;
        }

        if planet.hyperspace_drive >= 3 {
            bonus = bonus.saturating_add(planet.hyperspace_drive as u64 * 15);
        }

        if planet.astrophysics >= 4 {
            bonus = bonus.saturating_add(planet.astrophysics as u64 * 10);
        }

        bonus
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
        let fusion_prod = if planet.fusion_reactor == 0 {
            0
        } else {
            let base = mine_rate(planet.fusion_reactor, 30) * 180 / 100;
            base.saturating_mul(100 + planet.energy_tech as u64 * 10) / 100
        };

        planet.energy_production = solar_prod + fusion_prod;
        planet.energy_consumption = mine_rate(planet.metal_mine, 10)
            + mine_rate(planet.crystal_mine, 10)
            + mine_rate(planet.deuterium_synthesizer, 20);

        planet.metal_cap = store_cap(planet.metal_storage);
        planet.crystal_cap = store_cap(planet.crystal_storage);
        planet.deuterium_cap = store_cap(planet.deuterium_tank);
    }

pub(crate) fn require_active_vault(
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
        require_keys_eq!(player_profile.authority, authority, GameStateError::Unauthorized);

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
            bump: coords_bump,
        };

        let mut data = planet_coords_info.try_borrow_mut_data()?;
        let disc = <PlanetCoordinates as anchor_lang::Discriminator>::DISCRIMINATOR;
        data[..8].copy_from_slice(&disc);
        coords_data.serialize(&mut &mut data[8..])?;

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

        planet.active_missions = 0;

        for i in 0..MAX_MISSIONS {
            planet.missions[i] = MissionState::default();
        }

        planet.bump = bump;
        planet.ship_build_item = params.ship_build_item;
        planet.ship_build_qty = params.ship_build_qty;
        planet.ship_build_finish_ts = params.ship_build_finish_ts;

        msg!("create_planet_state: finished writing planet_state fields");

        Ok(())
    }

pub(crate) fn produce_planet(planet: &mut PlanetState, now: i64) -> Result<()> {
        settle_resources(planet, now);
        Ok(())
    }

pub(crate) fn finish_build_now(planet: &mut PlanetState, now: i64) -> Result<()> {
        settle_resources(planet, now);
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

pub(crate) fn start_build_planet(planet: &mut PlanetState, building_idx: u8, now: i64) -> Result<()> {
        settle_resources(planet, now);
        let current = planet.building_level(building_idx);
        let next = current.saturating_add(1);
        let (cm, cc, cd) = upgrade_cost(building_idx, next as u64);

        require!(planet.build_finish_ts == 0 || now >= planet.build_finish_ts, GameStateError::QueueBusy);
        require!(planet.used_fields < planet.max_fields, GameStateError::NoFields);
        enforce_building_requirements(building_idx, planet)?;
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

pub(crate) fn finish_build_planet(planet: &mut PlanetState, now: i64) -> Result<()> {
        require!(now >= planet.build_finish_ts, GameStateError::BuildNotFinished);
        finish_build_now(planet, now)
    }

pub(crate) fn start_research_planet(planet: &mut PlanetState, tech_idx: u8, now: i64) -> Result<()> {
        settle_resources(planet, now);
        require!(tech_idx <= 6, GameStateError::InvalidTech);
        require!(planet.research_lab >= 1, GameStateError::LabTooLow);
        require!(planet.research_queue_item == 255, GameStateError::ResearchQueueBusy);

        let lab_req = research_lab_requirement(tech_idx);
        require!(planet.research_lab >= lab_req, GameStateError::LabTooLow);
        enforce_research_requirements(tech_idx, planet)?;

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
        planet.research_finish_ts = now + research_seconds(next, planet.research_lab, planet.igr_network);
        Ok(())
    }

pub(crate) fn finish_research_now(planet: &mut PlanetState, now: i64) -> Result<()> {
        settle_resources(planet, now);
        require!(planet.research_queue_item != 255, GameStateError::NoResearch);

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
        require!(now >= planet.research_finish_ts, GameStateError::ResearchNotFinished);
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
        if from_galaxy != to_galaxy {
            return (from_galaxy as i64 - to_galaxy as i64).abs() as u64 * 20_000;
        }

        if from_system != to_system {
            return (from_system as i64 - to_system as i64).abs() as u64 * 2_000;
        }

        return (from_position as i64 - to_position as i64).abs() as u64 * 200 + 1_000;
    }

pub(crate) fn mission_flight_seconds(
        from_galaxy: u16,
        from_system: u16,
        from_position: u8,
        to_galaxy: u16,
        to_system: u16,
        to_position: u8,
        speed_factor: u8,
        planet: &PlanetState,
    ) -> i64 {
        let sf = speed_factor.clamp(10, 100) as u64;
        let dist = distance(
            from_galaxy,
            from_system,
            from_position,
            to_galaxy,
            to_system,
            to_position,
        );
        let tech_bonus =
            research_flight_bonus_pct(from_galaxy, from_system, to_galaxy, to_system, planet);
        ((dist * 100) / sf)
            .saturating_mul(100)
            .checked_div(tech_bonus.max(100))
            .unwrap_or(1)
            .max(1) as i64
    }


pub(crate) fn build_ship_planet(planet: &mut PlanetState, ship_type: u8, quantity: u32, now: i64) -> Result<()> {
        require!(quantity > 0, GameStateError::InvalidArgs);
        settle_resources(planet, now);
        require!(planet.shipyard >= 1, GameStateError::ShipyardTooLow);
        require!(planet.ship_build_item == 255, GameStateError::ShipyardQueueBusy);

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

        let dur = ship_build_seconds(ship_type, quantity, planet.shipyard, planet.nanite_factory);

        planet.ship_build_item = ship_type;
        planet.ship_build_qty = quantity;
        planet.ship_build_finish_ts = now + dur;

        Ok(())
    }

pub(crate) fn finish_ship_build_planet(planet: &mut PlanetState, now: i64) -> Result<()> {
        require!(now >= planet.ship_build_finish_ts, GameStateError::ShipBuildNotFinished);
        finish_ship_build_now(planet, now)
    }

pub(crate) fn finish_ship_build_now(planet: &mut PlanetState, now: i64) -> Result<()> {
        settle_resources(planet, now);

        require!(planet.ship_build_item != 255, GameStateError::NoShipBuild);
        require!(planet.ship_build_finish_ts > 0, GameStateError::NoShipBuild);

        let ship_type = planet.ship_build_item;
        let quantity = planet.ship_build_qty;

        planet.add_ship(ship_type, quantity)?;

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
        require!(planet.research_queue_item != 255, GameStateError::NoResearch);
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

pub(crate) fn launch_fleet_planet(planet: &mut PlanetState, params: LaunchFleetParams) -> Result<()> {
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

        let flight_seconds = mission_flight_seconds(
            planet.galaxy,
            planet.system,
            planet.position,
            params.target_galaxy,
            params.target_system,
            params.target_position,
            speed_factor,
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

        let arrive_ts = params.now.saturating_add(flight_seconds);

        let return_ts = 0;

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
            speed_factor,
        });

        planet.active_missions = planet.active_missions.saturating_add(1);
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
        require!(mission.mission_type == MISSION_TRANSPORT, GameStateError::InvalidMission);

        if !mission.applied {
            require!(now >= mission.arrive_ts, GameStateError::MissionInFlight);

            settle_resources(source, now);

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

            require!(source.deuterium >= return_fuel, GameStateError::InsufficientDeuterium);
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

        settle_resources(source, now);
        source.return_mission_assets(slot);
        source.clear_mission(slot);
        source.active_missions = source.active_missions.saturating_sub(1);
        Ok(())
    }

    /// Resolve a colonize mission.
    ///
    /// The `colony_planet` and `colony_coords` accounts must ALREADY be initialized
    /// (by `initialize_colony` / `initialize_colony_vault` in the same tx, or by a
    /// preceding tx). This instruction only clears the mission slot on the source
    /// planet — it does NOT create any accounts.
pub(crate) fn resolve_colonize_planet(
        source: &mut PlanetState,
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

        // Guard: source planet must not be at the target coords
        let coords_taken = source.galaxy == mission.target_galaxy
            && source.system == mission.target_system
            && source.position == mission.target_position;
        require!(!coords_taken, GameStateError::InvalidDestination);

        source.clear_mission(slot);
        source.active_missions = source.active_missions.saturating_sub(1);

        Ok(())
    }
