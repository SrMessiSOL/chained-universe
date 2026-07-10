use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token, TokenAccount};

use crate::constants::*;
use crate::error::GameStateError;
use crate::state::*;

#[derive(Accounts)]
pub struct InitializePlayer<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(
        init,
        payer = authority,
        space = PLAYER_PROFILE_SPACE,
        seeds = [b"player_profile", authority.key().as_ref()],
        bump
    )]
    pub player_profile: Box<Account<'info, PlayerProfile>>,
    #[account(
        init,
        payer = authority,
        space = AUTHORIZED_VAULT_SPACE,
        seeds = [b"authorized_vault", authority.key().as_ref()],
        bump
    )]
    pub authorized_vault: Box<Account<'info, AuthorizedVault>>,
    #[account(
        init,
        payer = authority,
        space = VAULT_BACKUP_SPACE,
        seeds = [b"vault_backup", authority.key().as_ref()],
        bump
    )]
    pub vault_backup: Account<'info, VaultBackup>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct InitializeVaultForExistingPlayer<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(
        seeds = [b"player_profile", authority.key().as_ref()],
        bump = player_profile.bump,
        has_one = authority @ GameStateError::Unauthorized
    )]
    pub player_profile: Box<Account<'info, PlayerProfile>>,
    #[account(
        init,
        payer = authority,
        space = AUTHORIZED_VAULT_SPACE,
        seeds = [b"authorized_vault", authority.key().as_ref()],
        bump
    )]
    pub authorized_vault: Box<Account<'info, AuthorizedVault>>,
    #[account(
        init,
        payer = authority,
        space = VAULT_BACKUP_SPACE,
        seeds = [b"vault_backup", authority.key().as_ref()],
        bump
    )]
    pub vault_backup: Account<'info, VaultBackup>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct RotateVault<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(
        mut,
        seeds = [b"authorized_vault", authority.key().as_ref()],
        bump = authorized_vault.bump,
        has_one = authority @ GameStateError::Unauthorized
    )]
    pub authorized_vault: Account<'info, AuthorizedVault>,
    #[account(
        mut,
        seeds = [b"vault_backup", authority.key().as_ref()],
        bump = vault_backup.bump,
        has_one = authority @ GameStateError::Unauthorized
    )]
    pub vault_backup: Account<'info, VaultBackup>,
}

#[derive(Accounts)]
pub struct ManageVault<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(
        mut,
        seeds = [b"authorized_vault", authority.key().as_ref()],
        bump = authorized_vault.bump,
        has_one = authority @ GameStateError::Unauthorized
    )]
    pub authorized_vault: Account<'info, AuthorizedVault>,
}

#[derive(Accounts)]
pub struct InitializeGameConfig<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    #[account(
        init,
        payer = admin,
        space = GAME_CONFIG_SPACE,
        seeds = [b"game_config"],
        bump
    )]
    pub game_config: Account<'info, GameConfig>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct UpdateGameConfig<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    #[account(
        mut,
        seeds = [b"game_config"],
        bump = game_config.bump,
        has_one = admin @ GameStateError::Unauthorized
    )]
    pub game_config: Account<'info, GameConfig>,
}

#[derive(Accounts)]
pub struct ClaimAntimatterFaucet<'info> {
    #[account(mut)]
    pub faucet_authority: Signer<'info>,
    /// CHECK: recipient wallet used as the faucet cooldown seed and token authority.
    pub recipient: UncheckedAccount<'info>,
    #[account(
        init_if_needed,
        payer = faucet_authority,
        space = FAUCET_CLAIM_SPACE,
        seeds = [b"antimatter_faucet", recipient.key().as_ref()],
        bump
    )]
    pub faucet_claim: Account<'info, FaucetClaim>,
    #[account(mut, address = PROTOCOL_ANTIMATTER_MINT @ GameStateError::InvalidAntimatterMint)]
    pub antimatter_mint: Account<'info, Mint>,
    #[account(
        mut,
        token::mint = antimatter_mint,
        token::authority = recipient
    )]
    pub recipient_antimatter_account: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct ReleaseResourcesFromMarket<'info> {
    #[account(mut)]
    pub seller_planet: Account<'info, PlanetState>,

    pub market_authority: Signer<'info>,
}

#[derive(Accounts)]
pub struct TransferResourcesFromMarket<'info> {
    #[account(mut)]
    pub buyer_planet: Account<'info, PlanetState>,

    pub market_authority: Signer<'info>,

    #[account(mut)]
    pub buyer: Signer<'info>,
}

