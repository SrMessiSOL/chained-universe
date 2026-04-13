    use anchor_lang::prelude::*;
    use anchor_spl::token::{self, Burn, Mint, Token, TokenAccount};

    declare_id!("7yKyjQ7m8tSqvqYnV65aVV9Jwdee7KqyELeDXf6Fxkt4");

    pub const MAX_PLANET_NAME_LEN: usize = 32;
    pub const MAX_MISSION_COLONY_NAME_LEN: usize = 32;
    pub const MAX_MISSIONS: usize = 4;
    pub const MISSION_TRANSPORT: u8 = 2;
    pub const MISSION_COLONIZE: u8 = 5;
    pub const ANTIMATTER_DECIMALS: u8 = 6;
    pub const ANTIMATTER_SCALE: u64 = 1_000_000;
    pub const PLANET_COORDS_SPACE: usize = 8 + PlanetCoordinates::INIT_SPACE;
    pub const PLAYER_PROFILE_SPACE: usize = 8 + PlayerProfile::INIT_SPACE;
    pub const PLANET_STATE_SPACE: usize = 8 + PlanetState::INIT_SPACE;
    pub const AUTHORIZED_VAULT_SPACE: usize = 8 + AuthorizedVault::INIT_SPACE;
    pub const VAULT_BACKUP_SPACE: usize = 8 + VaultBackup::INIT_SPACE;
    pub const GAME_CONFIG_SPACE: usize = 8 + GameConfig::INIT_SPACE;
    pub const MARKET_PROGRAM_ID: Pubkey = Pubkey::new_from_array([
        194, 171, 76, 163, 210, 137, 5, 66, 103, 236, 205, 120, 111, 87, 59, 250,
        139, 237, 101, 230, 54, 199, 209, 132, 25, 2, 106, 137, 247, 197, 199, 242,
    ]);

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

    fn research_seconds(next_level: u8, lab_level: u8, igr_network: u8) -> i64 {
        let speed_bonus = 100u64.saturating_add(igr_network as u64 * 10);
        let effective_lab = (lab_level.max(1) as u64)
            .saturating_mul(speed_bonus)
            / 100;
        ((next_level as u64 * 1800) / effective_lab.max(1)).max(1) as i64
    }

    fn ship_build_seconds(ship_type: u8, quantity: u32, shipyard: u8, nanite: u8) -> i64 {
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

    fn enforce_building_requirements(building_idx: u8, planet: &PlanetState) -> Result<()> {
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

    fn enforce_research_requirements(tech_idx: u8, planet: &PlanetState) -> Result<()> {
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

    fn research_flight_bonus_pct(
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

    fn require_market_authority(market_authority: &Signer<'_>) -> Result<()> {
        let (expected_pda, _) =
            Pubkey::find_program_address(&[b"market_authority"], &MARKET_PROGRAM_ID);
        require_keys_eq!(
            market_authority.key(),
            expected_pda,
            GameStateError::UnauthorizedMarket
        );
        Ok(())
    }

    fn create_planet_state<'info>(
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

    fn produce_planet(planet: &mut PlanetState, now: i64) -> Result<()> {
        settle_resources(planet, now);
        Ok(())
    }

    fn finish_build_now(planet: &mut PlanetState, now: i64) -> Result<()> {
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

    fn start_build_planet(planet: &mut PlanetState, building_idx: u8, now: i64) -> Result<()> {
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

    fn finish_build_planet(planet: &mut PlanetState, now: i64) -> Result<()> {
        require!(now >= planet.build_finish_ts, GameStateError::BuildNotFinished);
        finish_build_now(planet, now)
    }

    fn start_research_planet(planet: &mut PlanetState, tech_idx: u8, now: i64) -> Result<()> {
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

    fn finish_research_now(planet: &mut PlanetState, now: i64) -> Result<()> {
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

    fn finish_research_planet(planet: &mut PlanetState, now: i64) -> Result<()> {
        require!(now >= planet.research_finish_ts, GameStateError::ResearchNotFinished);
        finish_research_now(planet, now)
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

    fn mission_flight_seconds(
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


    fn build_ship_planet(planet: &mut PlanetState, ship_type: u8, quantity: u32, now: i64) -> Result<()> {
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

    fn finish_ship_build_planet(planet: &mut PlanetState, now: i64) -> Result<()> {
        require!(now >= planet.ship_build_finish_ts, GameStateError::ShipBuildNotFinished);
        finish_ship_build_now(planet, now)
    }

    fn finish_ship_build_now(planet: &mut PlanetState, now: i64) -> Result<()> {
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

    fn burn_antimatter<'info>(
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

    fn accelerate_build_with_antimatter_inner<'info>(
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

    fn accelerate_research_with_antimatter_inner<'info>(
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

    fn accelerate_ship_build_with_antimatter_inner<'info>(
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

    fn resolve_transport_empty_slot(
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
    fn resolve_colonize_planet(
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

    // =============================================
    // Program
    // =============================================

    #[program]
    pub mod game_state {
        use super::*;

        /// One-time wallet setup: creates player profile + authorizes vault + stores encrypted backup.
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

        /// Wallet-only: rotate vault key and update backup.
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

        /// Vault-signed: initialize homeworld.
        /// Creates both `planet_state` and `planet_coords` atomically.
        /// If `planet_coords` already exists for these coordinates the tx fails — client retries with new coords.
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
                ship_build_item: 255, ship_build_qty: 0, ship_build_finish_ts: 0,
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
                &ctx.accounts.planet_coords.to_account_info(),
                &ctx.accounts.vault_signer.to_account_info(),
                &ctx.accounts.system_program.to_account_info(),
                ctx.bumps.planet_state,
                &planet_params,
            )
        }

        /// Vault-signed: initialize colony.
        /// Creates both `planet_state` and `planet_coords` atomically.
        /// If `planet_coords` already exists the tx fails — client shows "slot occupied".
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
                ship_build_item: 255, ship_build_qty: 0, ship_build_finish_ts: 0,
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
                &ctx.accounts.planet_coords.to_account_info(),
                &ctx.accounts.vault_signer.to_account_info(),
                &ctx.accounts.system_program.to_account_info(),
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

        pub fn finish_ship_build(ctx: Context<MutatePlanetState>, now: i64) -> Result<()> {
            finish_ship_build_planet(&mut ctx.accounts.planet_state, now)
        }

        pub fn finish_ship_build_vault(ctx: Context<MutatePlanetStateVault>, now: i64) -> Result<()> {
            require_active_vault(ctx.accounts.vault_signer.key(), &ctx.accounts.authorized_vault, ctx.accounts.planet_state.authority)?;
            finish_ship_build_planet(&mut ctx.accounts.planet_state, now)
        }

        pub fn launch_fleet(ctx: Context<MutatePlanetState>, params: LaunchFleetParams) -> Result<()> {
            launch_fleet_planet(&mut ctx.accounts.planet_state, params)
        }

        pub fn launch_fleet_vault(ctx: Context<MutatePlanetStateVault>, params: LaunchFleetParams) -> Result<()> {
            require_active_vault(ctx.accounts.vault_signer.key(), &ctx.accounts.authorized_vault, ctx.accounts.planet_state.authority)?;
            launch_fleet_planet(&mut ctx.accounts.planet_state, params)
        }

        pub fn lock_resources_for_market(
            ctx: Context<MutatePlanetState>,
            resource_type: u8,
            amount: u64,
        ) -> Result<()> {
            require!(amount > 0, GameStateError::InvalidArgs);

            let resource_type = match resource_type {
                0 => ResourceType::Metal,
                1 => ResourceType::Crystal,
                2 => ResourceType::Deuterium,
                _ => return err!(GameStateError::InvalidArgs),
            };

            let planet = &mut ctx.accounts.planet_state;
            let now = Clock::get()?.unix_timestamp;
            settle_resources(planet, now);

            match resource_type {
                ResourceType::Metal => {
                    require!(planet.metal >= amount, GameStateError::InsufficientResources);
                    planet.metal = planet.metal.saturating_sub(amount);
                }
                ResourceType::Crystal => {
                    require!(planet.crystal >= amount, GameStateError::InsufficientResources);
                    planet.crystal = planet.crystal.saturating_sub(amount);
                }
                ResourceType::Deuterium => {
                    require!(planet.deuterium >= amount, GameStateError::InsufficientResources);
                    planet.deuterium = planet.deuterium.saturating_sub(amount);
                }
            }

            Ok(())
        }

        pub fn release_resources_from_market(
            ctx: Context<ReleaseResourcesFromMarket>,
            resource_type: u8,
            amount: u64,
        ) -> Result<()> {
            require!(amount > 0, GameStateError::InvalidArgs);
            require_market_authority(&ctx.accounts.market_authority)?;

            let resource_type = match resource_type {
                0 => ResourceType::Metal,
                1 => ResourceType::Crystal,
                2 => ResourceType::Deuterium,
                _ => return err!(GameStateError::InvalidArgs),
            };

            let seller = &mut ctx.accounts.seller_planet;
            let now = Clock::get()?.unix_timestamp;
            settle_resources(seller, now);

            match resource_type {
                ResourceType::Metal => seller.metal = seller.metal.saturating_add(amount),
                ResourceType::Crystal => seller.crystal = seller.crystal.saturating_add(amount),
                ResourceType::Deuterium => seller.deuterium = seller.deuterium.saturating_add(amount),
            }

            Ok(())
        }

        pub fn transfer_resources_from_market(
            ctx: Context<TransferResourcesFromMarket>,
            resource_type: u8,
            amount: u64,
        ) -> Result<()> {
            require!(amount > 0, GameStateError::InvalidArgs);
            require_market_authority(&ctx.accounts.market_authority)?;

            let resource_type = match resource_type {
                0 => ResourceType::Metal,
                1 => ResourceType::Crystal,
                2 => ResourceType::Deuterium,
                _ => return err!(GameStateError::InvalidArgs),
            };

            let buyer = &mut ctx.accounts.buyer_planet;
            let now = Clock::get()?.unix_timestamp;
            settle_resources(buyer, now);
            require_keys_eq!(buyer.authority, ctx.accounts.buyer.key(), GameStateError::Unauthorized);

            match resource_type {
                ResourceType::Metal => buyer.metal = buyer.metal.saturating_add(amount),
                ResourceType::Crystal => buyer.crystal = buyer.crystal.saturating_add(amount),
                ResourceType::Deuterium => buyer.deuterium = buyer.deuterium.saturating_add(amount),
            }

            msg!(
                "Market transfer: {} {} credited to buyer",
                amount,
                match resource_type {
                    ResourceType::Metal => "metal",
                    ResourceType::Crystal => "crystal",
                    ResourceType::Deuterium => "deuterium",
                }
            );

            Ok(())
        }

        pub fn resolve_transport(ctx: Context<ResolveTransport>, slot: u8, now: i64) -> Result<()> {
                msg!("resolve_transport: entered");
                msg!("resolve_transport: slot={}", slot);
            resolve_transport_planets(&mut ctx.accounts.source_planet, &mut ctx.accounts.destination_planet, slot as usize, now)
        }

        pub fn resolve_transport_vault(ctx: Context<ResolveTransportVault>, slot: u8, now: i64) -> Result<()> {
        msg!("resolve_transport_vault: entered");
        msg!("resolve_transport_vault: slot={}", slot);
        require_active_vault(
            ctx.accounts.vault_signer.key(),
            &ctx.accounts.authorized_vault,
            ctx.accounts.source_planet.authority
        )?;
        msg!("resolve_transport_vault: vault ok");
        resolve_transport_planets(&mut ctx.accounts.source_planet, &mut ctx.accounts.destination_planet, slot as usize, now)
    }

        pub fn resolve_transport_empty(ctx: Context<MutatePlanetState>, slot: u8, now: i64) -> Result<()> {
            resolve_transport_empty_slot(&mut ctx.accounts.planet_state, slot as usize, now)
        }

        pub fn resolve_transport_empty_vault(ctx: Context<MutatePlanetStateVault>, slot: u8, now: i64) -> Result<()> {
            require_active_vault(
                ctx.accounts.vault_signer.key(),
                &ctx.accounts.authorized_vault,
                ctx.accounts.planet_state.authority
            )?;
            resolve_transport_empty_slot(&mut ctx.accounts.planet_state, slot as usize, now)
        }

        /// Wallet-signed: resolve a colonize mission.
        /// The colony planet + coord lock must already exist (created by `initialize_colony`).
        pub fn resolve_colonize(ctx: Context<ResolveColonize>, slot: u8, now: i64) -> Result<()> {
            // Verify the coords PDA matches the mission target
            let mission = ctx.accounts.source_planet.mission(slot as usize);
            require_keys_eq!(
                ctx.accounts.colony_coords.planet,
                ctx.accounts.colony_planet.key(),
                GameStateError::InvalidDestination
            );
            require!(
                ctx.accounts.colony_coords.galaxy == mission.target_galaxy
                    && ctx.accounts.colony_coords.system == mission.target_system
                    && ctx.accounts.colony_coords.position == mission.target_position,
                GameStateError::InvalidDestination
            );
            resolve_colonize_planet(&mut ctx.accounts.source_planet, slot as usize, now)
        }

        /// Vault-signed: resolve a colonize mission.
    pub fn resolve_colonize_vault(ctx: Context<ResolveColonizeVault>, slot: u8, now: i64) -> Result<()> {
        msg!("resolve_colonize_vault: entered");
        msg!("resolve_colonize_vault: slot={}", slot);

        require_active_vault(
            ctx.accounts.vault_signer.key(),
            &ctx.accounts.authorized_vault,
            ctx.accounts.source_planet.authority,
        )?;
        msg!("resolve_colonize_vault: vault ok");

        let mission = ctx.accounts.source_planet.mission(slot as usize);
        msg!("resolve_colonize_vault: loaded mission");

        require_keys_eq!(
            ctx.accounts.colony_coords.planet,
            ctx.accounts.colony_planet.key(),
            GameStateError::InvalidDestination
        );
        msg!("resolve_colonize_vault: planet key matches");

        require!(
            ctx.accounts.colony_coords.galaxy == mission.target_galaxy
                && ctx.accounts.colony_coords.system == mission.target_system
                && ctx.accounts.colony_coords.position == mission.target_position,
            GameStateError::InvalidDestination
        );
        msg!("resolve_colonize_vault: coords match mission");

        resolve_colonize_planet(&mut ctx.accounts.source_planet, slot as usize, now)
    }

        /// Wallet-signed: transfer ownership of a single planet to a new authority.
        ///
        /// Both the old and new authorities must have initialized their player profile.
        /// After transfer, vault-signed gameplay by the new wallet works immediately
        /// because `MutatePlanetStateVault` looks up `authorized_vault` via
        /// `planet_state.authority`, which now points to the new wallet.
        ///
        /// The planet PDA address does not change — it stays seeded by the old wallet.
        /// The old wallet's wallet-signed fallback path for this planet stops working
        /// (by design — only the new authority owns it).
        pub fn transfer_planet(ctx: Context<TransferPlanet>) -> Result<()> {
            let planet = &mut ctx.accounts.planet_state;
            let coords = &mut ctx.accounts.planet_coords;
            let new_authority = ctx.accounts.new_authority.key();

            // Update ownership fields
            planet.authority = new_authority;
            coords.authority = new_authority;

            Ok(())
        }

        /// One-time admin setup for the global ANTIMATTER mint used to accelerate queues.
        pub fn initialize_game_config(
            ctx: Context<InitializeGameConfig>,
            antimatter_mint: Pubkey,
        ) -> Result<()> {
            ctx.accounts.game_config.set_inner(GameConfig {
                admin: ctx.accounts.admin.key(),
                antimatter_mint,
                bump: ctx.bumps.game_config,
            });
            Ok(())
        }

        /// Admin-only: rotate the ANTIMATTER mint reference if needed.
        pub fn update_antimatter_mint(
            ctx: Context<UpdateGameConfig>,
            antimatter_mint: Pubkey,
        ) -> Result<()> {
            ctx.accounts.game_config.antimatter_mint = antimatter_mint;
            Ok(())
        }

        /// Wallet-signed: burn 1 ANTIMATTER per second remaining to finish a building queue instantly.
        pub fn accelerate_build_with_antimatter(
            ctx: Context<UseAntimatter>,
        ) -> Result<()> {
            require_keys_eq!(
                ctx.accounts.game_config.antimatter_mint,
                ctx.accounts.antimatter_mint.key(),
                GameStateError::InvalidAntimatterMint
            );
            accelerate_build_with_antimatter_inner(
                &mut ctx.accounts.planet_state,
                &ctx.accounts.antimatter_mint,
                &ctx.accounts.user_antimatter_account,
                &ctx.accounts.authority,
                &ctx.accounts.token_program,
            )?;
            Ok(())
        }

        /// Wallet-signed: burn 1 ANTIMATTER per second remaining to finish research instantly.
        pub fn accelerate_research_with_antimatter(
            ctx: Context<UseAntimatter>,
        ) -> Result<()> {
            require_keys_eq!(
                ctx.accounts.game_config.antimatter_mint,
                ctx.accounts.antimatter_mint.key(),
                GameStateError::InvalidAntimatterMint
            );
            accelerate_research_with_antimatter_inner(
                &mut ctx.accounts.planet_state,
                &ctx.accounts.antimatter_mint,
                &ctx.accounts.user_antimatter_account,
                &ctx.accounts.authority,
                &ctx.accounts.token_program,
            )?;
            Ok(())
        }

        /// Wallet-signed: burn 1 ANTIMATTER per second remaining to finish ship production instantly.
        pub fn accelerate_ship_build_with_antimatter(
            ctx: Context<UseAntimatter>,
        ) -> Result<()> {
            require_keys_eq!(
                ctx.accounts.game_config.antimatter_mint,
                ctx.accounts.antimatter_mint.key(),
                GameStateError::InvalidAntimatterMint
            );
            accelerate_ship_build_with_antimatter_inner(
                &mut ctx.accounts.planet_state,
                &ctx.accounts.antimatter_mint,
                &ctx.accounts.user_antimatter_account,
                &ctx.accounts.authority,
                &ctx.accounts.token_program,
            )?;
            Ok(())
        }
    }

    // =============================================
    // Account Contexts
    // =============================================

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

    #[derive(Accounts)]
    pub struct InitializeGameConfig<'info> {
        #[account(mut)]
        pub admin: Signer<'info>,
        #[account(
            init,
            payer = admin,
            space = GAME_CONFIG_SPACE,
            seeds = [b"game_config"],
            bump
        )]
        pub game_config: Account<'info, GameConfig>,
        pub system_program: Program<'info, System>,
    }

    #[derive(Accounts)]
    pub struct UpdateGameConfig<'info> {
        #[account(mut)]
        pub admin: Signer<'info>,
        #[account(
            mut,
            seeds = [b"game_config"],
            bump = game_config.bump,
            has_one = admin @ GameStateError::Unauthorized
        )]
        pub game_config: Account<'info, GameConfig>,
    }

     #[derive(Accounts)]
    pub struct ReleaseResourcesFromMarket<'info> {
        #[account(mut)]
        pub seller_planet: Account<'info, PlanetState>,

        pub market_authority: Signer<'info>,
    }

    #[derive(Accounts)]
    pub struct TransferResourcesFromMarket<'info> {
        #[account(mut)]
        pub buyer_planet: Account<'info, PlanetState>,

        pub market_authority: Signer<'info>,

        #[account(mut)]
        pub buyer: Signer<'info>,
    }

    /// Vault-signed planet creation (homeworld or colony).
    /// Both `planet_state` and `planet_coords` are initialized atomically inside
    /// `create_planet_state`, which verifies the coords PDA seeds manually and
    /// uses a CPI to System::create_account.  The `planet_coords` account is
    /// passed as a writable unchecked account so that the same context struct
    /// works for both `initialize_homeworld` and `initialize_colony` (which have
    /// different param types and therefore different coordinate values).
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
        /// CHECK: verified and initialized manually inside `create_planet_state`
        /// using `find_program_address` against the coords stored in params.
        /// Anchor cannot derive the seeds here because this struct is shared
        /// between two instructions with different param types.
        #[account(mut)]
        pub planet_coords: UncheckedAccount<'info>,
        pub system_program: Program<'info, System>,
    }

    /// Wallet-signed planet mutation (fallback / recovery path).
    #[derive(Accounts)]
    pub struct MutatePlanetState<'info> {
        #[account(mut)]
        pub authority: Signer<'info>,
        #[account(mut, has_one = authority @ GameStateError::Unauthorized)]
        pub planet_state: Account<'info, PlanetState>,
    }

    #[derive(Accounts)]
    pub struct UseAntimatter<'info> {
        #[account(mut)]
        pub authority: Signer<'info>,
        #[account(
            seeds = [b"game_config"],
            bump = game_config.bump,
        )]
        pub game_config: Account<'info, GameConfig>,
        #[account(
            mut,
            has_one = authority @ GameStateError::Unauthorized
        )]
        pub planet_state: Account<'info, PlanetState>,
        #[account(address = game_config.antimatter_mint @ GameStateError::InvalidAntimatterMint)]
        pub antimatter_mint: Account<'info, Mint>,
        #[account(
            mut,
            token::mint = antimatter_mint,
            token::authority = authority
        )]
        pub user_antimatter_account: Account<'info, TokenAccount>,
        pub token_program: Program<'info, Token>,
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

    #[derive(Accounts)]
    pub struct ResolveTransport<'info> {
        #[account(mut)]
        pub authority: Signer<'info>,

        #[account(
            mut,
            has_one = authority @ GameStateError::Unauthorized
        )]
        pub source_planet: Box<Account<'info, PlanetState>>,

        #[account(mut)]
        pub destination_planet: Box<Account<'info, PlanetState>>,
    }

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
        pub source_planet: Box<Account<'info, PlanetState>>,

        #[account(mut)]
        pub destination_planet: Box<Account<'info, PlanetState>>,
    }

    /// Wallet-signed colonize resolution.
    /// The colony planet + coords must have been created beforehand (separate tx or same tx via CPI).
    #[derive(Accounts)]
    pub struct ResolveColonize<'info> {
        #[account(mut)]
        pub authority: Signer<'info>,

        #[account(
            mut,
            has_one = authority @ GameStateError::Unauthorized
        )]
        pub source_planet: Box<Account<'info, PlanetState>>,

        pub colony_planet: Box<Account<'info, PlanetState>>,
        pub colony_coords: Box<Account<'info, PlanetCoordinates>>,
    }

    /// Vault-signed colonize resolution.
    #[derive(Accounts)]
    pub struct ResolveColonizeVault<'info> {
        #[account(mut)]
        pub vault_signer: Signer<'info>,

        /// CHECK: authority comes from source_planet checks
        pub authority: UncheckedAccount<'info>,

        #[account(
            seeds = [b"authorized_vault", authority.key().as_ref()],
            bump = authorized_vault.bump,
        )]
        pub authorized_vault: Account<'info, AuthorizedVault>,

        #[account(
            mut,
            seeds = [b"planet_state", authority.key().as_ref(), &source_planet.planet_index.to_le_bytes()],
            bump = source_planet.bump,
            has_one = authority @ GameStateError::Unauthorized
        )]
        pub source_planet: Box<Account<'info, PlanetState>>,

        pub colony_planet: Box<Account<'info, PlanetState>>,
        pub colony_coords: Box<Account<'info, PlanetCoordinates>>,
    }

    /// Wallet-signed: transfer a planet to a new authority.
    /// Both wallets must have initialized their player profile.
    /// The old authority signs to authorize the transfer.
    #[derive(Accounts)]
    pub struct TransferPlanet<'info> {
        /// The current owner — must sign.
        #[account(mut)]
        pub authority: Signer<'info>,

        /// CHECK: destination wallet — just a pubkey, not required to sign.
        /// The new_authority must have already called initialize_player.
        pub new_authority: UncheckedAccount<'info>,

        /// Verify new_authority has a player profile (they've initialized their account).
        #[account(
            seeds = [b"player_profile", new_authority.key().as_ref()],
            bump = new_player_profile.bump,
        )]
        pub new_player_profile: Account<'info, PlayerProfile>,

        /// The planet being transferred — verified against old authority.
        #[account(
            mut,
            seeds = [b"planet_state", authority.key().as_ref(), &planet_state.planet_index.to_le_bytes()],
            bump = planet_state.bump,
            has_one = authority @ GameStateError::Unauthorized
        )]
        pub planet_state: Account<'info, PlanetState>,

        /// The coordinate lock for this planet — authority field also updated.
        #[account(
            mut,
            constraint = planet_coords.planet == planet_state.key() @ GameStateError::InvalidDestination,
            constraint = planet_coords.authority == authority.key() @ GameStateError::Unauthorized,
        )]
        pub planet_coords: Account<'info, PlanetCoordinates>,
    }

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
    pub struct GameConfig {
        pub admin: Pubkey,
        pub antimatter_mint: Pubkey,
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
        pub ship_build_item: u8,
        pub ship_build_qty: u32,
        pub ship_build_finish_ts: i64,
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

    #[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq)]
pub enum ResourceType {
    Metal = 0,
    Crystal = 1,
    Deuterium = 2,
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
        pub speed_factor: u8,
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
        pub ship_build_item: u8,
        pub ship_build_qty: u32,
        pub ship_build_finish_ts: i64,
    }

    /// Params for `initialize_homeworld` — galaxy/system/position are optional hints.
    /// If galaxy == 0, program derives coordinates from authority pubkey bytes.
    /// Client should pass the resolved coords (non-zero) so the `planet_coords` PDA
    /// can be derived correctly for the `InitializePlanetVault` context.
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
        #[msg("Shipyard queue is busy.")]
        ShipyardQueueBusy,
        #[msg("No ship build is currently queued.")]
        NoShipBuild,
        #[msg("The queued ship build has not finished yet.")]
        ShipBuildNotFinished,
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
        #[msg("Transfer target has not initialized a player profile.")]
        TransferTargetNotInitialized,
        #[msg("The provided ANTIMATTER mint is invalid.")]
        InvalidAntimatterMint,
        #[msg("The provided ANTIMATTER mint must use 6 decimals.")]
        InvalidAntimatterMintDecimals,
        #[msg("The provided ANTIMATTER token account is invalid.")]
        InvalidAntimatterAccount,
        #[msg("Insufficient ANTIMATTER tokens.")]
        InsufficientAntimatter,
        #[msg("There is no remaining time to accelerate.")]
        NoAccelerationNeeded,
        #[msg("The ANTIMATTER burn amount overflowed.")]
        AntimatterAmountOverflow,
        #[msg("Only the authorized market PDA may settle market resources.")]
        UnauthorizedMarket,
    }
