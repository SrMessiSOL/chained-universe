use anchor_lang::prelude::*;
pub mod contexts;
pub mod constants;
pub mod error;
pub mod instructions;
pub mod state;
pub mod utils;

use contexts::*;
pub use constants::*;
pub use error::*;
pub use state::*;

declare_id!("7yKyjQ7m8tSqvqYnV65aVV9Jwdee7KqyELeDXf6Fxkt4");

#[program]
pub mod game_state {
    use super::*;

    pub fn initialize_player(
        ctx: Context<InitializePlayer>,
        vault: Pubkey,
        expires_at: i64,
        backup_version: u8,
        backup_ciphertext: Vec<u8>,
        backup_iv: [u8; 12],
        backup_salt: [u8; 16],
        backup_kdf_salt: [u8; 16],
    ) -> Result<()> {
        instructions::initialize_player(
            ctx,
            vault,
            expires_at,
            backup_version,
            backup_ciphertext,
            backup_iv,
            backup_salt,
            backup_kdf_salt,
        )
    }

    pub fn rotate_vault(
        ctx: Context<RotateVault>,
        new_vault: Pubkey,
        expires_at: i64,
        backup_version: u8,
        backup_ciphertext: Vec<u8>,
        backup_iv: [u8; 12],
        backup_salt: [u8; 16],
        backup_kdf_salt: [u8; 16],
    ) -> Result<()> {
        instructions::rotate_vault(
            ctx,
            new_vault,
            expires_at,
            backup_version,
            backup_ciphertext,
            backup_iv,
            backup_salt,
            backup_kdf_salt,
        )
    }

    pub fn revoke_vault(ctx: Context<ManageVault>) -> Result<()> {
        instructions::revoke_vault(ctx)
    }

    pub fn extend_vault(ctx: Context<ManageVault>, expires_at: i64) -> Result<()> {
        instructions::extend_vault(ctx, expires_at)
    }

    pub fn initialize_homeworld(
        ctx: Context<InitializePlanetVault>,
        params: InitializeHomeworldParams,
    ) -> Result<()> {
        instructions::initialize_homeworld(ctx, params)
    }

    pub fn initialize_colony(
        ctx: Context<InitializePlanetVault>,
        params: InitializeColonyParams,
    ) -> Result<()> {
        instructions::initialize_colony(ctx, params)
    }

    pub fn produce(ctx: Context<MutatePlanetState>, now: i64) -> Result<()> {
        instructions::produce(ctx, now)
    }

    pub fn produce_vault(ctx: Context<MutatePlanetStateVault>, now: i64) -> Result<()> {
        instructions::produce_vault(ctx, now)
    }

    pub fn start_build(ctx: Context<MutatePlanetState>, building_idx: u8, now: i64) -> Result<()> {
        instructions::start_build(ctx, building_idx, now)
    }

    pub fn start_build_vault(
        ctx: Context<MutatePlanetStateVault>,
        building_idx: u8,
        now: i64,
    ) -> Result<()> {
        instructions::start_build_vault(ctx, building_idx, now)
    }

    pub fn finish_build(ctx: Context<MutatePlanetState>, now: i64) -> Result<()> {
        instructions::finish_build(ctx, now)
    }

    pub fn finish_build_vault(ctx: Context<MutatePlanetStateVault>, now: i64) -> Result<()> {
        instructions::finish_build_vault(ctx, now)
    }

    pub fn start_research(ctx: Context<MutatePlanetState>, tech_idx: u8, now: i64) -> Result<()> {
        instructions::start_research(ctx, tech_idx, now)
    }

    pub fn start_research_vault(
        ctx: Context<MutatePlanetStateVault>,
        tech_idx: u8,
        now: i64,
    ) -> Result<()> {
        instructions::start_research_vault(ctx, tech_idx, now)
    }

    pub fn finish_research(ctx: Context<MutatePlanetState>, now: i64) -> Result<()> {
        instructions::finish_research(ctx, now)
    }

    pub fn finish_research_vault(
        ctx: Context<MutatePlanetStateVault>,
        now: i64,
    ) -> Result<()> {
        instructions::finish_research_vault(ctx, now)
    }

    pub fn build_ship(
        ctx: Context<MutatePlanetState>,
        ship_type: u8,
        quantity: u32,
        now: i64,
    ) -> Result<()> {
        instructions::build_ship(ctx, ship_type, quantity, now)
    }