#[derive(Accounts)]
pub struct InitializePlanetVault<'info> {
    #[account(mut)]
    pub vault_signer: Signer<'info>,
    /// CHECK: authority is read from player_profile.authority and only used as a seed/reference.
    pub authority: UncheckedAccount<'info>,
    /// CHECK: validated manually in initialize_homeworld/initialize_colony.
    pub authorized_vault: UncheckedAccount<'info>,
    #[account(
        mut,
        seeds = [b"player_profile", authority.key().as_ref()],
        bump = player_profile.bump,
        has_one = authority @ GameStateError::Unauthorized
    )]
    pub player_profile: Box<Account<'info, PlayerProfile>>,
    #[account(
        init,
        payer = vault_signer,
        space = PLANET_STATE_SPACE,
        seeds = [b"planet_state", authority.key().as_ref(), &player_profile.planet_count.to_le_bytes()],
        bump
    )]
    pub planet_state: Box<Account<'info, PlanetState>>,
    /// CHECK: verified and initialized manually inside `create_planet_state`.
    #[account(mut)]
    pub planet_coords: UncheckedAccount<'info>,
    /// CHECK: PDA, owner, and contents are validated/initialized manually.
    #[account(mut)]
    pub quest_state: UncheckedAccount<'info>,
    /// CHECK: PDA, owner, and contents are validated/initialized manually.
    #[account(mut)]
    pub quest_progress: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct InitializeColonyVault<'info> {
    #[account(mut)]
    pub vault_signer: Signer<'info>,
    /// CHECK: authority is read from player_profile.authority and only used as a seed/reference.
    pub authority: UncheckedAccount<'info>,
    /// CHECK: validated manually in initialize_colony.
    pub authorized_vault: UncheckedAccount<'info>,
    #[account(
        mut,
        seeds = [b"player_profile", authority.key().as_ref()],
        bump = player_profile.bump,
        has_one = authority @ GameStateError::Unauthorized
    )]
    pub player_profile: Box<Account<'info, PlayerProfile>>,
    #[account(
        mut,
        has_one = authority @ GameStateError::Unauthorized
    )]
    pub source_planet: Box<Account<'info, PlanetState>>,
    #[account(
        init,
        payer = vault_signer,
        space = PLANET_STATE_SPACE,
        seeds = [b"planet_state", authority.key().as_ref(), &player_profile.planet_count.to_le_bytes()],
        bump
    )]
    pub planet_state: Box<Account<'info, PlanetState>>,
    /// CHECK: verified and initialized manually inside `create_planet_state`.
    #[account(mut)]
    pub planet_coords: UncheckedAccount<'info>,
    /// CHECK: PDA, owner, and contents are validated/initialized manually.
    #[account(mut)]
    pub quest_state: UncheckedAccount<'info>,
    /// CHECK: PDA, owner, and contents are validated/initialized manually.
    #[account(mut)]
    pub quest_progress: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(slot: u32)]
