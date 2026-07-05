use anchor_lang::prelude::*;
pub mod constants;
pub mod contexts;
pub mod error;
pub mod instructions;
pub mod state;
pub mod utils;

pub use constants::*;
use contexts::*;
pub use error::*;
pub use state::*;

declare_id!("FJGxh6SKgNoTVzHj98oBsC2oaEy8ovadVJf8rDUNaEHb");

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
        ctx: Context<InitializeColonyVault>,
        params: InitializeColonyParams,
        slot: u8,
    ) -> Result<()> {
        instructions::initialize_colony(ctx, params, slot)
    }

    pub fn initialize_public_homeworld(
        ctx: Context<InitializePublicPlanetVault>,
        params: InitializeHomeworldParams,
    ) -> Result<()> {
        instructions::initialize_public_homeworld(ctx, params)
    }

    pub fn initialize_public_colony(
        ctx: Context<InitializePublicPlanetVault>,
        params: InitializeColonyParams,
    ) -> Result<()> {
        instructions::initialize_public_colony(ctx, params)
    }

    pub fn sync_public_planet_view(ctx: Context<SyncPublicPlanetView>) -> Result<()> {
        instructions::sync_public_planet_view(ctx)
    }

    pub fn initialize_quest_state(ctx: Context<InitializeQuestState>) -> Result<()> {
        instructions::initialize_quest_state(ctx)
    }

    pub fn initialize_quest_progress(ctx: Context<InitializeQuestProgress>) -> Result<()> {
        instructions::initialize_quest_progress(ctx)
    }

    pub fn initialize_quest_reward_targets(
        ctx: Context<InitializeQuestRewardTargets>,
    ) -> Result<()> {
        instructions::initialize_quest_reward_targets(ctx)
    }

    pub fn initialize_store_config(
        ctx: Context<InitializeStoreConfig>,
        enabled: bool,
    ) -> Result<()> {
        instructions::initialize_store_config(ctx, enabled)
    }

    pub fn update_store_config(ctx: Context<UpdateStoreConfig>, enabled: bool) -> Result<()> {
        instructions::update_store_config(ctx, enabled)
    }

    pub fn purchase_store_pack(
        ctx: Context<PurchaseStorePack>,
        period: u8,
        pack_id: u8,
    ) -> Result<()> {
        instructions::purchase_store_pack(ctx, period, pack_id)
    }

    pub fn daily_check_in(ctx: Context<QuestAction>) -> Result<()> {
        instructions::daily_check_in(ctx)
    }

    pub fn daily_check_in_vault(ctx: Context<QuestActionVault>) -> Result<()> {
        instructions::daily_check_in_vault(ctx)
    }

    pub fn claim_quest(ctx: Context<QuestAction>, period: u8, quest_id: u8) -> Result<()> {
        instructions::claim_quest(ctx, period, quest_id)
    }

    pub fn claim_quest_vault(
        ctx: Context<QuestActionVault>,
        period: u8,
        quest_id: u8,
    ) -> Result<()> {
        instructions::claim_quest_vault(ctx, period, quest_id)
    }

    pub fn create_alliance(
        ctx: Context<CreateAlliance>,
        name: String,
        tag: String,
        image_url: String,
    ) -> Result<()> {
        instructions::create_alliance(ctx, name, tag, image_url)
    }

    pub fn join_alliance(ctx: Context<JoinAlliance>) -> Result<()> {
        instructions::join_alliance(ctx)
    }

    pub fn request_join_alliance(ctx: Context<RequestJoinAlliance>) -> Result<()> {
        instructions::request_join_alliance(ctx)
    }

    pub fn request_join_alliance_vault(ctx: Context<RequestJoinAllianceVault>) -> Result<()> {
        instructions::request_join_alliance_vault(ctx)
    }

    pub fn approve_join_request(ctx: Context<ApproveJoinRequest>) -> Result<()> {
        instructions::approve_join_request(ctx)
    }

    pub fn reject_join_request(ctx: Context<RejectJoinRequest>) -> Result<()> {
        instructions::reject_join_request(ctx)
    }

    pub fn expel_alliance_member(ctx: Context<ExpelAllianceMember>) -> Result<()> {
        instructions::expel_alliance_member(ctx)
    }

    pub fn transfer_alliance_leadership(ctx: Context<TransferAllianceLeadership>) -> Result<()> {
        instructions::transfer_alliance_leadership(ctx)
    }

    pub fn leave_alliance(ctx: Context<LeaveAlliance>) -> Result<()> {
        instructions::leave_alliance(ctx)
    }

    pub fn claim_alliance_mission(
        ctx: Context<AllianceMissionAction>,
        period: u8,
        mission_id: u8,
    ) -> Result<()> {
        instructions::claim_alliance_mission(ctx, period, mission_id)
    }

    pub fn initialize_alliance_treasury(ctx: Context<InitializeAllianceTreasury>) -> Result<()> {
        instructions::initialize_alliance_treasury(ctx)
    }

    pub fn initialize_alliance_treasury_vault(
        ctx: Context<InitializeAllianceTreasuryVault>,
    ) -> Result<()> {
        instructions::initialize_alliance_treasury_vault(ctx)
    }

    pub fn deposit_alliance_resources(
        ctx: Context<DepositAllianceResources>,
        period: u8,
        mission_id: u8,
        metal: u64,
        crystal: u64,
        deuterium: u64,
        antimatter: u64,
    ) -> Result<()> {
        instructions::deposit_alliance_resources(
            ctx, period, mission_id, metal, crystal, deuterium, antimatter,
        )
    }

    pub fn deposit_alliance_resources_vault(
        ctx: Context<DepositAllianceResourcesVault>,
        period: u8,
        mission_id: u8,
        metal: u64,
        crystal: u64,
        deuterium: u64,
        antimatter: u64,
    ) -> Result<()> {
        instructions::deposit_alliance_resources_vault(
            ctx, period, mission_id, metal, crystal, deuterium, antimatter,
        )
    }

    pub fn upgrade_alliance_building(
        ctx: Context<UpgradeAllianceBuilding>,
        building_id: u8,
    ) -> Result<()> {
        instructions::upgrade_alliance_building(ctx, building_id)
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

    pub fn finish_research_vault(ctx: Context<MutatePlanetStateVault>, now: i64) -> Result<()> {
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

    pub fn finish_ship_build_vault(ctx: Context<MutatePlanetStateVault>, now: i64) -> Result<()> {
        instructions::finish_ship_build_vault(ctx, now)
    }

    pub fn build_defense(
        ctx: Context<MutatePlanetState>,
        defense_type: u8,
        quantity: u32,
        now: i64,
    ) -> Result<()> {
        instructions::build_defense(ctx, defense_type, quantity, now)
    }

    pub fn build_defense_vault(
        ctx: Context<MutatePlanetStateVault>,
        defense_type: u8,
        quantity: u32,
        now: i64,
    ) -> Result<()> {
        instructions::build_defense_vault(ctx, defense_type, quantity, now)
    }

    pub fn finish_defense_build(ctx: Context<MutatePlanetState>, now: i64) -> Result<()> {
        instructions::finish_defense_build(ctx, now)
    }

    pub fn finish_defense_build_vault(
        ctx: Context<MutatePlanetStateVault>,
        now: i64,
    ) -> Result<()> {
        instructions::finish_defense_build_vault(ctx, now)
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

    pub fn resolve_attack(ctx: Context<ResolveAttack>, slot: u8, now: i64) -> Result<()> {
        instructions::resolve_attack(ctx, slot, now)
    }

    pub fn resolve_attack_vault(
        ctx: Context<ResolveAttackVault>,
        slot: u8,
        now: i64,
    ) -> Result<()> {
        instructions::resolve_attack_vault(ctx, slot, now)
    }

    pub fn resolve_espionage(ctx: Context<ResolveAttack>, slot: u8, now: i64) -> Result<()> {
        instructions::resolve_espionage(ctx, slot, now)
    }

    pub fn resolve_espionage_vault(
        ctx: Context<ResolveAttackVault>,
        slot: u8,
        now: i64,
    ) -> Result<()> {
        instructions::resolve_espionage_vault(ctx, slot, now)
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

    pub fn transfer_planet_from_market(ctx: Context<TransferPlanetFromMarket>) -> Result<()> {
        instructions::transfer_planet_from_market(ctx)
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

    pub fn accelerate_defense_build_with_antimatter(ctx: Context<UseAntimatter>) -> Result<()> {
        instructions::accelerate_defense_build_with_antimatter(ctx)
    }

    pub fn accelerate_mission_with_antimatter(
        ctx: Context<UseAntimatter>,
        slot: u8,
        leg: u8,
    ) -> Result<()> {
        instructions::accelerate_mission_with_antimatter(ctx, slot, leg)
    }
}
