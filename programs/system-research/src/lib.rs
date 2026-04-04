use bolt_lang::*;
use component_planet::Planet;
use component_investigation::Investigation as Research;
use component_resources::Resources;

declare_id!("CXwXVUeovhbpXGWpHk56SgrnH2DwoqoTSErgtrJghK5Z");

fn base_cost(idx: u8) -> (u64, u64, u64) {
    match idx {
        0 => (0, 800, 400),
        1 => (400, 0, 600),
        2 => (2000, 4000, 600),
        3 => (10000, 20000, 6000),
        4 => (0, 400, 600),
        5 => (4000, 2000, 1000),
        6 => (240000, 400000, 160000),
        _ => (0, 0, 0),
    }
}

fn lab_requirement(idx: u8) -> u8 {
    match idx {
        0 | 1 | 4 => 1,
        5 => 3,
        2 => 5,
        3 => 7,
        6 => 10,
        _ => 255,
    }
}

fn pow2(level: u8) -> u64 {
    1u64.checked_shl(level as u32).unwrap_or(u64::MAX)
}

fn cost_for_level(idx: u8, current: u8) -> (u64, u64, u64) {
    let (m, c, d) = base_cost(idx);
    let mult = pow2(current);
    (m.saturating_mul(mult), c.saturating_mul(mult), d.saturating_mul(mult))
}

fn research_seconds(next_level: u8, lab_level: u8) -> i64 {
    ((next_level as u64 * 1800) / (lab_level.max(1) as u64)).max(1) as i64
}

#[system]
pub mod system_research {
    pub fn execute(ctx: Context<Components>, args: Vec<u8>) -> Result<Components> {
        require!(args.len() >= 10, ResearchError::InvalidArgs);

        let instruction = args[0];
        let idx = args[1];
        let now = i64::from_le_bytes(args[2..10].try_into().unwrap());

        match instruction {
            0 => {
                require!(idx <= 6, ResearchError::InvalidTech);
                require!(ctx.accounts.planet.research_lab >= 1, ResearchError::LabTooLow);
                require!(ctx.accounts.research.queue_item == 255, ResearchError::QueueBusy);

                let lab_req = lab_requirement(idx);
                require!(ctx.accounts.planet.research_lab >= lab_req, ResearchError::LabTooLow);

                let current = ctx.accounts.research.level(idx);
                let next = current.saturating_add(1);
                let (cm, cc, cd) = cost_for_level(idx, current);

                require!(ctx.accounts.resources.metal >= cm, ResearchError::InsufficientMetal);
                require!(ctx.accounts.resources.crystal >= cc, ResearchError::InsufficientCrystal);
                require!(ctx.accounts.resources.deuterium >= cd, ResearchError::InsufficientDeuterium);

                ctx.accounts.resources.metal -= cm;
                ctx.accounts.resources.crystal -= cc;
                ctx.accounts.resources.deuterium -= cd;

                ctx.accounts.research.queue_item = idx;
                ctx.accounts.research.queue_target = next;
                ctx.accounts.research.research_finish_ts = now + research_seconds(next, ctx.accounts.planet.research_lab);
            }
            1 => {
                require!(ctx.accounts.research.queue_item != 255, ResearchError::NoResearch);
                require!(now >= ctx.accounts.research.research_finish_ts, ResearchError::NotFinished);

                let idx = ctx.accounts.research.queue_item;
                let target = ctx.accounts.research.queue_target;
                ctx.accounts.research.set_level(idx, target);
                ctx.accounts.research.queue_item = 255;
                ctx.accounts.research.queue_target = 0;
                ctx.accounts.research.research_finish_ts = 0;
            }
            _ => return Err(ResearchError::InvalidArgs.into()),
        }

        Ok(ctx.accounts)
    }

    #[system_input]
    pub struct Components {
        pub planet: Planet,
        pub resources: Resources,
        pub research: Research,
    }
}

#[error_code]
pub enum ResearchError {
    #[msg("Invalid args")] InvalidArgs,
    #[msg("Invalid tech")] InvalidTech,
    #[msg("Lab level too low")] LabTooLow,
    #[msg("Research queue busy")] QueueBusy,
    #[msg("No research in progress")] NoResearch,
    #[msg("Research not finished")] NotFinished,
    #[msg("Insufficient metal")] InsufficientMetal,
    #[msg("Insufficient crystal")] InsufficientCrystal,
    #[msg("Insufficient deuterium")] InsufficientDeuterium,
}