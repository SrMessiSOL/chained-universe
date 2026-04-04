use bolt_lang::*;
use component_planet::Planet;
use component_resources::Resources;

declare_id!("DapYcTdYUwB7qWhmqMGZU6V1vqS3NEagzt15fnWwfQMC");

const MIN_ARGS_LEN: usize = 105;

fn i64_at(b: &[u8], o: usize) -> i64 {
    i64::from_le_bytes(b[o..o + 8].try_into().unwrap_or([0; 8]))
}

fn u64_at(b: &[u8], o: usize) -> u64 {
    u64::from_le_bytes(b[o..o + 8].try_into().unwrap_or([0; 8]))
}

fn pubkey_at(b: &[u8], o: usize) -> Pubkey {
    Pubkey::new_from_array(b[o..o + 32].try_into().unwrap_or([0; 32]))
}

fn colony_name_from_args(args: &[u8]) -> [u8; 32] {
    let mut name = [0u8; 32];
    if args.len() >= 45 {
        name.copy_from_slice(&args[13..45]);
    }
    if name[0] == 0 {
        let mut default_name = [0u8; 32];
        default_name[..6].copy_from_slice(b"Colony");
        default_name
    } else {
        name
    }
}

#[system]
pub mod system_initialize_new_colony {
    pub fn execute(ctx: Context<Components>, args: Vec<u8>) -> Result<Components> {
        require!(
            ctx.accounts.planet.creator == Pubkey::default(),
            InitNewColonyError::AlreadyInitialized
        );
        require!(args.len() >= MIN_ARGS_LEN, InitNewColonyError::InvalidArgs);

        let now = i64_at(&args, 0);
        let galaxy = u16::from_le_bytes(args[8..10].try_into().unwrap_or([0; 2]));
        let system = u16::from_le_bytes(args[10..12].try_into().unwrap_or([0; 2]));
        let position = args[12];
        let name = colony_name_from_args(&args);
        let entity_pda = pubkey_at(&args, 45);
        let planet_index = u32::from_le_bytes(args[77..81].try_into().unwrap_or([0; 4]));
        let cargo_metal = u64_at(&args, 81);
        let cargo_crystal = u64_at(&args, 89);
        let cargo_deuterium = u64_at(&args, 97);

        require!((1..=9).contains(&galaxy), InitNewColonyError::InvalidTarget);
        require!((1..=499).contains(&system), InitNewColonyError::InvalidTarget);
        require!((1..=15).contains(&position), InitNewColonyError::InvalidTarget);
        require!(entity_pda != Pubkey::default(), InitNewColonyError::InvalidArgs);

        let owner = *ctx.accounts.authority.key;
        let temperature = (120i16 - (position as i16 * 12)).clamp(-60, 120);
        let diameter = 8_000u32
            + ((galaxy as u32 * 997 + system as u32 * 37 + position as u32 * 101) % 10_000);
        let max_fields = 163u16 + ((galaxy + system + position as u16) % 40);

        {
            let planet = &mut ctx.accounts.planet;
            planet.creator = owner;
            planet.entity = entity_pda;
            planet.owner = owner;
            planet.name = name;
            planet.galaxy = galaxy;
            planet.system = system;
            planet.position = position;
            planet.planet_index = planet_index;
            planet.diameter = diameter;
            planet.temperature = temperature;
            planet.max_fields = max_fields;
            planet.used_fields = 3;
            planet.metal_mine = 1;
            planet.crystal_mine = 1;
            planet.deuterium_synthesizer = 1;
            planet.solar_plant = 1;
            planet.build_queue_item = 255;
        }

        {
            let resources = &mut ctx.accounts.resources;
            resources.metal = cargo_metal;
            resources.crystal = cargo_crystal;
            resources.deuterium = cargo_deuterium;
            resources.metal_hour = 33;
            resources.crystal_hour = 22;
            resources.deuterium_hour = 14;
            resources.energy_production = 22;
            resources.energy_consumption = 42;
            resources.metal_cap = 1_000_000;
            resources.crystal_cap = 1_000_000;
            resources.deuterium_cap = 1_000_000;
            resources.last_update_ts = now;
        }

        Ok(ctx.accounts)
    }

    #[system_input]
    pub struct Components {
        pub planet: Planet,
        pub resources: Resources,
    }
}

#[error_code]
pub enum InitNewColonyError {
    #[msg("Colony already initialized")]
    AlreadyInitialized,
    #[msg("Invalid args")]
    InvalidArgs,
    #[msg("Invalid colony target")]
    InvalidTarget,
}