    pub fn build_ship_vault(
        ctx: Context<MutatePlanetStateVault>,
        ship_type: u8,
        quantity: u32,
        now: i64,
    ) -> Result<()> {
        instructions::build_ship_vault(ctx, ship_type, quantity, now)
    }

    pub fn finish_ship_build(ctx: Context<MutatePlanetState>, now: i64) -> Result<()> {
        instructions::finish_ship_build(ctx, now)
    }

    pub fn finish_ship_build_vault(
        ctx: Context<MutatePlanetStateVault>,
        now: i64,
    ) -> Result<()> {
        instructions::finish_ship_build_vault(ctx, now)
    }

    pub fn launch_fleet(ctx: Context<MutatePlanetState>, params: LaunchFleetParams) -> Result<()> {
        instructions::launch_fleet(ctx, params)
    }

    pub fn launch_fleet_vault(
        ctx: Context<MutatePlanetStateVault>,
        params: LaunchFleetParams,
    ) -> Result<()> {
        instructions::launch_fleet_vault(ctx, params)
    }

    pub fn lock_resources_for_market(
        ctx: Context<MutatePlanetState>,
        resource_type: u8,
        amount: u64,
    ) -> Result<()> {
        instructions::lock_resources_for_market(ctx, resource_type, amount)
    }

    pub fn release_resources_from_market(
        ctx: Context<ReleaseResourcesFromMarket>,
        resource_type: u8,
        amount: u64,
    ) -> Result<()> {
        instructions::release_resources_from_market(ctx, resource_type, amount)
    }

    pub fn transfer_resources_from_market(
        ctx: Context<TransferResourcesFromMarket>,
        resource_type: u8,
        amount: u64,
    ) -> Result<()> {
        instructions::transfer_resources_from_market(ctx, resource_type, amount)
    }

    pub fn resolve_transport(ctx: Context<ResolveTransport>, slot: u8, now: i64) -> Result<()> {
        instructions::resolve_transport(ctx, slot, now)
    }

    pub fn resolve_transport_vault(
        ctx: Context<ResolveTransportVault>,
        slot: u8,
        now: i64,
    ) -> Result<()> {
        instructions::resolve_transport_vault(ctx, slot, now)
    }

    pub fn resolve_transport_empty(
        ctx: Context<MutatePlanetState>,
        slot: u8,
        now: i64,
    ) -> Result<()> {
        instructions::resolve_transport_empty(ctx, slot, now)
    }

    pub fn resolve_transport_empty_vault(
        ctx: Context<MutatePlanetStateVault>,
        slot: u8,
        now: i64,
    ) -> Result<()> {
        instructions::resolve_transport_empty_vault(ctx, slot, now)
    }

    pub fn resolve_colonize(ctx: Context<ResolveColonize>, slot: u8, now: i64) -> Result<()> {
        instructions::resolve_colonize(ctx, slot, now)
    }

    pub fn resolve_colonize_vault(
        ctx: Context<ResolveColonizeVault>,
        slot: u8,
        now: i64,
    ) -> Result<()> {
        instructions::resolve_colonize_vault(ctx, slot, now)
    }

    pub fn transfer_planet(ctx: Context<TransferPlanet>) -> Result<()> {
        instructions::transfer_planet(ctx)
    }

    pub fn initialize_game_config(
        ctx: Context<InitializeGameConfig>,
        antimatter_mint: Pubkey,
    ) -> Result<()> {
        instructions::initialize_game_config(ctx, antimatter_mint)
    }

    pub fn update_antimatter_mint(
        ctx: Context<UpdateGameConfig>,
        antimatter_mint: Pubkey,
    ) -> Result<()> {
        instructions::update_antimatter_mint(ctx, antimatter_mint)
    }

    pub fn accelerate_build_with_antimatter(ctx: Context<UseAntimatter>) -> Result<()> {
        instructions::accelerate_build_with_antimatter(ctx)
    }

    pub fn accelerate_research_with_antimatter(ctx: Context<UseAntimatter>) -> Result<()> {
        instructions::accelerate_research_with_antimatter(ctx)
    }

    pub fn accelerate_ship_build_with_antimatter(ctx: Context<UseAntimatter>) -> Result<()> {
        instructions::accelerate_ship_build_with_antimatter(ctx)
    }
}
