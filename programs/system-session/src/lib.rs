use bolt_lang::*;
use component_planet::Planet;
use component_resources::Resources;
use component_fleet::Fleet;

declare_id!("EASuSJPK7oY4wjgD5b4XUkkFw7Wp3gCwSzY3u7qwuaHj");

/// System-Session
///
/// This system exists solely to give the BOLT World program a registered entry
/// point for session management. The actual commit+undelegate CPI to the magic
/// program is performed client-side (game.ts endSession) directly against the
/// ER validator's magic program — no Rust CPI needed.
///
/// Why: The #[system] macro creates two invariant lifetime regions for
/// `ctx.remaining_accounts` and `ctx.accounts` that cannot be unified.
/// Any attempt to mix AccountInfo refs from both regions causes a compiler
/// error that cannot be resolved without changing the execute signature,
/// which in turn breaks the #[system] macro parser.
///
/// The client calls commit_and_undelegate_accounts via the ER SDK's TypeScript
/// helper directly, passing the planet/resources/fleet PDAs. This is the
/// supported pattern for BOLT + ER integration.
#[system]
pub mod system_session {

    pub fn execute(ctx: Context<Components>, args: Vec<u8>) -> Result<Components> {
        // Validate that the authority signed
        require!(
            ctx.accounts.authority.is_signer,
            SessionError::Unauthorized
        );
        // args[0] = 0 means end_session (reserved for future use)
        require!(!args.is_empty(), SessionError::InvalidArgs);
        require!(args[0] == 0, SessionError::InvalidArgs);

        // No-op: commit+undelegate is handled client-side via ER SDK
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
pub enum SessionError {
    #[msg("Invalid args — args[0] must be 0 (end_session)")]
    InvalidArgs,
    #[msg("Authority must sign the session instruction")]
    Unauthorized,
}