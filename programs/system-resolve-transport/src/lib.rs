use bolt_lang::*;
use component_fleet::Fleet;
use component_resources::Resources;
use component_resources::Resources as DestinationResources;
use component_planet::Planet as DestinationPlanet;

declare_id!("DkzcueEX3ca9haAmFoHKsW7JQVFxBfeZJX1VdHSdPnYP");

const TRANSPORT_MISSION: u8 = 2;

fn i64_at(b: &[u8], o: usize) -> i64 {
    i64::from_le_bytes(b[o..o + 8].try_into().unwrap_or([0; 8]))
}

fn settle_resources(res: &mut Resources, now: i64) {
    if res.last_update_ts <= 0 || now <= res.last_update_ts {
        res.last_update_ts = now;
        return;
    }
    let dt = (now - res.last_update_ts) as u64;
    let eff_num = if res.energy_consumption == 0 {
        10_000u64
    } else {
        (res.energy_production * 10_000 / res.energy_consumption).min(10_000)
    };
    let add_res = |current: u64, rate_per_hour: u64, cap: u64| -> u64 {
        let produced = rate_per_hour
            .saturating_mul(dt)
            .saturating_mul(eff_num)
            / 3600
            / 10_000;
        current.saturating_add(produced).min(cap)
    };
    res.metal = add_res(res.metal, res.metal_hour, res.metal_cap);
    res.crystal = add_res(res.crystal, res.crystal_hour, res.crystal_cap);
    res.deuterium = add_res(res.deuterium, res.deuterium_hour, res.deuterium_cap);
    res.last_update_ts = now;
}

#[system]
pub mod system_resolve_transport {
    pub fn execute(ctx: Context<Components>, args: Vec<u8>) -> Result<Components> {
        require!(args.len() >= 9, ResolveTransportError::InvalidArgs);

        let slot = args[0] as usize;
        let now = i64_at(&args, 1);
        require!(slot < 4, ResolveTransportError::InvalidSlot);

        let mission_type = ctx.accounts.fleet.m_type(slot);
        require!(mission_type == TRANSPORT_MISSION, ResolveTransportError::InvalidMission);

        require!(
            ctx.accounts.fleet.m_target_galaxy(slot) == ctx.accounts.destination_planet.galaxy
                && ctx.accounts.fleet.m_target_system(slot) == ctx.accounts.destination_planet.system
                && ctx.accounts.fleet.m_target_position(slot) == ctx.accounts.destination_planet.position,
            ResolveTransportError::InvalidDestination
        );

        if !ctx.accounts.fleet.m_applied(slot) {
            require!(
                now >= ctx.accounts.fleet.m_arrive_ts(slot),
                ResolveTransportError::MissionInFlight
            );

            settle_resources(&mut ctx.accounts.destination_resources, now);
            ctx.accounts.destination_resources.metal = ctx.accounts
                .destination_resources
                .metal
                .saturating_add(ctx.accounts.fleet.m_cargo_metal(slot));
            ctx.accounts.destination_resources.crystal = ctx.accounts
                .destination_resources
                .crystal
                .saturating_add(ctx.accounts.fleet.m_cargo_crystal(slot));
            ctx.accounts.destination_resources.deuterium = ctx.accounts
                .destination_resources
                .deuterium
                .saturating_add(ctx.accounts.fleet.m_cargo_deuterium(slot));

            ctx.accounts.fleet.set_mission_applied(slot, true);
            return Ok(ctx.accounts);
        }

        require!(
            ctx.accounts.fleet.m_return_ts(slot) > 0 && now >= ctx.accounts.fleet.m_return_ts(slot),
            ResolveTransportError::ReturnInFlight
        );

        ctx.accounts.fleet.return_mission_ships(slot);
        ctx.accounts.fleet.clear_mission(slot);
        ctx.accounts.fleet.active_missions = ctx.accounts.fleet.active_missions.saturating_sub(1);

        Ok(ctx.accounts)
    }

    #[system_input]
    pub struct Components {
        pub fleet: Fleet,
        pub destination_planet: DestinationPlanet,
        pub destination_resources: DestinationResources,
    }
}

#[error_code]
pub enum ResolveTransportError {
    #[msg("Invalid args")]
    InvalidArgs,
    #[msg("Invalid mission slot")]
    InvalidSlot,
    #[msg("Mission is not transport")]
    InvalidMission,
    #[msg("Destination does not match the mission target")]
    InvalidDestination,
    #[msg("Mission has not arrived yet")]
    MissionInFlight,
    #[msg("Return trip has not completed yet")]
    ReturnInFlight,
}
