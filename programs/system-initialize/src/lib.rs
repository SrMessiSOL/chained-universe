use bolt_lang::*;
use component_fleet::Fleet;
use component_planet::Planet;
use component_investigation::Investigation;
use component_resources::Resources;

declare_id!("GHBGdcof2e5tsPe2vP3zJYNxJscojY7J7gdRXCsgdpY9");

#[system]
pub mod system_initialize {
    pub fn execute(ctx: Context<Components>, args: Vec<u8>) -> Result<Components> {
        require!(
            ctx.accounts.planet.creator == Pubkey::default(),
            InitError::AlreadyInitialized
        );
        require!(args.len() >= 65, InitError::InvalidArgs);

        let now = i64::from_le_bytes(args[0..8].try_into().unwrap());
        let galaxy = u16::from_le_bytes(args[8..10].try_into().unwrap());
        let system = u16::from_le_bytes(args[10..12].try_into().unwrap());
        let position = args[12];

        let mut name = [0u8; 32];
        name[..19].copy_from_slice(&args[13..32]);

        let auth_key = *ctx.accounts.authority.key;
        let auth_bytes = auth_key.to_bytes();

        let galaxy_final = if galaxy == 0 { ((auth_bytes[0] as u16) % 9) + 1 } else { galaxy.min(9).max(1) };
        let system_final = if system == 0 {
            (u16::from_le_bytes([auth_bytes[1], auth_bytes[2]]) % 499) + 1
        } else {
            system.min(499).max(1)
        };
        let position_final = if position == 0 { (auth_bytes[3] % 15) + 1 } else { position.min(15).max(1) };

        let diameter = 8_000u32 + (u16::from_le_bytes([auth_bytes[4], auth_bytes[5]]) as u32 % 10_000);
        let base_temp = 120i16 - (position_final as i16 * 12);
        let temperature = (base_temp + ((auth_bytes[6] as i16) % 40 - 20)).max(-60).min(120);
        let max_fields = 163u16 + (auth_bytes[7] as u16 % 40);

        let effective_name = if name[0] == 0 {
            let mut d = [0u8; 32];
            d[..9].copy_from_slice(b"Homeworld");
            d
        } else {
            name
        };

        let mut entity_bytes = [0u8; 32];
        entity_bytes.copy_from_slice(&args[32..64]);
        let entity_pda = Pubkey::new_from_array(entity_bytes);
        let planet_index = if args.len() >= 68 {
            u32::from_le_bytes(args[64..68].try_into().unwrap_or([0; 4]))
        } else {
            0
        };

        {
            let p = &mut ctx.accounts.planet;
            p.creator = auth_key;
            p.entity = entity_pda;
            p.owner = auth_key;
            p.name = effective_name;
            p.galaxy = galaxy_final;
            p.system = system_final;
            p.position = position_final;
            p.planet_index = planet_index;
            p.diameter = diameter;
            p.temperature = temperature;
            p.max_fields = max_fields;
            p.used_fields = 3;
            p.metal_mine = 1;
            p.crystal_mine = 1;
            p.deuterium_synthesizer = 1;
            p.solar_plant = 1;
            p.build_queue_item = 255;
        }

        {
            let r = &mut ctx.accounts.resources;
            r.metal = 1_000_000;
            r.crystal = 1_000_000;
            r.deuterium = 1_000_000;
            r.metal_hour = 33;
            r.crystal_hour = 22;
            r.deuterium_hour = 14;
            r.energy_production = 22;
            r.energy_consumption = 42;
            r.metal_cap = 1_000_000;
            r.crystal_cap = 1_000_000;
            r.deuterium_cap = 1_000_000;
            r.last_update_ts = now;
        }

        {
            let f = &mut ctx.accounts.fleet;
            f.creator = auth_key;
        }

        Ok(ctx.accounts)
    }

    #[system_input]
    pub struct Components {
        pub planet: Planet,
        pub resources: Resources,
        pub fleet: Fleet,
    }
}

#[error_code]
pub enum InitError {
    #[msg("Planet already initialized")]
    AlreadyInitialized,
    #[msg("Invalid args — need at least 64 bytes")]
    InvalidArgs,
}