pub struct SyncPlanetOwnerIndexVault<'info> {
    #[account(mut)]
    pub vault_signer: Signer<'info>,
    /// CHECK: authority is validated through player profile and authorized vault.
    pub authority: UncheckedAccount<'info>,
    #[account(
        seeds = [b"authorized_vault", authority.key().as_ref()],
        bump = authorized_vault.bump,
    )]
    pub authorized_vault: Account<'info, AuthorizedVault>,
    #[account(
        mut,
        seeds = [b"player_profile", authority.key().as_ref()],
        bump = player_profile.bump,
        has_one = authority @ GameStateError::Unauthorized
    )]
    pub player_profile: Account<'info, PlayerProfile>,
    #[account(
        has_one = authority @ GameStateError::Unauthorized
    )]
    pub planet_state: Account<'info, PlanetState>,
    #[account(
        init_if_needed,
        payer = vault_signer,
        space = PLANET_OWNER_INDEX_SPACE,
        seeds = [b"planet_owner_index", authority.key().as_ref(), &slot.to_le_bytes()],
        bump
    )]
    pub owner_index: Account<'info, PlanetOwnerIndex>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct InitializePublicPlanetVault<'info> {
    #[account(mut)]
    pub vault_signer: Signer<'info>,
    /// CHECK: authority is read from player_profile.authority and only used as a seed/reference.
    pub authority: UncheckedAccount<'info>,
    #[account()]
    pub authorized_vault: Account<'info, AuthorizedVault>,
    #[account(
        mut,
        seeds = [b"player_profile", authority.key().as_ref()],
        bump = player_profile.bump,
        has_one = authority @ GameStateError::Unauthorized
    )]
    pub player_profile: Account<'info, PlayerProfile>,
    #[account(
        init,
        payer = vault_signer,
        space = PUBLIC_PLANET_STATE_SPACE,
        seeds = [b"public_planet_v2", authority.key().as_ref(), &player_profile.planet_count.to_le_bytes()],
        bump
    )]
    pub public_planet_state: Account<'info, PublicPlanetState>,
    /// CHECK: verified and initialized manually inside the V2 public planet handler.
    #[account(mut)]
    pub public_planet_coords: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct SyncPublicPlanetView<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,
    pub planet_state: Account<'info, PlanetState>,
    #[account(
        init_if_needed,
        payer = payer,
        space = PUBLIC_PLANET_STATE_SPACE,
        seeds = [b"public_planet", planet_state.key().as_ref()],
        bump
    )]
    pub public_planet_state: Account<'info, PublicPlanetState>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct InitializeQuestState<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(
        seeds = [b"player_profile", authority.key().as_ref()],
        bump = player_profile.bump,
        has_one = authority @ GameStateError::Unauthorized
    )]
    pub player_profile: Account<'info, PlayerProfile>,
    #[account(
        init,
        payer = authority,
        space = QUEST_STATE_SPACE,
        seeds = [b"quest_state", authority.key().as_ref()],
        bump
    )]
    pub quest_state: Account<'info, QuestState>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct InitializeQuestProgress<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(
        seeds = [b"player_profile", authority.key().as_ref()],
        bump = player_profile.bump,
        has_one = authority @ GameStateError::Unauthorized
    )]
    pub player_profile: Account<'info, PlayerProfile>,
    #[account(
        init,
        payer = authority,
        space = QUEST_PROGRESS_STATE_SPACE,
        seeds = [b"quest_progress", authority.key().as_ref()],
        bump
    )]
    pub quest_progress: Account<'info, QuestProgressState>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct InitializeQuestRewardTargets<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(
        seeds = [b"player_profile", authority.key().as_ref()],
        bump = player_profile.bump,
        has_one = authority @ GameStateError::Unauthorized
    )]
    pub player_profile: Account<'info, PlayerProfile>,
    #[account(
        init,
        payer = authority,
        space = QUEST_REWARD_TARGET_STATE_SPACE,
        seeds = [b"quest_reward_targets", authority.key().as_ref()],
        bump
    )]
    pub quest_reward_targets: Account<'info, QuestRewardTargetState>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct QuestAction<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(
        mut,
        seeds = [b"quest_state", authority.key().as_ref()],
        bump = quest_state.bump,
        has_one = authority @ GameStateError::Unauthorized
    )]
    pub quest_state: Account<'info, QuestState>,
    #[account(mut, has_one = authority @ GameStateError::Unauthorized)]
    pub planet_state: Account<'info, PlanetState>,
    /// CHECK: deserialized and validated manually in the handler.
    #[account(mut)]
    pub quest_progress: UncheckedAccount<'info>,
}

#[derive(Accounts)]
pub struct QuestActionVault<'info> {
    #[account(mut)]
    pub vault_signer: Signer<'info>,
    /// CHECK: authority is validated through quest state and authorized vault seeds.
    pub authority: UncheckedAccount<'info>,
    #[account(
        seeds = [b"authorized_vault", authority.key().as_ref()],
        bump = authorized_vault.bump,
    )]
    pub authorized_vault: Account<'info, AuthorizedVault>,
    #[account(
        mut,
        seeds = [b"quest_state", authority.key().as_ref()],
        bump = quest_state.bump,
        has_one = authority @ GameStateError::Unauthorized
    )]
    pub quest_state: Account<'info, QuestState>,
    /// CHECK: deserialized and validated manually in the handler.
    #[account(mut)]
    pub planet_state: UncheckedAccount<'info>,
    /// CHECK: deserialized and validated manually in the handler.
    #[account(mut)]
    pub quest_progress: UncheckedAccount<'info>,
}

#[derive(Accounts)]
pub struct InitializeStoreConfig<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    pub usdc_mint: Account<'info, Mint>,
    #[account(token::mint = usdc_mint)]
    pub treasury_usdc_account: Account<'info, TokenAccount>,
    #[account(
        init,
        payer = admin,
        space = STORE_CONFIG_SPACE,
        seeds = [b"store_config"],
        bump
    )]
    pub store_config: Account<'info, StoreConfig>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct UpdateStoreConfig<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    pub usdc_mint: Account<'info, Mint>,
    #[account(token::mint = usdc_mint)]
    pub treasury_usdc_account: Account<'info, TokenAccount>,
    #[account(
        mut,
        seeds = [b"store_config"],
        bump = store_config.bump,
        has_one = admin @ GameStateError::Unauthorized
    )]
    pub store_config: Account<'info, StoreConfig>,
}

