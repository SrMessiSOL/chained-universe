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
///   [0..8]   now:        i64
///   [8..10]  galaxy:     u16 (0 = derive from authority)
///   [10..12] system:     u16 (0 = derive)
///   [12]     position:   u8  (0 = derive)
///   [13..32] name:       19 bytes UTF-8 (null-padded)
///   [32..64] entity_pda: 32 bytes
///
/// Registry CPI is intentionally NOT here — the BOLT World program rejects
/// writable non-owned accounts passed via extraAccounts ("signer privilege
/// escalated"). The registry is written by game.ts as a separate tx
/// directly to the registry program after this system confirms.
#[system]
pub mod system_initialize {
    pub fn execute(ctx: Context<Components>, args: Vec<u8>) -> Result<Components> {
        require!(
            ctx.accounts.planet.creator == Pubkey::default(),
            InitError::AlreadyInitialized
        );
        require!(args.len() >= 64, InitError::InvalidArgs);

        let now      = i64::from_le_bytes(args[0..8].try_into().unwrap());
        let galaxy   = u16::from_le_bytes(args[8..10].try_into().unwrap());
        let system   = u16::from_le_bytes(args[10..12].try_into().unwrap());
        let position = args[12];

        let mut name = [0u8; 32];
        name[..19].copy_from_slice(&args[13..32]);

        let auth_key   = *ctx.accounts.authority.key;
        let auth_bytes = auth_key.to_bytes();

        let galaxy_final   = if galaxy   == 0 { ((auth_bytes[0] as u16) % 9) + 1 } else { galaxy.min(9).max(1) };
        let system_final   = if system   == 0 { (u16::from_le_bytes([auth_bytes[1], auth_bytes[2]]) % 499) + 1 } else { system.min(499).max(1) };
        let position_final = if position == 0 { (auth_bytes[3] % 15) + 1 } else { position.min(15).max(1) };

        let diameter    = 8_000u32 + (u16::from_le_bytes([auth_bytes[4], auth_bytes[5]]) as u32 % 10_000);
        let base_temp   = 120i16 - (position_final as i16 * 12);
        let temperature = (base_temp + ((auth_bytes[6] as i16) % 40 - 20)).max(-60).min(120);
        let max_fields  = 163u16 + (auth_bytes[7] as u16 % 40);

        let effective_name = if name[0] == 0 {
            let mut d = [0u8; 32]; d[..9].copy_from_slice(b"Homeworld"); d
        } else { name };

        let mut entity_bytes = [0u8; 32];
        entity_bytes.copy_from_slice(&args[32..64]);
        let entity_pda = Pubkey::new_from_array(entity_bytes);

        // ── Planet ───────────────────────────────────────────────────────────
        {
            let p = &mut ctx.accounts.planet;
            p.creator = auth_key; p.entity = entity_pda; p.owner = auth_key;
            p.name = effective_name; p.galaxy = galaxy_final;
            p.system = system_final; p.position = position_final;
            p.diameter = diameter; p.temperature = temperature;
            p.max_fields = max_fields; p.used_fields = 3;
            p.metal_mine = 1; p.crystal_mine = 1; p.deuterium_synthesizer = 1;
            p.solar_plant = 1; p.build_queue_item = 255;
        }

        // ── Resources ────────────────────────────────────────────────────────
        {
            let r = &mut ctx.accounts.resources;
            r.metal = 5_000; r.crystal = 3_000; r.deuterium = 1_000;
            r.metal_hour = 33; r.crystal_hour = 22; r.deuterium_hour = 14;
            r.energy_production = 22; r.energy_consumption = 42;
            r.metal_cap = 10_000; r.crystal_cap = 10_000; r.deuterium_cap = 10_000;
            r.last_update_ts = now;
        }

        // ── Fleet ─────────────────────────────────────────────────────────────
        { ctx.accounts.fleet.creator = auth_key; }

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
    #[msg("Planet already initialized")] AlreadyInitialized,
    #[msg("Invalid args — need 64 bytes")] InvalidArgs,
}