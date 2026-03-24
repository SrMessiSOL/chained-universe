use bolt_lang::*;
use component_planet::Planet;
use component_resources::Resources;
use component_fleet::Fleet;

declare_id!("BvTJfpb1KMtBiKQhcNVvHJnKZAvoRALrm4GYQ2Uz36TX");

/// System-Initialize
///
/// Must be called once per player right after InitializeComponent.
/// Seeds all three components with proper starting values.
///
/// Args (64 bytes total):
///   [0..8]   now:        i64  — Unix timestamp from client
///   [8..10]  galaxy:     u16  — desired galaxy (1-9); 0 = derive from authority key
///   [10..12] system:     u16  — desired system (1-499); 0 = derive
///   [12]     position:   u8   — desired position (1-15); 0 = derive
///   [13..32] name:       19 bytes UTF-8 planet name (null-padded)
///   [32..64] entity_pda: 32 bytes — the entity PDA, stored on-chain for cache-free lookup
#[system]
pub mod system_initialize {
    pub fn execute(ctx: Context<Components>, args: Vec<u8>) -> Result<Components> {
        // Only require the caller to be a signer.
        // DO NOT check bolt_metadata.authority — it equals the World Program PDA,
        // not the user wallet, so that check always fails.
        // Must not be already initialized (creator != default)
        require!(
            ctx.accounts.planet.creator == Pubkey::default(),
            InitError::AlreadyInitialized
        );

        require!(args.len() >= 64, InitError::InvalidArgs);

        let now      = i64::from_le_bytes(args[0..8].try_into().unwrap());
        let galaxy   = u16::from_le_bytes(args[8..10].try_into().unwrap());
        let system   = u16::from_le_bytes(args[10..12].try_into().unwrap());
        let position = args[12];

        // Read 19 bytes of name (null-padded)
        let name_bytes = &args[13..32];
        let mut name = [0u8; 32];
        for (i, &b) in name_bytes.iter().enumerate().take(19) {
            name[i] = b;
        }

        // Derive coordinates from authority key if not provided
        let auth_bytes = ctx.accounts.authority.key.to_bytes();
        let galaxy_final = if galaxy == 0 {
            ((auth_bytes[0] as u16) % 9) + 1
        } else {
            galaxy.min(9).max(1)
        };
        let system_final = if system == 0 {
            (u16::from_le_bytes([auth_bytes[1], auth_bytes[2]]) % 499) + 1
        } else {
            system.min(499).max(1)
        };
        let position_final = if position == 0 {
            (auth_bytes[3] % 15) + 1
        } else {
            position.min(15).max(1)
        };

        // Derive planet physical properties from authority bytes
        let diameter = 8_000u32 + (u16::from_le_bytes([auth_bytes[4], auth_bytes[5]]) as u32 % 10_000);
        let base_temp: i16 = 120 - (position_final as i16 * 12);
        let temp_variance: i16 = (auth_bytes[6] as i16) % 40 - 20;
        let temperature = (base_temp + temp_variance).max(-60).min(120);
        let max_fields = 163u16 + (auth_bytes[7] as u16 % 40);

        // Default planet name if none provided
        let effective_name = if name[0] == 0 {
            let mut default_name = [0u8; 32];
            let n = b"Homeworld";
            default_name[..n.len()].copy_from_slice(n);
            default_name
        } else {
            name
        };

        // Initialize Planet
        let planet = &mut ctx.accounts.planet;
        planet.creator               = *ctx.accounts.authority.key; // ← wallet, used for lookup
        // Read entity PDA from args[32..64] — passed by client at creation time
        let mut entity_pda_bytes = [0u8; 32];
        entity_pda_bytes.copy_from_slice(&args[32..64]);
        planet.entity                = Pubkey::new_from_array(entity_pda_bytes);
        planet.owner                 = *ctx.accounts.authority.key;
        planet.name                  = effective_name;
        planet.galaxy                = galaxy_final;
        planet.system                = system_final;
        planet.position              = position_final;
        planet.diameter              = diameter;
        planet.temperature           = temperature;
        planet.max_fields            = max_fields;
        planet.used_fields           = 3;
        planet.metal_mine            = 1;
        planet.crystal_mine          = 1;
        planet.deuterium_synthesizer = 1;
        planet.solar_plant           = 1;
        planet.fusion_reactor        = 0;
        planet.robotics_factory      = 0;
        planet.nanite_factory        = 0;
        planet.shipyard              = 0;
        planet.metal_storage         = 0;
        planet.crystal_storage       = 0;
        planet.deuterium_tank        = 0;
        planet.research_lab          = 0;
        planet.missile_silo          = 0;
        planet.build_queue_item      = 255;
        planet.build_queue_target    = 0;
        planet.build_finish_ts       = 0;

        // Initialize Resources — use current timestamp so production starts NOW
        let resources = &mut ctx.accounts.resources;
        resources.metal              = 500;
        resources.crystal            = 500;
        resources.deuterium          = 0;
        resources.metal_hour         = 33;
        resources.crystal_hour       = 22;
        resources.deuterium_hour     = 14;
        resources.energy_production  = 22;
        resources.energy_consumption = 42;
        resources.metal_cap          = 10_000;
        resources.crystal_cap        = 10_000;
        resources.deuterium_cap      = 10_000;
        resources.last_update_ts     = now;

        // Fleet is already zeroed by Default
        let _ = &ctx.accounts.fleet;

        Ok(ctx.accounts)
    }

    #[system_input]
    pub struct Components {
        pub planet:    Planet,
        pub resources: Resources,
        pub fleet:     Fleet,
    }
}

#[error_code]
pub enum InitError {
    #[msg("Unauthorized")]
    Unauthorized,
    #[msg("Planet already initialized")]
    AlreadyInitialized,
    #[msg("Invalid args — need 64 bytes")]
    InvalidArgs,
}