#[derive(Accounts)]
pub struct PurchaseStorePack<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(
        seeds = [b"store_config"],
        bump = store_config.bump,
    )]
    pub store_config: Box<Account<'info, StoreConfig>>,
    #[account(
        init_if_needed,
        payer = authority,
        space = STORE_PURCHASE_STATE_SPACE,
        seeds = [b"store_purchase_state", authority.key().as_ref()],
        bump
    )]
    pub purchase_state: Box<Account<'info, StorePurchaseState>>,
    /// CHECK: deserialized and validated manually in the handler.
    #[account(mut)]
    pub planet_state: UncheckedAccount<'info>,
    #[account(address = store_config.usdc_mint @ GameStateError::InvalidUsdcMint)]
    pub usdc_mint: Box<Account<'info, Mint>>,
    #[account(
        mut,
        token::mint = usdc_mint,
        token::authority = authority
    )]
    pub user_usdc_account: Box<Account<'info, TokenAccount>>,
    #[account(mut, token::mint = usdc_mint)]
    pub treasury_usdc_account: Box<Account<'info, TokenAccount>>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct CreateAlliance<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(
        seeds = [b"player_profile", authority.key().as_ref()],
        bump = player_profile.bump,
        has_one = authority @ GameStateError::Unauthorized
    )]
    pub player_profile: Box<Account<'info, PlayerProfile>>,
    #[account(
        init,
        payer = authority,
        space = ALLIANCE_STATE_SPACE,
        seeds = [b"alliance", authority.key().as_ref()],
        bump
    )]
    pub alliance: Box<Account<'info, AllianceState>>,
    #[account(
        init,
        payer = authority,
        space = ALLIANCE_METADATA_SPACE,
        seeds = [b"alliance_metadata", alliance.key().as_ref()],
        bump
    )]
    pub metadata: Box<Account<'info, AllianceMetadata>>,
    #[account(
        init,
        payer = authority,
        space = ALLIANCE_MEMBERSHIP_SPACE,
        seeds = [b"alliance_membership", authority.key().as_ref()],
        bump
    )]
    pub membership: Box<Account<'info, AllianceMembership>>,
    #[account(
        seeds = [b"game_config"],
        bump = game_config.bump,
    )]
    pub game_config: Box<Account<'info, GameConfig>>,
    #[account(
        seeds = [b"store_config"],
        bump = store_config.bump,
    )]
    pub store_config: Box<Account<'info, StoreConfig>>,
    #[account(address = game_config.antimatter_mint @ GameStateError::InvalidAntimatterMint)]
    pub antimatter_mint: Box<Account<'info, Mint>>,
    #[account(
        mut,
        token::mint = antimatter_mint,
        token::authority = authority
    )]
    pub user_antimatter_account: Box<Account<'info, TokenAccount>>,
    #[account(mut, token::mint = antimatter_mint)]
    pub treasury_antimatter_account: Box<Account<'info, TokenAccount>>,
    #[account(address = store_config.usdc_mint @ GameStateError::InvalidUsdcMint)]
    pub usdc_mint: Box<Account<'info, Mint>>,
    #[account(
        mut,
        token::mint = usdc_mint,
        token::authority = authority
    )]
    pub user_usdc_account: Box<Account<'info, TokenAccount>>,
    #[account(mut, token::mint = usdc_mint)]
    pub treasury_usdc_account: Box<Account<'info, TokenAccount>>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct JoinAlliance<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(
        seeds = [b"player_profile", authority.key().as_ref()],
        bump = player_profile.bump,
        has_one = authority @ GameStateError::Unauthorized
    )]
    pub player_profile: Account<'info, PlayerProfile>,
    #[account(mut)]
    pub alliance: Box<Account<'info, AllianceState>>,
    #[account(
        init,
        payer = authority,
        space = ALLIANCE_MEMBERSHIP_SPACE,
        seeds = [b"alliance_membership", authority.key().as_ref()],
        bump
    )]
    pub membership: Box<Account<'info, AllianceMembership>>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct RequestJoinAlliance<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(
        seeds = [b"player_profile", authority.key().as_ref()],
        bump = player_profile.bump,
        has_one = authority @ GameStateError::Unauthorized
    )]
    pub player_profile: Account<'info, PlayerProfile>,
    pub alliance: Account<'info, AllianceState>,
    #[account(
        init,
        payer = authority,
        space = ALLIANCE_JOIN_REQUEST_SPACE,
        seeds = [b"alliance_join_request", alliance.key().as_ref(), authority.key().as_ref()],
        bump
    )]
    pub join_request: Account<'info, AllianceJoinRequest>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct RequestJoinAllianceVault<'info> {
    #[account(mut)]
    pub vault_signer: Signer<'info>,
    /// CHECK: authority is validated through player profile and authorized vault seeds.
    pub authority: UncheckedAccount<'info>,
    #[account(
        seeds = [b"authorized_vault", authority.key().as_ref()],
        bump = authorized_vault.bump,
    )]
    pub authorized_vault: Account<'info, AuthorizedVault>,
    #[account(
        seeds = [b"player_profile", authority.key().as_ref()],
        bump = player_profile.bump,
        has_one = authority @ GameStateError::Unauthorized
    )]
    pub player_profile: Account<'info, PlayerProfile>,
    pub alliance: Account<'info, AllianceState>,
    #[account(
        init,
        payer = vault_signer,
        space = ALLIANCE_JOIN_REQUEST_SPACE,
        seeds = [b"alliance_join_request", alliance.key().as_ref(), authority.key().as_ref()],
        bump
    )]
    pub join_request: Account<'info, AllianceJoinRequest>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct ApproveJoinRequest<'info> {
    #[account(mut)]
    pub leader: Signer<'info>,
    #[account(mut)]
    pub alliance: Account<'info, AllianceState>,
    #[account(
        seeds = [b"alliance_membership", leader.key().as_ref()],
        bump = leader_membership.bump,
        has_one = alliance @ GameStateError::InvalidAllianceMember,
        constraint = leader_membership.role == 2 @ GameStateError::AllianceLeaderRequired
    )]
    pub leader_membership: Account<'info, AllianceMembership>,
    /// CHECK: applicant is stored in the join request and used as the membership PDA seed.
    pub applicant: UncheckedAccount<'info>,
    #[account(
        mut,
        seeds = [b"alliance_join_request", alliance.key().as_ref(), applicant.key().as_ref()],
        bump = join_request.bump,
        has_one = alliance @ GameStateError::InvalidAllianceMember,
        constraint = join_request.applicant == applicant.key() @ GameStateError::InvalidAllianceMember,
        close = applicant
    )]
    pub join_request: Account<'info, AllianceJoinRequest>,
    #[account(
        init,
        payer = leader,
        space = ALLIANCE_MEMBERSHIP_SPACE,
        seeds = [b"alliance_membership", applicant.key().as_ref()],
        bump
    )]
    pub membership: Account<'info, AllianceMembership>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct RejectJoinRequest<'info> {
    #[account(mut)]
    pub leader: Signer<'info>,
    pub alliance: Account<'info, AllianceState>,
    #[account(
        seeds = [b"alliance_membership", leader.key().as_ref()],
        bump = leader_membership.bump,
        has_one = alliance @ GameStateError::InvalidAllianceMember,
        constraint = leader_membership.role == 2 @ GameStateError::AllianceLeaderRequired
    )]
    pub leader_membership: Account<'info, AllianceMembership>,
    /// CHECK: applicant is stored in the join request and receives closed rent.
    #[account(mut)]
    pub applicant: UncheckedAccount<'info>,
    #[account(
        mut,
        seeds = [b"alliance_join_request", alliance.key().as_ref(), applicant.key().as_ref()],
        bump = join_request.bump,
        has_one = alliance @ GameStateError::InvalidAllianceMember,
        constraint = join_request.applicant == applicant.key() @ GameStateError::InvalidAllianceMember,
        close = applicant
    )]
    pub join_request: Account<'info, AllianceJoinRequest>,
}

