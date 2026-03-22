use bolt_lang::*;
use component_resources::Resources;

declare_id!("EkNaTMh1N29W6PCXDGnvh7mVzcrA1pMS3uz2xKWRUZRH");

fn require_component_authority(authority: &AccountInfo, resources: &Resources) -> Result<()> {
    require!(authority.is_signer, ProduceError::Unauthorized);
    require_keys_eq!(
        resources.bolt_metadata.authority,
        *authority.key,
        ProduceError::Unauthorized
    );
    Ok(())
}

/// ─────────────────────────────────────────────────────────────────────────
/// Produce System
///
/// Settles pending resource production for a planet.
/// Call this before any read or mutation that depends on current balances.
///
/// Args:
///   [0..8] now: i64 (Unix timestamp, little-endian)
///
/// This is intentionally minimal — it only touches the Resources component.
/// In an Ephemeral Rollup session this can be cranked every second.
/// ─────────────────────────────────────────────────────────────────────────
#[system]
pub mod system_produce {

    pub fn execute(ctx: Context<Components>, args: Vec<u8>) -> Result<Components> {
        require_component_authority(&ctx.accounts.authority, &ctx.accounts.resources)?;

        require!(args.len() >= 8, ProduceError::InvalidArgs);
        let now = i64::from_le_bytes(args[0..8].try_into().unwrap());
        ctx.accounts.resources.settle(now);
        Ok(ctx.accounts)
    }

    #[system_input]
    pub struct Components {
        pub resources: Resources,
    }
}

#[error_code]
pub enum ProduceError {
    #[msg("Unauthorized")]
    Unauthorized,
    #[msg("Invalid args — need 8 bytes for timestamp")]
    InvalidArgs,
}
