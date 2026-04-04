use bolt_lang::*;
use component_resources::Resources;

declare_id!("DNNJg4A1yirXgUN5cdJ4ozuG8zJVkmxB2AsWvTqVsbk4");

/// ─────────────────────────────────────────────────────────────────────────
/// Produce System
///
/// Settles pending resource production up to `now`.
/// Call before any read/mutation that depends on current balances.
///
/// Args: [0..8] now: i64 (Unix timestamp, little-endian)
///
/// Authority: only requires is_signer — NOT bolt_metadata.authority.
/// During an ER session the burner keypair signs; the ER validator
/// enforces account ownership.
/// ─────────────────────────────────────────────────────────────────────────
#[system]
pub mod system_produce {

    pub fn execute(ctx: Context<Components>, args: Vec<u8>) -> Result<Components> {
        require!(args.len() >= 8, ProduceError::InvalidArgs);

        let now = i64::from_le_bytes(args[0..8].try_into().unwrap());

        // Settle pending production (inlined — Resources has no methods)
        let res = &mut ctx.accounts.resources;
        if res.last_update_ts > 0 && now > res.last_update_ts {
            let dt = (now - res.last_update_ts) as u64;
            let eff = if res.energy_consumption == 0 {
                10_000u64
            } else {
                (res.energy_production * 10_000 / res.energy_consumption).min(10_000)
            };
            let produce = |current: u64, rate: u64, cap: u64| -> u64 {
                let gained = rate.saturating_mul(dt).saturating_mul(eff) / 3_600 / 10_000;
                current.saturating_add(gained).min(cap)
            };
            res.metal     = produce(res.metal,     res.metal_hour,     res.metal_cap);
            res.crystal   = produce(res.crystal,   res.crystal_hour,   res.crystal_cap);
            res.deuterium = produce(res.deuterium, res.deuterium_hour, res.deuterium_cap);
        }
        res.last_update_ts = now;

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