#[derive(Accounts)]
pub struct ExpelAllianceMember<'info> {
    #[account(mut)]
    pub leader: Signer<'info>,
    #[account(mut)]
    pub alliance: Account<'info, AllianceState>,
    #[account(
        seeds = [b"alliance_membership", leader.key().as_ref()],
        bump = leader_membership.bump,
        has_one = alliance @ GameStateError::InvalidAllianceMember,
        constraint = leader_membership.role == 2 @ GameStateError::AllianceLeaderRequired
    )]
    pub leader_membership: Account<'info, AllianceMembership>,
    /// CHECK: target is the member authority and receives closed rent.
    #[account(mut)]
    pub target: UncheckedAccount<'info>,
    #[account(
        mut,
        seeds = [b"alliance_membership", target.key().as_ref()],
        bump = target_membership.bump,
        has_one = alliance @ GameStateError::InvalidAllianceMember,
        close = target
    )]
    pub target_membership: Account<'info, AllianceMembership>,
}

#[derive(Accounts)]
pub struct TransferAllianceLeadership<'info> {
    #[account(mut)]
    pub leader: Signer<'info>,
    #[account(mut)]
    pub alliance: Account<'info, AllianceState>,
    #[account(
        mut,
        seeds = [b"alliance_membership", leader.key().as_ref()],
        bump = leader_membership.bump,
        has_one = alliance @ GameStateError::InvalidAllianceMember,
        constraint = leader_membership.role == 2 @ GameStateError::AllianceLeaderRequired
    )]
    pub leader_membership: Account<'info, AllianceMembership>,
    /// CHECK: new leader authority is used as the membership PDA seed.
    pub new_leader: UncheckedAccount<'info>,
    #[account(
        mut,
        seeds = [b"alliance_membership", new_leader.key().as_ref()],
        bump = new_leader_membership.bump,
        has_one = alliance @ GameStateError::InvalidAllianceMember
    )]
    pub new_leader_membership: Account<'info, AllianceMembership>,
}

