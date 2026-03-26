use anchor_lang::prelude::*;

declare_id!("N1K6B3oiseLvLrvXELjWPdPAuhPw8MjFo3oepnHd5d3");

// ── Player Registry ───────────────────────────────────────────────────────────
//
// A simple non-component, non-delegated Anchor program that stores a permanent
// mapping from wallet pubkey → (entity_pda, planet_pda).
//
// Key properties:
//   - PDA seeds: ["registry", wallet] — deterministic, derivable from any device
//   - Never touched by the BOLT delegation system
//   - Always owned by this program on Solana devnet
//   - Queryable by any client with just the wallet pubkey
//   - Written once during world initialization, never needs updating
//
// This solves the cross-device problem: a player who starts an ER session on
// one device can reconnect from any device by deriving the registry PDA from
// their wallet and fetching it directly — no getProgramAccounts needed.

#[program]
pub mod system_registry {
    use super::*;

    /// Create the registry entry for a player's entity and planet PDAs.
    /// Called once after system_initialize confirms.
    pub fn register(
        ctx: Context<Register>,
        entity_pda: Pubkey,
        planet_pda: Pubkey,
    ) -> Result<()> {
        let registry        = &mut ctx.accounts.registry;
        registry.wallet     = ctx.accounts.wallet.key();
        registry.entity_pda = entity_pda;
        registry.planet_pda = planet_pda;
        registry.created_at = Clock::get()?.unix_timestamp;
        msg!(
            "Registry: wallet={} entity={} planet={}",
            registry.wallet,
            registry.entity_pda,
            registry.planet_pda,
        );
        Ok(())
    }

    /// Update an existing registry entry (in case of planet re-initialization).
    /// Requires the same wallet — PDA constraint enforces ownership.
    pub fn update(
        ctx: Context<Update>,
        entity_pda: Pubkey,
        planet_pda: Pubkey,
    ) -> Result<()> {
        let registry        = &mut ctx.accounts.registry;
        registry.entity_pda = entity_pda;
        registry.planet_pda = planet_pda;
        msg!(
            "Registry updated: wallet={} entity={} planet={}",
            registry.wallet,
            registry.entity_pda,
            registry.planet_pda,
        );
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Register<'info> {
    /// The player wallet — pays rent and signs.
    #[account(mut)]
    pub wallet: Signer<'info>,

    /// The registry PDA for this wallet.
    /// Seeds: ["registry", wallet.key()]
    /// Use `init` (not init_if_needed) — call `update` for subsequent changes.
    #[account(
        init,
        payer  = wallet,
        space  = PlayerRegistry::SIZE,
        seeds  = [b"registry", wallet.key().as_ref()],
        bump,
    )]
    pub registry: Account<'info, PlayerRegistry>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Update<'info> {
    /// Must be the original wallet — PDA seed constraint enforces this.
    pub wallet: Signer<'info>,

    #[account(
        mut,
        seeds  = [b"registry", wallet.key().as_ref()],
        bump,
    )]
    pub registry: Account<'info, PlayerRegistry>,
}

#[account]
pub struct PlayerRegistry {
    /// The player wallet pubkey
    pub wallet:     Pubkey, // 32
    /// The BOLT entity PDA
    pub entity_pda: Pubkey, // 32
    /// The component-planet PDA
    pub planet_pda: Pubkey, // 32
    /// Unix timestamp of registration
    pub created_at: i64,    // 8
}

impl PlayerRegistry {
    // 8 discriminator + 32 + 32 + 32 + 8
    pub const SIZE: usize = 8 + 32 + 32 + 32 + 8;
}