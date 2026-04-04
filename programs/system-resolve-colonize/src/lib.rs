use bolt_lang::*;
use component_fleet::Fleet;
declare_id!("AuYuVgjpX64Fea3zGtUaEHjoewwyWBeT8Srsh8EXFhGL");

const COLONIZE_MISSION: u8 = 5;
const MIN_ARGS_LEN: usize = 9;

fn i64_at(b: &[u8], o: usize) -> i64 {
    i64::from_le_bytes(b[o..o + 8].try_into().unwrap_or([0; 8]))
}

#[system]
pub mod system_resolve_colonize {
    pub fn execute(ctx: Context<Components>, args: Vec<u8>) -> Result<Components> {
        require!(args.len() >= MIN_ARGS_LEN, ResolveColonizeError::InvalidArgs);

        let slot = args[0] as usize;
        let now = i64_at(&args, 1);
        require!(slot < 4, ResolveColonizeError::InvalidSlot);
        require!(
            ctx.accounts.source_fleet.m_type(slot) == COLONIZE_MISSION,
            ResolveColonizeError::InvalidMission
        );
        require!(
            !ctx.accounts.source_fleet.m_applied(slot),
            ResolveColonizeError::AlreadyResolved
        );
        require!(
            now >= ctx.accounts.source_fleet.m_arrive_ts(slot),
            ResolveColonizeError::MissionInFlight
        );
        require!(
            ctx.accounts.source_fleet.m_colony_ship(slot) > 0,
            ResolveColonizeError::MissingColonyShip
        );
        require!(
            (1..=9).contains(&ctx.accounts.source_fleet.m_target_galaxy(slot))
                && (1..=499).contains(&ctx.accounts.source_fleet.m_target_system(slot))
                && (1..=15).contains(&ctx.accounts.source_fleet.m_target_position(slot)),
            ResolveColonizeError::InvalidTarget
        );

        ctx.accounts.source_fleet.clear_mission(slot);
        ctx.accounts.source_fleet.active_missions =
            ctx.accounts.source_fleet.active_missions.saturating_sub(1);

        Ok(ctx.accounts)
    }

    #[system_input]
    pub struct Components {
        pub source_fleet: Fleet,
    }
}

#[error_code]
pub enum ResolveColonizeError {
    #[msg("Invalid args")]
    InvalidArgs,
    #[msg("Invalid mission slot")]
    InvalidSlot,
    #[msg("Mission is not colonize")]
    InvalidMission,
    #[msg("Mission has not arrived yet")]
    MissionInFlight,
    #[msg("Mission target is invalid")]
    InvalidTarget,
    #[msg("Mission was already resolved")]
    AlreadyResolved,
    #[msg("Mission does not include a colony ship")]
    MissingColonyShip,
}