#[derive(Accounts)]
pub struct LeaveAlliance<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(mut)]
    pub alliance: Account<'info, AllianceState>,
    #[account(
        mut,
        seeds = [b"alliance_membership", authority.key().as_ref()],
        bump = membership.bump,
        has_one = alliance @ GameStateError::InvalidAllianceMember,
        has_one = authority @ GameStateError::Unauthorized,
        close = authority
    )]
    pub membership: Account<'info, AllianceMembership>,
}

#[derive(Accounts)]
pub struct InitializeAllianceTreasury<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    pub alliance: Account<'info, AllianceState>,
    #[account(
        seeds = [b"alliance_membership", authority.key().as_ref()],
        bump = membership.bump,
        has_one = authority @ GameStateError::Unauthorized,
        has_one = alliance @ GameStateError::InvalidAllianceMember
    )]
    pub membership: Account<'info, AllianceMembership>,
    #[account(
        init,
        payer = authority,
        space = ALLIANCE_TREASURY_SPACE,
        seeds = [b"alliance_treasury", alliance.key().as_ref()],
        bump
    )]
    pub alliance_treasury: Box<Account<'info, AllianceTreasuryState>>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct InitializeAllianceTreasuryVault<'info> {
    #[account(mut)]
    pub vault_signer: Signer<'info>,
    /// CHECK: authority is validated through membership and authorized vault seeds.
    pub authority: UncheckedAccount<'info>,
    #[account(
        seeds = [b"authorized_vault", authority.key().as_ref()],
        bump = authorized_vault.bump,
    )]
    pub authorized_vault: Account<'info, AuthorizedVault>,
    pub alliance: Account<'info, AllianceState>,
    #[account(
        seeds = [b"alliance_membership", authority.key().as_ref()],
        bump = membership.bump,
        has_one = authority @ GameStateError::Unauthorized,
        has_one = alliance @ GameStateError::InvalidAllianceMember
    )]
    pub membership: Account<'info, AllianceMembership>,
    #[account(
        init,
        payer = vault_signer,
        space = ALLIANCE_TREASURY_SPACE,
        seeds = [b"alliance_treasury", alliance.key().as_ref()],
        bump
    )]
    pub alliance_treasury: Box<Account<'info, AllianceTreasuryState>>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct DepositAllianceResources<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    /// CHECK: deserialized and validated manually in the handler.
    #[account(mut)]
    pub alliance: UncheckedAccount<'info>,
    /// CHECK: deserialized and validated manually in the handler.
    #[account(mut)]
    pub membership: UncheckedAccount<'info>,
    /// CHECK: deserialized and validated manually in the handler.
    #[account(mut)]
    pub alliance_treasury: UncheckedAccount<'info>,
    /// CHECK: deserialized and validated manually in the handler.
    #[account(mut)]
    pub planet_state: UncheckedAccount<'info>,
    /// CHECK: deserialized and validated manually in the handler.
    #[account()]
    pub game_config: UncheckedAccount<'info>,
    /// CHECK: deserialized and validated manually in the handler.
    #[account()]
    pub store_config: UncheckedAccount<'info>,
    /// CHECK: deserialized and validated manually in the handler.
    #[account()]
    pub antimatter_mint: UncheckedAccount<'info>,
    /// CHECK: deserialized and validated manually in the handler.
    #[account(mut)]
    pub user_antimatter_account: UncheckedAccount<'info>,
    /// CHECK: deserialized and validated manually in the handler.
    #[account(mut)]
    pub treasury_antimatter_account: UncheckedAccount<'info>,
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct DepositAllianceResourcesVault<'info> {
    #[account(mut)]
    pub vault_signer: Signer<'info>,
    /// CHECK: authority is validated through planet, membership, and authorized vault seeds.
    pub authority: UncheckedAccount<'info>,
    /// CHECK: deserialized and validated manually in the handler.
    #[account()]
    pub authorized_vault: UncheckedAccount<'info>,
    /// CHECK: deserialized and validated manually in the handler.
    #[account(mut)]
    pub alliance: UncheckedAccount<'info>,
    /// CHECK: deserialized and validated manually in the handler.
    #[account(mut)]
    pub membership: UncheckedAccount<'info>,
    /// CHECK: deserialized and validated manually in the handler.
    #[account(mut)]
    pub alliance_treasury: UncheckedAccount<'info>,
    /// CHECK: deserialized and validated manually in the handler.
    #[account(mut)]
    pub planet_state: UncheckedAccount<'info>,
    /// CHECK: deserialized and validated manually in the handler.
    #[account()]
    pub game_config: UncheckedAccount<'info>,
    /// CHECK: deserialized and validated manually in the handler.
    #[account()]
    pub store_config: UncheckedAccount<'info>,
    /// CHECK: deserialized and validated manually in the handler.
    #[account()]
    pub antimatter_mint: UncheckedAccount<'info>,
    /// CHECK: deserialized and validated manually in the handler.
    #[account(mut)]
    pub user_antimatter_account: UncheckedAccount<'info>,
    /// CHECK: deserialized and validated manually in the handler.
    #[account(mut)]
    pub treasury_antimatter_account: UncheckedAccount<'info>,
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct UpgradeAllianceBuilding<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(mut)]
    pub alliance: Account<'info, AllianceState>,
    #[account(
        seeds = [b"alliance_membership", authority.key().as_ref()],
        bump = membership.bump,
        has_one = authority @ GameStateError::Unauthorized,
        has_one = alliance @ GameStateError::InvalidAllianceMember
    )]
    pub membership: Account<'info, AllianceMembership>,
    #[account(
        mut,
        seeds = [b"alliance_treasury", alliance.key().as_ref()],
        bump = alliance_treasury.bump,
        has_one = alliance @ GameStateError::InvalidAllianceMember
    )]
    pub alliance_treasury: Account<'info, AllianceTreasuryState>,
}

#[derive(Accounts)]
pub struct AllianceMissionAction<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(mut)]
    pub alliance: Account<'info, AllianceState>,
    #[account(
        mut,
        seeds = [b"alliance_membership", authority.key().as_ref()],
        bump = membership.bump,
        has_one = alliance @ GameStateError::InvalidAllianceMember,
        has_one = authority @ GameStateError::Unauthorized
    )]
    pub membership: Account<'info, AllianceMembership>,
    #[account(has_one = authority @ GameStateError::Unauthorized)]
    pub planet_state: Account<'info, PlanetState>,
}

#[derive(Accounts)]
pub struct MutatePlanetState<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(mut, has_one = authority @ GameStateError::Unauthorized)]
    pub planet_state: Account<'info, PlanetState>,
}

#[derive(Accounts)]
pub struct UseAntimatter<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    #[account(
        seeds = [b"game_config"],
        bump = game_config.bump,
    )]
    pub game_config: Account<'info, GameConfig>,
    #[account(
        mut,
        has_one = authority @ GameStateError::Unauthorized
    )]
    pub planet_state: Account<'info, PlanetState>,
    #[account(address = game_config.antimatter_mint @ GameStateError::InvalidAntimatterMint)]
    pub antimatter_mint: Account<'info, Mint>,
    #[account(
        mut,
        token::mint = antimatter_mint,
        token::authority = authority
    )]
    pub user_antimatter_account: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct MutatePlanetStateVault<'info> {
    #[account(mut)]
    pub vault_signer: Signer<'info>,
    #[account()]
    pub authorized_vault: Account<'info, AuthorizedVault>,
    /// CHECK: deserialized and validated manually in the handler.
    #[account(mut)]
    pub planet_state: UncheckedAccount<'info>,
}

#[derive(Accounts)]
pub struct ResolveTransport<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(
        mut,
        has_one = authority @ GameStateError::Unauthorized
    )]
    pub source_planet: Box<Account<'info, PlanetState>>,

    #[account(mut)]
    pub destination_planet: Box<Account<'info, PlanetState>>,
}

#[derive(Accounts)]
pub struct ResolveTransportVault<'info> {
    #[account(mut)]
    pub vault_signer: Signer<'info>,

    #[account(
        seeds = [b"authorized_vault", source_planet.authority.as_ref()],
        bump = authorized_vault.bump,
    )]
    pub authorized_vault: Account<'info, AuthorizedVault>,

    #[account(mut)]
    pub source_planet: Box<Account<'info, PlanetState>>,

    #[account(mut)]
    pub destination_planet: Box<Account<'info, PlanetState>>,
}

#[derive(Accounts)]
pub struct ResolveAttack<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(
        mut,
        has_one = authority @ GameStateError::Unauthorized
    )]
    pub source_planet: Box<Account<'info, PlanetState>>,

    #[account(mut)]
    pub destination_planet: Box<Account<'info, PlanetState>>,

    #[account(
        mut,
        constraint = destination_coords.planet == destination_planet.key() @ GameStateError::InvalidDestination
    )]
    pub destination_coords: Box<Account<'info, PlanetCoordinates>>,
}

#[derive(Accounts)]
pub struct ResolveAttackVault<'info> {
    #[account(mut)]
    pub vault_signer: Signer<'info>,

    #[account(
        seeds = [b"authorized_vault", source_planet.authority.as_ref()],
        bump = authorized_vault.bump,
    )]
    pub authorized_vault: Account<'info, AuthorizedVault>,

    #[account(mut)]
    pub source_planet: Box<Account<'info, PlanetState>>,

    #[account(mut)]
    pub destination_planet: Box<Account<'info, PlanetState>>,

    #[account(
        mut,
        constraint = destination_coords.planet == destination_planet.key() @ GameStateError::InvalidDestination
    )]
    pub destination_coords: Box<Account<'info, PlanetCoordinates>>,
}

#[derive(Accounts)]
pub struct ResolveColonize<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(
        mut,
        has_one = authority @ GameStateError::Unauthorized
    )]
    pub source_planet: Box<Account<'info, PlanetState>>,

    pub colony_planet: Box<Account<'info, PlanetState>>,
    pub colony_coords: Box<Account<'info, PlanetCoordinates>>,
}

#[derive(Accounts)]
pub struct ResolveColonizeVault<'info> {
    #[account(mut)]
    pub vault_signer: Signer<'info>,

    /// CHECK: authority is validated through source_planet and authorized_vault seeds.
    pub authority: UncheckedAccount<'info>,

    #[account(
        seeds = [b"authorized_vault", authority.key().as_ref()],
        bump = authorized_vault.bump,
    )]
    pub authorized_vault: Account<'info, AuthorizedVault>,

    #[account(
        mut,
        has_one = authority @ GameStateError::Unauthorized
    )]
    pub source_planet: Box<Account<'info, PlanetState>>,

    pub colony_planet: Box<Account<'info, PlanetState>>,
    pub colony_coords: Box<Account<'info, PlanetCoordinates>>,
}

#[derive(Accounts)]
pub struct TransferPlanet<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    /// CHECK: destination wallet is only used as a pubkey and PDA seed input.
    pub new_authority: UncheckedAccount<'info>,

    #[account(
        mut,
        seeds = [b"player_profile", new_authority.key().as_ref()],
        bump,
        constraint = new_player_profile.authority == new_authority.key() @ GameStateError::Unauthorized
    )]
    pub new_player_profile: Account<'info, PlayerProfile>,

    #[account(
        mut,
        seeds = [b"planet_state", authority.key().as_ref(), &planet_state.planet_index.to_le_bytes()],
        bump = planet_state.bump,
        has_one = authority @ GameStateError::Unauthorized
    )]
    pub planet_state: Account<'info, PlanetState>,

    #[account(
        mut,
        constraint = planet_coords.planet == planet_state.key() @ GameStateError::InvalidDestination,
        constraint = planet_coords.authority == authority.key() @ GameStateError::Unauthorized,
    )]
    pub planet_coords: Account<'info, PlanetCoordinates>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct TransferPlanetFromMarket<'info> {
    /// CHECK: seller wallet is stored in planet_state.authority and only used as a seed/input.
    pub seller: UncheckedAccount<'info>,

    #[account(mut)]
    pub new_authority: Signer<'info>,

    #[account(
        init_if_needed,
        payer = new_authority,
        space = PLAYER_PROFILE_SPACE,
        seeds = [b"player_profile", new_authority.key().as_ref()],
        bump,
        constraint = new_player_profile.authority == new_authority.key() || new_player_profile.authority == Pubkey::default() @ GameStateError::Unauthorized
    )]
    pub new_player_profile: Account<'info, PlayerProfile>,

    #[account(mut)]
    pub planet_state: Account<'info, PlanetState>,

    #[account(
        mut,
        constraint = planet_coords.planet == planet_state.key() @ GameStateError::InvalidDestination,
    )]
    pub planet_coords: Account<'info, PlanetCoordinates>,

    pub market_authority: Signer<'info>,

    pub system_program: Program<'info, System>,
}
