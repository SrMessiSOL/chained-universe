use anchor_lang::prelude::*;

use crate::constants::*;
use crate::contexts::*;
use crate::error::GameStateError;
use crate::state::*;
use crate::utils::*;

/// One-time wallet setup: creates player profile + authorizes vault + stores encrypted backup.
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
    require!(!backup_ciphertext.is_empty(), GameStateError::InvalidArgs);
    require!(
        backup_ciphertext.len() <= 512,
        GameStateError::BackupTooLarge
    );

    let authority = ctx.accounts.authority.key();

    ctx.accounts.player_profile.set_inner(PlayerProfile {
        authority,
        planet_count: 0,
        bump: ctx.bumps.player_profile,
    });

    let now = Clock::get()?.unix_timestamp;
    require!(
        expires_at == 0 || expires_at > now,
        GameStateError::InvalidArgs
    );

    ctx.accounts.authorized_vault.set_inner(AuthorizedVault {
        authority,
        vault,
        expires_at,
        revoked: false,
        bump: ctx.bumps.authorized_vault,
    });

    ctx.accounts.vault_backup.set_inner(VaultBackup {
        authority,
        vault,
        version: backup_version,
        ciphertext: backup_ciphertext,
        iv: backup_iv,
        salt: backup_salt,
        kdf_salt: backup_kdf_salt,
        updated_at: now,
        bump: ctx.bumps.vault_backup,
    });

    Ok(())
}

/// Wallet-only: rotate vault key and update backup.
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
    require!(!backup_ciphertext.is_empty(), GameStateError::InvalidArgs);
    require!(
        backup_ciphertext.len() <= 512,
        GameStateError::BackupTooLarge
    );

    let now = Clock::get()?.unix_timestamp;
    require!(
        expires_at == 0 || expires_at > now,
        GameStateError::InvalidArgs
    );

    ctx.accounts.authorized_vault.vault = new_vault;
    ctx.accounts.authorized_vault.expires_at = expires_at;
    ctx.accounts.authorized_vault.revoked = false;

    ctx.accounts.vault_backup.vault = new_vault;
    ctx.accounts.vault_backup.version = backup_version;
    ctx.accounts.vault_backup.ciphertext = backup_ciphertext;
    ctx.accounts.vault_backup.iv = backup_iv;
    ctx.accounts.vault_backup.salt = backup_salt;
    ctx.accounts.vault_backup.kdf_salt = backup_kdf_salt;
    ctx.accounts.vault_backup.updated_at = now;

    Ok(())
}

/// Wallet-only: revoke vault access (emergency lockout).
pub fn revoke_vault(ctx: Context<ManageVault>) -> Result<()> {
    ctx.accounts.authorized_vault.revoked = true;
    Ok(())
}

/// Wallet-only: extend vault expiry.
pub fn extend_vault(ctx: Context<ManageVault>, expires_at: i64) -> Result<()> {
    let now = Clock::get()?.unix_timestamp;
    require!(
        expires_at == 0 || expires_at > now,
        GameStateError::InvalidArgs
    );
    ctx.accounts.authorized_vault.expires_at = expires_at;
    ctx.accounts.authorized_vault.revoked = false;
    Ok(())
}

/// Vault-signed: initialize homeworld.
/// Creates both `planet_state` and `planet_coords` atomically.
/// If `planet_coords` already exists for these coordinates the tx fails — client retries with new coords.
pub fn initialize_homeworld(
    ctx: Context<InitializePlanetVault>,
    params: InitializeHomeworldParams,
) -> Result<()> {
    let _ = (&ctx, &params);
    return err!(GameStateError::LegacyPlanetStateDisabled);
    #[allow(unreachable_code)]
    {
    let now = chain_now()?;
    require_active_vault(
        ctx.accounts.vault_signer.key(),
        &ctx.accounts.authorized_vault,
        ctx.accounts.player_profile.authority,
    )?;

    let authority = ctx.accounts.player_profile.authority;
    let auth_bytes = authority.to_bytes();
    let position = if params.position == 0 {
        (auth_bytes[3] % 15) + 1
    } else {
        params.position.clamp(1, 15)
    };
    let base_temp = 120i16 - (position as i16 * 12);

    let planet_params = InitializePlanetParams {
        name: if params.name.is_empty() {
            "Homeworld".to_string()
        } else {
            params.name
        },
        galaxy: if params.galaxy == 0 {
            ((auth_bytes[0] as u16) % 999) + 1
        } else {
            params.galaxy.clamp(1, 999)
        },
        system: if params.system == 0 {
            (u16::from_le_bytes([auth_bytes[1], auth_bytes[2]]) % 999) + 1
        } else {
            params.system.clamp(1, 999)
        },
        position,
        diameter: 8_000u32 + (u16::from_le_bytes([auth_bytes[4], auth_bytes[5]]) as u32 % 10_000),
        temperature: (base_temp + ((auth_bytes[6] as i16) % 40 - 20)).clamp(-60, 120),
        max_fields: 163u16 + (auth_bytes[7] as u16 % 40),
        used_fields: 4,
        metal_mine: 1,
        crystal_mine: 1,
        deuterium_synthesizer: 1,
        solar_plant: 1,
        fusion_reactor: 0,
        robotics_factory: 0,
        nanite_factory: 0,
        shipyard: 0,
        metal_storage: 0,
        crystal_storage: 0,
        deuterium_tank: 0,
        research_lab: 0,
        missile_silo: 0,
        energy_tech: 0,
        combustion_drive: 0,
        impulse_drive: 0,
        hyperspace_drive: 0,
        computer_tech: 0,
        astrophysics: 0,
        igr_network: 0,
        weapons_technology: 0,
        shielding_technology: 0,
        armor_technology: 0,
        research_queue_item: 255,
        research_queue_target: 0,
        research_finish_ts: 0,
        build_queue_item: 255,
        build_queue_target: 0,
        build_finish_ts: 0,
        ship_build_item: 255,
        ship_build_qty: 0,
        ship_build_finish_ts: 0,
        defense_build_item: 255,
        defense_build_qty: 0,
        defense_build_finish_ts: 0,
        metal: STARTING_METAL,
        crystal: STARTING_CRYSTAL,
        deuterium: STARTING_DEUTERIUM,
        metal_hour: 33,
        crystal_hour: 22,
        deuterium_hour: 14,
        energy_production: 22,
        energy_consumption: 42,
        metal_cap: BASE_STORAGE_CAP,
        crystal_cap: BASE_STORAGE_CAP,
        deuterium_cap: BASE_STORAGE_CAP,
        last_update_ts: now,
        created_at: now,
        protection_until_ts: now.saturating_add(NEW_PLAYER_PROTECTION_SECONDS),
        market_unlocked_at: now.saturating_add(MARKET_UNLOCK_SECONDS),
        attack_unlocked_at: now.saturating_add(ATTACK_UNLOCK_SECONDS),
        last_attack_launch_ts: 0,
        last_attacked_ts: 0,
        small_cargo: 0,
        large_cargo: 0,
        light_fighter: 0,
        heavy_fighter: 0,
        cruiser: 0,
        battleship: 0,
        battlecruiser: 0,
        bomber: 0,
        destroyer: 0,
        deathstar: 0,
        recycler: 0,
        espionage_probe: 0,
        colony_ship: 0,
        solar_satellite: 0,
        rocket_launcher: 0,
        light_laser: 0,
        heavy_laser: 0,
        gauss_cannon: 0,
        ion_cannon: 0,
        plasma_turret: 0,
        small_shield_dome: 0,
        large_shield_dome: 0,
        anti_ballistic_missile: 0,
        interplanetary_missile: 0,
    };

    create_planet_state(
        authority,
        &mut ctx.accounts.player_profile,
        &mut ctx.accounts.planet_state,
        &ctx.accounts.planet_coords.to_account_info(),
        &ctx.accounts.vault_signer.to_account_info(),
        &ctx.accounts.system_program.to_account_info(),
        ctx.bumps.planet_state,
        &planet_params,
    )
    }
}

/// Vault-signed: initialize colony.
/// Creates both `planet_state` and `planet_coords` atomically.
/// If `planet_coords` already exists the tx fails — client shows "slot occupied".
pub fn initialize_colony(
    ctx: Context<InitializePlanetVault>,
    params: InitializeColonyParams,
) -> Result<()> {
    let _ = (&ctx, &params);
    return err!(GameStateError::LegacyPlanetStateDisabled);
    #[allow(unreachable_code)]
    {
    let now = chain_now()?;
    require_active_vault(
        ctx.accounts.vault_signer.key(),
        &ctx.accounts.authorized_vault,
        ctx.accounts.player_profile.authority,
    )?;

    let authority = ctx.accounts.player_profile.authority;

    let planet_params = InitializePlanetParams {
        name: if params.name.is_empty() {
            "Colony".to_string()
        } else {
            params.name
        },
        galaxy: params.galaxy,
        system: params.system,
        position: params.position,
        diameter: 8_000u32
            + ((params.galaxy as u32 * 997
                + params.system as u32 * 37
                + params.position as u32 * 101)
                % 10_000),
        temperature: (120i16 - (params.position as i16 * 12)).clamp(-60, 120),
        max_fields: 163u16 + ((params.galaxy + params.system + params.position as u16) % 40),
        used_fields: 4,
        metal_mine: 1,
        crystal_mine: 1,
        deuterium_synthesizer: 1,
        solar_plant: 1,
        fusion_reactor: 0,
        robotics_factory: 0,
        nanite_factory: 0,
        shipyard: 0,
        metal_storage: 0,
        crystal_storage: 0,
        deuterium_tank: 0,
        research_lab: 0,
        missile_silo: 0,
        energy_tech: 0,
        combustion_drive: 0,
        impulse_drive: 0,
        hyperspace_drive: 0,
        computer_tech: 0,
        astrophysics: 0,
        igr_network: 0,
        weapons_technology: 0,
        shielding_technology: 0,
        armor_technology: 0,
        research_queue_item: 255,
        research_queue_target: 0,
        research_finish_ts: 0,
        build_queue_item: 255,
        build_queue_target: 0,
        build_finish_ts: 0,
        ship_build_item: 255,
        ship_build_qty: 0,
        ship_build_finish_ts: 0,
        defense_build_item: 255,
        defense_build_qty: 0,
        defense_build_finish_ts: 0,
        metal: params.cargo_metal,
        crystal: params.cargo_crystal,
        deuterium: params.cargo_deuterium,
        metal_hour: 33,
        crystal_hour: 22,
        deuterium_hour: 14,
        energy_production: 22,
        energy_consumption: 42,
        metal_cap: BASE_STORAGE_CAP,
        crystal_cap: BASE_STORAGE_CAP,
        deuterium_cap: BASE_STORAGE_CAP,
        last_update_ts: now,
        created_at: now,
        protection_until_ts: now.saturating_add(NEW_PLAYER_PROTECTION_SECONDS),
        market_unlocked_at: now.saturating_add(MARKET_UNLOCK_SECONDS),
        attack_unlocked_at: now.saturating_add(ATTACK_UNLOCK_SECONDS),
        last_attack_launch_ts: 0,
        last_attacked_ts: 0,
        small_cargo: params.small_cargo,
        large_cargo: params.large_cargo,
        light_fighter: params.light_fighter,
        heavy_fighter: params.heavy_fighter,
        cruiser: params.cruiser,
        battleship: params.battleship,
        battlecruiser: params.battlecruiser,
        bomber: params.bomber,
        destroyer: params.destroyer,
        deathstar: params.deathstar,
        recycler: params.recycler,
        espionage_probe: params.espionage_probe,
        colony_ship: 0,
        solar_satellite: params.solar_satellite,
        rocket_launcher: 0,
        light_laser: 0,
        heavy_laser: 0,
        gauss_cannon: 0,
        ion_cannon: 0,
        plasma_turret: 0,
        small_shield_dome: 0,
        large_shield_dome: 0,
        anti_ballistic_missile: 0,
        interplanetary_missile: 0,
    };

    create_planet_state(
        authority,
        &mut ctx.accounts.player_profile,
        &mut ctx.accounts.planet_state,
        &ctx.accounts.planet_coords.to_account_info(),
        &ctx.accounts.vault_signer.to_account_info(),
        &ctx.accounts.system_program.to_account_info(),
        ctx.bumps.planet_state,
        &planet_params,
    )
    }
}

pub fn initialize_public_homeworld(
    ctx: Context<InitializePublicPlanetVault>,
    params: InitializeHomeworldParams,
) -> Result<()> {
    let now = chain_now()?;
    require_active_vault(
        ctx.accounts.vault_signer.key(),
        &ctx.accounts.authorized_vault,
        ctx.accounts.player_profile.authority,
    )?;

    let authority = ctx.accounts.player_profile.authority;
    let auth_bytes = authority.to_bytes();
    let position = if params.position == 0 {
        (auth_bytes[3] % 15) + 1
    } else {
        params.position.clamp(1, 15)
    };
    let galaxy = if params.galaxy == 0 {
        ((auth_bytes[0] as u16) % 999) + 1
    } else {
        params.galaxy.clamp(1, 999)
    };
    let system = if params.system == 0 {
        (u16::from_le_bytes([auth_bytes[1], auth_bytes[2]]) % 999) + 1
    } else {
        params.system.clamp(1, 999)
    };

    create_public_planet_state(
        authority,
        &mut ctx.accounts.player_profile,
        &mut ctx.accounts.public_planet_state,
        &ctx.accounts.public_planet_coords.to_account_info(),
        &ctx.accounts.vault_signer.to_account_info(),
        &ctx.accounts.system_program.to_account_info(),
        ctx.bumps.public_planet_state,
        &if params.name.is_empty() {
            "Homeworld".to_string()
        } else {
            params.name
        },
        galaxy,
        system,
        position,
        now,
    )
}

pub fn initialize_public_colony(
    ctx: Context<InitializePublicPlanetVault>,
    params: InitializeColonyParams,
) -> Result<()> {
    let now = chain_now()?;
    require_active_vault(
        ctx.accounts.vault_signer.key(),
        &ctx.accounts.authorized_vault,
        ctx.accounts.player_profile.authority,
    )?;

    create_public_planet_state(
        ctx.accounts.player_profile.authority,
        &mut ctx.accounts.player_profile,
        &mut ctx.accounts.public_planet_state,
        &ctx.accounts.public_planet_coords.to_account_info(),
        &ctx.accounts.vault_signer.to_account_info(),
        &ctx.accounts.system_program.to_account_info(),
        ctx.bumps.public_planet_state,
        &if params.name.is_empty() {
            "Colony".to_string()
        } else {
            params.name
        },
        params.galaxy,
        params.system,
        params.position,
        now,
    )
}

pub fn sync_public_planet_view(ctx: Context<SyncPublicPlanetView>) -> Result<()> {
    let planet = &ctx.accounts.planet_state;
    let public_planet = &mut ctx.accounts.public_planet_state;

    public_planet.authority = planet.authority;
    public_planet.player = planet.player;
    public_planet.planet_index = planet.planet_index;
    public_planet.galaxy = planet.galaxy;
    public_planet.system = planet.system;
    public_planet.position = planet.position;
    public_planet.version = 1;
    public_planet.name = planet.name;
    public_planet.created_at = planet.created_at;
    public_planet.bump = ctx.bumps.public_planet_state;

    Ok(())
}

pub fn initialize_quest_state(ctx: Context<InitializeQuestState>) -> Result<()> {
    let now = chain_now()?;
    ctx.accounts.quest_state.set_inner(QuestState {
        authority: ctx.accounts.authority.key(),
        tutorial_claimed_mask: 0,
        daily_epoch: now / 86_400,
        weekly_epoch: now / 604_800,
        monthly_epoch: now / 2_592_000,
        daily_claimed_mask: 0,
        weekly_claimed_mask: 0,
        monthly_claimed_mask: 0,
        daily_checkin_day: -1,
        daily_checkin_streak: 0,
        total_checkins: 0,
        last_updated_ts: now,
        bump: ctx.bumps.quest_state,
    });
    Ok(())
}

pub fn daily_check_in(ctx: Context<QuestAction>) -> Result<()> {
    let now = chain_now()?;
    claim_daily_check_in(
        &mut ctx.accounts.quest_state,
        &mut ctx.accounts.planet_state,
        now,
    )
}

pub fn daily_check_in_vault(ctx: Context<QuestActionVault>) -> Result<()> {
    require_active_vault(
        ctx.accounts.vault_signer.key(),
        &ctx.accounts.authorized_vault,
        ctx.accounts.authority.key(),
    )?;
    let now = chain_now()?;
    claim_daily_check_in(
        &mut ctx.accounts.quest_state,
        &mut ctx.accounts.planet_state,
        now,
    )
}

pub fn claim_quest(ctx: Context<QuestAction>, period: u8, quest_id: u8) -> Result<()> {
    let now = chain_now()?;
    claim_quest_reward(
        &mut ctx.accounts.quest_state,
        &mut ctx.accounts.planet_state,
        period,
        quest_id,
        now,
    )
}

pub fn claim_quest_vault(ctx: Context<QuestActionVault>, period: u8, quest_id: u8) -> Result<()> {
    require_active_vault(
        ctx.accounts.vault_signer.key(),
        &ctx.accounts.authorized_vault,
        ctx.accounts.authority.key(),
    )?;
    let now = chain_now()?;
    claim_quest_reward(
        &mut ctx.accounts.quest_state,
        &mut ctx.accounts.planet_state,
        period,
        quest_id,
        now,
    )
}

pub fn create_alliance(ctx: Context<CreateAlliance>, name: String) -> Result<()> {
    require!(!name.trim().is_empty(), GameStateError::InvalidArgs);
    require!(
        ctx.accounts.store_config.enabled,
        GameStateError::StoreDisabled
    );
    require!(
        ctx.accounts.user_antimatter_account.amount >= ALLIANCE_CREATE_ANTIMATTER_COST,
        GameStateError::InsufficientAntimatter
    );
    burn_antimatter(
        &ctx.accounts.antimatter_mint,
        &ctx.accounts.user_antimatter_account,
        &ctx.accounts.authority,
        &ctx.accounts.token_program,
        ALLIANCE_CREATE_ANTIMATTER_BURN,
    )?;
    transfer_antimatter(
        &ctx.accounts.antimatter_mint,
        &ctx.accounts.user_antimatter_account,
        &ctx.accounts.treasury_antimatter_account,
        &ctx.accounts.authority,
        &ctx.accounts.token_program,
        ALLIANCE_CREATE_ANTIMATTER_TREASURY,
    )?;
    transfer_usdc(
        &ctx.accounts.usdc_mint,
        &ctx.accounts.user_usdc_account,
        &ctx.accounts.treasury_usdc_account,
        &ctx.accounts.authority,
        &ctx.accounts.token_program,
        ALLIANCE_CREATE_USDC_COST,
    )?;
    let now = chain_now()?;
    ctx.accounts.alliance.set_inner(AllianceState {
        founder: ctx.accounts.authority.key(),
        name: copy_name::<MAX_ALLIANCE_NAME_LEN>(&name, "Alliance"),
        level: 1,
        xp: 0,
        member_count: 1,
        max_members: alliance_max_members(1),
        total_missions_completed: 0,
        created_at: now,
        bump: ctx.bumps.alliance,
    });
    ctx.accounts.membership.set_inner(AllianceMembership {
        authority: ctx.accounts.authority.key(),
        alliance: ctx.accounts.alliance.key(),
        role: 2,
        joined_at: now,
        daily_epoch: now / 86_400,
        weekly_epoch: now / 604_800,
        monthly_epoch: now / 2_592_000,
        daily_claimed_mask: 0,
        weekly_claimed_mask: 0,
        monthly_claimed_mask: 0,
        bump: ctx.bumps.membership,
    });
    Ok(())
}

pub fn join_alliance(_ctx: Context<JoinAlliance>) -> Result<()> {
    err!(GameStateError::DirectAllianceJoinDisabled)
}

pub fn request_join_alliance(ctx: Context<RequestJoinAlliance>) -> Result<()> {
    require!(
        ctx.accounts.alliance.member_count < ctx.accounts.alliance.max_members,
        GameStateError::AllianceFull
    );
    let now = chain_now()?;
    ctx.accounts.join_request.set_inner(AllianceJoinRequest {
        applicant: ctx.accounts.authority.key(),
        alliance: ctx.accounts.alliance.key(),
        created_at: now,
        bump: ctx.bumps.join_request,
    });
    Ok(())
}

pub fn approve_join_request(ctx: Context<ApproveJoinRequest>) -> Result<()> {
    let now = chain_now()?;
    require!(
        ctx.accounts.alliance.member_count < ctx.accounts.alliance.max_members,
        GameStateError::AllianceFull
    );
    ctx.accounts.alliance.member_count = ctx.accounts.alliance.member_count.saturating_add(1);
    init_alliance_membership(
        &mut ctx.accounts.membership,
        ctx.accounts.applicant.key(),
        ctx.accounts.alliance.key(),
        1,
        now,
        ctx.bumps.membership,
    );
    Ok(())
}

pub fn reject_join_request(_ctx: Context<RejectJoinRequest>) -> Result<()> {
    Ok(())
}

pub fn expel_alliance_member(ctx: Context<ExpelAllianceMember>) -> Result<()> {
    require!(
        ctx.accounts.leader.key() != ctx.accounts.target.key(),
        GameStateError::CannotExpelAllianceLeader
    );
    ctx.accounts.alliance.member_count = ctx.accounts.alliance.member_count.saturating_sub(1);
    Ok(())
}

pub fn transfer_alliance_leadership(ctx: Context<TransferAllianceLeadership>) -> Result<()> {
    require!(
        ctx.accounts.leader.key() != ctx.accounts.new_leader.key(),
        GameStateError::InvalidAllianceMember
    );
    ctx.accounts.leader_membership.role = 1;
    ctx.accounts.new_leader_membership.role = 2;
    ctx.accounts.alliance.founder = ctx.accounts.new_leader.key();
    Ok(())
}

pub fn leave_alliance(ctx: Context<LeaveAlliance>) -> Result<()> {
    require!(
        ctx.accounts.membership.role != 2,
        GameStateError::AllianceFounderCannotLeave
    );
    ctx.accounts.alliance.member_count = ctx.accounts.alliance.member_count.saturating_sub(1);
    Ok(())
}

fn init_alliance_membership(
    membership: &mut Account<AllianceMembership>,
    authority: Pubkey,
    alliance: Pubkey,
    role: u8,
    now: i64,
    bump: u8,
) {
    membership.set_inner(AllianceMembership {
        authority,
        alliance,
        role,
        joined_at: now,
        daily_epoch: now / 86_400,
        weekly_epoch: now / 604_800,
        monthly_epoch: now / 2_592_000,
        daily_claimed_mask: 0,
        weekly_claimed_mask: 0,
        monthly_claimed_mask: 0,
        bump,
    });
}

pub fn claim_alliance_mission(
    ctx: Context<AllianceMissionAction>,
    period: u8,
    mission_id: u8,
) -> Result<()> {
    require!(mission_id < 64, GameStateError::InvalidAllianceMission);
    let now = chain_now()?;
    sync_alliance_periods(&mut ctx.accounts.membership, now);
    let mission = alliance_mission(period, mission_id)?;
    require!(
        requirement_met(mission.req, &ctx.accounts.planet_state),
        GameStateError::AllianceMissionRequirementsNotMet
    );

    let bit = 1u64 << mission_id;
    let claimed_mask = match period {
        1 => ctx.accounts.membership.daily_claimed_mask,
        2 => ctx.accounts.membership.weekly_claimed_mask,
        3 => ctx.accounts.membership.monthly_claimed_mask,
        _ => return err!(GameStateError::InvalidAllianceMission),
    };
    require!(
        claimed_mask & bit == 0,
        GameStateError::AllianceMissionAlreadyClaimed
    );

    match period {
        1 => ctx.accounts.membership.daily_claimed_mask |= bit,
        2 => ctx.accounts.membership.weekly_claimed_mask |= bit,
        3 => ctx.accounts.membership.monthly_claimed_mask |= bit,
        _ => unreachable!(),
    }
    ctx.accounts.alliance.xp = ctx.accounts.alliance.xp.saturating_add(mission.xp);
    ctx.accounts.alliance.total_missions_completed = ctx
        .accounts
        .alliance
        .total_missions_completed
        .saturating_add(1);
    refresh_alliance_level(&mut ctx.accounts.alliance);
    Ok(())
}

fn sync_alliance_periods(membership: &mut Account<AllianceMembership>, now: i64) {
    let daily_epoch = now / 86_400;
    let weekly_epoch = now / 604_800;
    let monthly_epoch = now / 2_592_000;

    if membership.daily_epoch != daily_epoch {
        membership.daily_epoch = daily_epoch;
        membership.daily_claimed_mask = 0;
    }
    if membership.weekly_epoch != weekly_epoch {
        membership.weekly_epoch = weekly_epoch;
        membership.weekly_claimed_mask = 0;
    }
    if membership.monthly_epoch != monthly_epoch {
        membership.monthly_epoch = monthly_epoch;
        membership.monthly_claimed_mask = 0;
    }
}

fn alliance_max_members(level: u16) -> u16 {
    BASE_ALLIANCE_MAX_MEMBERS.saturating_add(
        level
            .saturating_sub(1)
            .saturating_mul(ALLIANCE_MEMBERS_PER_LEVEL),
    )
}

fn alliance_level_threshold(level: u16) -> u64 {
    let previous = level.saturating_sub(1) as u64;
    previous
        .saturating_mul(level as u64)
        .saturating_mul(ALLIANCE_XP_UNIT)
        / 2
}

fn refresh_alliance_level(alliance: &mut Account<AllianceState>) {
    while alliance.level < u16::MAX {
        let next = alliance.level.saturating_add(1);
        if alliance.xp < alliance_level_threshold(next) {
            break;
        }
        alliance.level = next;
    }
    alliance.max_members = alliance_max_members(alliance.level);
}

fn sync_quest_periods(quest: &mut Account<QuestState>, now: i64) {
    let daily_epoch = now / 86_400;
    let weekly_epoch = now / 604_800;
    let monthly_epoch = now / 2_592_000;

    if quest.daily_epoch != daily_epoch {
        quest.daily_epoch = daily_epoch;
        quest.daily_claimed_mask = 0;
    }
    if quest.weekly_epoch != weekly_epoch {
        quest.weekly_epoch = weekly_epoch;
        quest.weekly_claimed_mask = 0;
    }
    if quest.monthly_epoch != monthly_epoch {
        quest.monthly_epoch = monthly_epoch;
        quest.monthly_claimed_mask = 0;
    }
}

fn claim_daily_check_in(
    quest: &mut Account<QuestState>,
    planet: &mut Account<PlanetState>,
    now: i64,
) -> Result<()> {
    sync_quest_periods(quest, now);
    let day = now / 86_400;
    require!(
        quest.daily_checkin_day != day,
        GameStateError::DailyCheckInAlreadyClaimed
    );

    if quest.daily_checkin_day == day.saturating_sub(1) {
        quest.daily_checkin_streak = quest.daily_checkin_streak.saturating_add(1);
    } else {
        quest.daily_checkin_streak = 1;
    }
    quest.daily_checkin_day = day;
    quest.total_checkins = quest.total_checkins.saturating_add(1);
    quest.daily_claimed_mask |= 1;
    quest.last_updated_ts = now;

    let streak_bonus = (quest.daily_checkin_streak as u64)
        .min(30)
        .saturating_mul(50);
    award_resources(
        planet,
        now,
        500u64.saturating_add(streak_bonus),
        300u64.saturating_add(streak_bonus / 2),
        100u64.saturating_add(streak_bonus / 5),
    )
}

fn claim_quest_reward(
    quest: &mut Account<QuestState>,
    planet: &mut Account<PlanetState>,
    period: u8,
    quest_id: u8,
    now: i64,
) -> Result<()> {
    if period == 1 && quest_id == 0 {
        return claim_daily_check_in(quest, planet, now);
    }

    require!(quest_id < 64, GameStateError::InvalidQuest);
    sync_quest_periods(quest, now);

    let bit = 1u64 << quest_id;
    let claimed_mask = match period {
        0 => quest.tutorial_claimed_mask,
        1 => quest.daily_claimed_mask,
        2 => quest.weekly_claimed_mask,
        3 => quest.monthly_claimed_mask,
        _ => return Err(GameStateError::InvalidQuest.into()),
    };
    require!(claimed_mask & bit == 0, GameStateError::QuestAlreadyClaimed);
    let epoch = match period {
        0 => 0,
        1 => quest.daily_epoch,
        2 => quest.weekly_epoch,
        3 => quest.monthly_epoch,
        _ => return Err(GameStateError::InvalidQuest.into()),
    };
    require!(
        quest_requirement_met(period, quest_id, epoch, planet),
        GameStateError::QuestRequirementsNotMet
    );

    let (metal, crystal, deuterium) = quest_reward(period, quest_id, epoch)?;
    match period {
        0 => quest.tutorial_claimed_mask |= bit,
        1 => quest.daily_claimed_mask |= bit,
        2 => quest.weekly_claimed_mask |= bit,
        3 => quest.monthly_claimed_mask |= bit,
        _ => unreachable!(),
    }
    quest.last_updated_ts = now;

    award_resources(planet, now, metal, crystal, deuterium)
}

fn award_resources(
    planet: &mut Account<PlanetState>,
    now: i64,
    metal: u64,
    crystal: u64,
    deuterium: u64,
) -> Result<()> {
    settle_resources(planet, now)?;
    planet.credit_resources(metal, crystal, deuterium)
}

pub fn initialize_store_config(ctx: Context<InitializeStoreConfig>, enabled: bool) -> Result<()> {
    require_keys_eq!(
        ctx.accounts.admin.key(),
        PROTOCOL_AUTHORITY,
        GameStateError::Unauthorized
    );
    require_keys_eq!(
        ctx.accounts.usdc_mint.key(),
        STORE_USDC_MINT,
        GameStateError::InvalidUsdcMint
    );
    ctx.accounts.store_config.set_inner(StoreConfig {
        admin: ctx.accounts.admin.key(),
        usdc_mint: ctx.accounts.usdc_mint.key(),
        treasury_usdc_account: ctx.accounts.treasury_usdc_account.key(),
        enabled,
        bump: ctx.bumps.store_config,
    });
    Ok(())
}

pub fn update_store_config(ctx: Context<UpdateStoreConfig>, enabled: bool) -> Result<()> {
    require_keys_eq!(
        ctx.accounts.usdc_mint.key(),
        STORE_USDC_MINT,
        GameStateError::InvalidUsdcMint
    );
    ctx.accounts.store_config.usdc_mint = ctx.accounts.usdc_mint.key();
    ctx.accounts.store_config.treasury_usdc_account = ctx.accounts.treasury_usdc_account.key();
    ctx.accounts.store_config.enabled = enabled;
    Ok(())
}

fn sync_store_periods(store: &mut Account<StorePurchaseState>, now: i64) {
    let daily_epoch = now / 86_400;
    let weekly_epoch = now / 604_800;
    let monthly_epoch = now / 2_592_000;

    if store.daily_epoch != daily_epoch {
        store.daily_epoch = daily_epoch;
        store.daily_purchased_mask = 0;
    }
    if store.weekly_epoch != weekly_epoch {
        store.weekly_epoch = weekly_epoch;
        store.weekly_purchased_mask = 0;
    }
    if store.monthly_epoch != monthly_epoch {
        store.monthly_epoch = monthly_epoch;
        store.monthly_purchased_mask = 0;
    }
}

#[derive(Clone, Copy)]
struct StorePack {
    price_usdc: u64,
    metal: u64,
    crystal: u64,
    deuterium: u64,
    shield_seconds: i64,
}

fn store_pack(period: u8, pack_id: u8) -> Result<StorePack> {
    match (period, pack_id) {
        (1, 0) => Ok(StorePack {
            price_usdc: 1_000_000,
            metal: 3_000,
            crystal: 2_000,
            deuterium: 750,
            shield_seconds: 0,
        }),
        (1, 1) => Ok(StorePack {
            price_usdc: 2_500_000,
            metal: 8_000,
            crystal: 5_000,
            deuterium: 2_000,
            shield_seconds: 0,
        }),
        (1, 16) => Ok(StorePack {
            price_usdc: 1_000_000,
            metal: 0,
            crystal: 0,
            deuterium: 0,
            shield_seconds: DAILY_SHIELD_SECONDS,
        }),
        (2, 0) => Ok(StorePack {
            price_usdc: 7_500_000,
            metal: 35_000,
            crystal: 24_000,
            deuterium: 10_000,
            shield_seconds: 0,
        }),
        (2, 1) => Ok(StorePack {
            price_usdc: 15_000_000,
            metal: 80_000,
            crystal: 55_000,
            deuterium: 25_000,
            shield_seconds: 0,
        }),
        (2, 16) => Ok(StorePack {
            price_usdc: 5_000_000,
            metal: 0,
            crystal: 0,
            deuterium: 0,
            shield_seconds: WEEKLY_SHIELD_SECONDS,
        }),
        (3, 0) => Ok(StorePack {
            price_usdc: 30_000_000,
            metal: 180_000,
            crystal: 125_000,
            deuterium: 60_000,
            shield_seconds: 0,
        }),
        (3, 1) => Ok(StorePack {
            price_usdc: 60_000_000,
            metal: 400_000,
            crystal: 275_000,
            deuterium: 140_000,
            shield_seconds: 0,
        }),
        (3, 16) => Ok(StorePack {
            price_usdc: 12_500_000,
            metal: 0,
            crystal: 0,
            deuterium: 0,
            shield_seconds: MONTHLY_SHIELD_SECONDS,
        }),
        _ => Err(GameStateError::InvalidStorePack.into()),
    }
}

fn apply_store_shield(planet: &mut Account<PlanetState>, now: i64, seconds: i64) {
    if seconds <= 0 {
        return;
    }
    let base = planet.protection_until_ts.max(now);
    let max_until = now.saturating_add(MAX_PURCHASED_SHIELD_SECONDS);
    planet.protection_until_ts = base.saturating_add(seconds).min(max_until);
}

pub fn purchase_store_pack(ctx: Context<PurchaseStorePack>, period: u8, pack_id: u8) -> Result<()> {
    require!(
        ctx.accounts.store_config.enabled,
        GameStateError::StoreDisabled
    );
    require!(pack_id < 64, GameStateError::InvalidStorePack);
    require_keys_eq!(
        ctx.accounts.store_config.usdc_mint,
        ctx.accounts.usdc_mint.key(),
        GameStateError::InvalidUsdcMint
    );
    require_keys_eq!(
        ctx.accounts.store_config.treasury_usdc_account,
        ctx.accounts.treasury_usdc_account.key(),
        GameStateError::InvalidUsdcAccount
    );

    let now = chain_now()?;
    let purchase_state = &mut ctx.accounts.purchase_state;
    if purchase_state.authority == Pubkey::default() {
        purchase_state.authority = ctx.accounts.authority.key();
        purchase_state.daily_epoch = now / 86_400;
        purchase_state.weekly_epoch = now / 604_800;
        purchase_state.monthly_epoch = now / 2_592_000;
        purchase_state.daily_purchased_mask = 0;
        purchase_state.weekly_purchased_mask = 0;
        purchase_state.monthly_purchased_mask = 0;
        purchase_state.bump = ctx.bumps.purchase_state;
    }
    require_keys_eq!(
        purchase_state.authority,
        ctx.accounts.authority.key(),
        GameStateError::Unauthorized
    );
    sync_store_periods(purchase_state, now);

    let bit = 1u64 << pack_id;
    let claimed_mask = match period {
        1 => purchase_state.daily_purchased_mask,
        2 => purchase_state.weekly_purchased_mask,
        3 => purchase_state.monthly_purchased_mask,
        _ => return Err(GameStateError::InvalidStorePack.into()),
    };
    require!(
        claimed_mask & bit == 0,
        GameStateError::StorePackAlreadyPurchased
    );

    let pack = store_pack(period, pack_id)?;
    settle_resources(&mut ctx.accounts.planet_state, now)?;
    ctx.accounts
        .planet_state
        .ensure_resource_room(pack.metal, pack.crystal, pack.deuterium)?;
    transfer_usdc(
        &ctx.accounts.usdc_mint,
        &ctx.accounts.user_usdc_account,
        &ctx.accounts.treasury_usdc_account,
        &ctx.accounts.authority,
        &ctx.accounts.token_program,
        pack.price_usdc,
    )?;

    match period {
        1 => purchase_state.daily_purchased_mask |= bit,
        2 => purchase_state.weekly_purchased_mask |= bit,
        3 => purchase_state.monthly_purchased_mask |= bit,
        _ => unreachable!(),
    }
    purchase_state.last_updated_ts = now;

    apply_store_shield(&mut ctx.accounts.planet_state, now, pack.shield_seconds);
    award_resources(
        &mut ctx.accounts.planet_state,
        now,
        pack.metal,
        pack.crystal,
        pack.deuterium,
    )
}

#[derive(Clone, Copy)]
enum QuestRequirement {
    MetalMine(u8),
    CrystalMine(u8),
    DeuteriumSynthesizer(u8),
    SolarPlant(u8),
    FusionReactor(u8),
    RoboticsFactory(u8),
    NaniteFactory(u8),
    Shipyard(u8),
    ResearchLab(u8),
    MetalStorage(u8),
    CrystalStorage(u8),
    DeuteriumTank(u8),
    EnergyTech(u8),
    CombustionDrive(u8),
    ImpulseDrive(u8),
    HyperspaceDrive(u8),
    ComputerTech(u8),
    Astrophysics(u8),
    IgrNetwork(u8),
    WeaponsTechnology(u8),
    ShieldingTechnology(u8),
    ArmorTechnology(u8),
    Ships(u64),
    Defenses(u64),
    SmallCargo(u32),
    LargeCargo(u32),
    LightFighter(u32),
    HeavyFighter(u32),
    Cruiser(u32),
    Battleship(u32),
    Recycler(u32),
    EspionageProbe(u32),
    ColonyShip(u32),
    RocketLauncher(u32),
    LightLaser(u32),
    HeavyLaser(u32),
    GaussCannon(u32),
    IonCannon(u32),
    PlasmaTurret(u32),
}

#[derive(Clone, Copy)]
struct QuestCatalogEntry {
    req: QuestRequirement,
    metal: u64,
    crystal: u64,
    deuterium: u64,
}

#[derive(Clone, Copy)]
struct AllianceMissionEntry {
    req: QuestRequirement,
    xp: u64,
}

const DAILY_ALLIANCE_MISSIONS: [AllianceMissionEntry; 12] = [
    AllianceMissionEntry {
        req: QuestRequirement::MetalMine(3),
        xp: 80,
    },
    AllianceMissionEntry {
        req: QuestRequirement::CrystalMine(3),
        xp: 80,
    },
    AllianceMissionEntry {
        req: QuestRequirement::DeuteriumSynthesizer(2),
        xp: 90,
    },
    AllianceMissionEntry {
        req: QuestRequirement::SolarPlant(4),
        xp: 80,
    },
    AllianceMissionEntry {
        req: QuestRequirement::RoboticsFactory(1),
        xp: 90,
    },
    AllianceMissionEntry {
        req: QuestRequirement::Shipyard(2),
        xp: 110,
    },
    AllianceMissionEntry {
        req: QuestRequirement::ResearchLab(1),
        xp: 100,
    },
    AllianceMissionEntry {
        req: QuestRequirement::Ships(3),
        xp: 120,
    },
    AllianceMissionEntry {
        req: QuestRequirement::Defenses(3),
        xp: 110,
    },
    AllianceMissionEntry {
        req: QuestRequirement::SmallCargo(1),
        xp: 110,
    },
    AllianceMissionEntry {
        req: QuestRequirement::EnergyTech(1),
        xp: 100,
    },
    AllianceMissionEntry {
        req: QuestRequirement::ComputerTech(1),
        xp: 120,
    },
];

const WEEKLY_ALLIANCE_MISSIONS: [AllianceMissionEntry; 12] = [
    AllianceMissionEntry {
        req: QuestRequirement::MetalMine(6),
        xp: 350,
    },
    AllianceMissionEntry {
        req: QuestRequirement::CrystalMine(6),
        xp: 350,
    },
    AllianceMissionEntry {
        req: QuestRequirement::DeuteriumSynthesizer(5),
        xp: 380,
    },
    AllianceMissionEntry {
        req: QuestRequirement::MetalStorage(2),
        xp: 320,
    },
    AllianceMissionEntry {
        req: QuestRequirement::CrystalStorage(2),
        xp: 320,
    },
    AllianceMissionEntry {
        req: QuestRequirement::DeuteriumTank(2),
        xp: 340,
    },
    AllianceMissionEntry {
        req: QuestRequirement::CombustionDrive(2),
        xp: 380,
    },
    AllianceMissionEntry {
        req: QuestRequirement::ImpulseDrive(1),
        xp: 450,
    },
    AllianceMissionEntry {
        req: QuestRequirement::Ships(10),
        xp: 430,
    },
    AllianceMissionEntry {
        req: QuestRequirement::Defenses(10),
        xp: 420,
    },
    AllianceMissionEntry {
        req: QuestRequirement::EspionageProbe(3),
        xp: 360,
    },
    AllianceMissionEntry {
        req: QuestRequirement::Recycler(1),
        xp: 420,
    },
];

const MONTHLY_ALLIANCE_MISSIONS: [AllianceMissionEntry; 12] = [
    AllianceMissionEntry {
        req: QuestRequirement::MetalMine(12),
        xp: 1_200,
    },
    AllianceMissionEntry {
        req: QuestRequirement::CrystalMine(12),
        xp: 1_200,
    },
    AllianceMissionEntry {
        req: QuestRequirement::DeuteriumSynthesizer(10),
        xp: 1_300,
    },
    AllianceMissionEntry {
        req: QuestRequirement::SolarPlant(14),
        xp: 1_100,
    },
    AllianceMissionEntry {
        req: QuestRequirement::ResearchLab(5),
        xp: 1_400,
    },
    AllianceMissionEntry {
        req: QuestRequirement::Shipyard(5),
        xp: 1_350,
    },
    AllianceMissionEntry {
        req: QuestRequirement::ComputerTech(5),
        xp: 1_500,
    },
    AllianceMissionEntry {
        req: QuestRequirement::Astrophysics(1),
        xp: 1_500,
    },
    AllianceMissionEntry {
        req: QuestRequirement::ColonyShip(1),
        xp: 1_600,
    },
    AllianceMissionEntry {
        req: QuestRequirement::Battleship(1),
        xp: 1_700,
    },
    AllianceMissionEntry {
        req: QuestRequirement::Ships(50),
        xp: 1_650,
    },
    AllianceMissionEntry {
        req: QuestRequirement::Defenses(50),
        xp: 1_550,
    },
];

fn alliance_mission(period: u8, mission_id: u8) -> Result<AllianceMissionEntry> {
    let catalog = match period {
        1 => &DAILY_ALLIANCE_MISSIONS[..],
        2 => &WEEKLY_ALLIANCE_MISSIONS[..],
        3 => &MONTHLY_ALLIANCE_MISSIONS[..],
        _ => return err!(GameStateError::InvalidAllianceMission),
    };
    catalog
        .get(mission_id as usize)
        .copied()
        .ok_or_else(|| GameStateError::InvalidAllianceMission.into())
}

const DAILY_ROTATING_QUESTS: [QuestCatalogEntry; 23] = [
    QuestCatalogEntry {
        req: QuestRequirement::MetalMine(3),
        metal: 1_000,
        crystal: 500,
        deuterium: 100,
    },
    QuestCatalogEntry {
        req: QuestRequirement::CrystalMine(3),
        metal: 500,
        crystal: 1_000,
        deuterium: 100,
    },
    QuestCatalogEntry {
        req: QuestRequirement::SolarPlant(4),
        metal: 700,
        crystal: 700,
        deuterium: 150,
    },
    QuestCatalogEntry {
        req: QuestRequirement::DeuteriumSynthesizer(2),
        metal: 600,
        crystal: 500,
        deuterium: 300,
    },
    QuestCatalogEntry {
        req: QuestRequirement::RoboticsFactory(1),
        metal: 800,
        crystal: 650,
        deuterium: 150,
    },
    QuestCatalogEntry {
        req: QuestRequirement::Shipyard(2),
        metal: 1_200,
        crystal: 900,
        deuterium: 250,
    },
    QuestCatalogEntry {
        req: QuestRequirement::ResearchLab(1),
        metal: 900,
        crystal: 900,
        deuterium: 250,
    },
    QuestCatalogEntry {
        req: QuestRequirement::EnergyTech(1),
        metal: 800,
        crystal: 800,
        deuterium: 250,
    },
    QuestCatalogEntry {
        req: QuestRequirement::ComputerTech(1),
        metal: 800,
        crystal: 1_000,
        deuterium: 300,
    },
    QuestCatalogEntry {
        req: QuestRequirement::Ships(3),
        metal: 1_500,
        crystal: 1_000,
        deuterium: 250,
    },
    QuestCatalogEntry {
        req: QuestRequirement::Defenses(3),
        metal: 1_200,
        crystal: 800,
        deuterium: 150,
    },
    QuestCatalogEntry {
        req: QuestRequirement::SmallCargo(2),
        metal: 1_000,
        crystal: 700,
        deuterium: 200,
    },
    QuestCatalogEntry {
        req: QuestRequirement::LightFighter(2),
        metal: 1_100,
        crystal: 700,
        deuterium: 200,
    },
    QuestCatalogEntry {
        req: QuestRequirement::RocketLauncher(3),
        metal: 900,
        crystal: 500,
        deuterium: 100,
    },
    QuestCatalogEntry {
        req: QuestRequirement::LightLaser(1),
        metal: 1_000,
        crystal: 700,
        deuterium: 100,
    },
    QuestCatalogEntry {
        req: QuestRequirement::EspionageProbe(1),
        metal: 700,
        crystal: 700,
        deuterium: 250,
    },
    QuestCatalogEntry {
        req: QuestRequirement::CombustionDrive(1),
        metal: 900,
        crystal: 600,
        deuterium: 300,
    },
    QuestCatalogEntry {
        req: QuestRequirement::MetalStorage(1),
        metal: 700,
        crystal: 500,
        deuterium: 100,
    },
    QuestCatalogEntry {
        req: QuestRequirement::CrystalStorage(1),
        metal: 500,
        crystal: 700,
        deuterium: 100,
    },
    QuestCatalogEntry {
        req: QuestRequirement::DeuteriumTank(1),
        metal: 600,
        crystal: 500,
        deuterium: 250,
    },
    QuestCatalogEntry {
        req: QuestRequirement::HeavyFighter(1),
        metal: 1_400,
        crystal: 1_000,
        deuterium: 350,
    },
    QuestCatalogEntry {
        req: QuestRequirement::ShieldingTechnology(1),
        metal: 900,
        crystal: 1_100,
        deuterium: 250,
    },
    QuestCatalogEntry {
        req: QuestRequirement::ArmorTechnology(1),
        metal: 1_100,
        crystal: 800,
        deuterium: 250,
    },
];

const WEEKLY_ROTATING_QUESTS: [QuestCatalogEntry; 23] = [
    QuestCatalogEntry {
        req: QuestRequirement::ResearchLab(2),
        metal: 6_000,
        crystal: 7_000,
        deuterium: 2_000,
    },
    QuestCatalogEntry {
        req: QuestRequirement::Shipyard(2),
        metal: 8_000,
        crystal: 5_000,
        deuterium: 1_500,
    },
    QuestCatalogEntry {
        req: QuestRequirement::Ships(5),
        metal: 9_000,
        crystal: 6_000,
        deuterium: 2_500,
    },
    QuestCatalogEntry {
        req: QuestRequirement::Defenses(5),
        metal: 8_000,
        crystal: 7_000,
        deuterium: 1_500,
    },
    QuestCatalogEntry {
        req: QuestRequirement::MetalMine(5),
        metal: 10_000,
        crystal: 3_000,
        deuterium: 500,
    },
    QuestCatalogEntry {
        req: QuestRequirement::CrystalMine(5),
        metal: 4_000,
        crystal: 10_000,
        deuterium: 500,
    },
    QuestCatalogEntry {
        req: QuestRequirement::DeuteriumSynthesizer(4),
        metal: 5_000,
        crystal: 4_000,
        deuterium: 3_000,
    },
    QuestCatalogEntry {
        req: QuestRequirement::CombustionDrive(2),
        metal: 5_000,
        crystal: 3_000,
        deuterium: 3_000,
    },
    QuestCatalogEntry {
        req: QuestRequirement::ComputerTech(2),
        metal: 4_000,
        crystal: 5_000,
        deuterium: 3_000,
    },
    QuestCatalogEntry {
        req: QuestRequirement::SmallCargo(5),
        metal: 9_000,
        crystal: 7_000,
        deuterium: 1_500,
    },
    QuestCatalogEntry {
        req: QuestRequirement::RocketLauncher(10),
        metal: 12_000,
        crystal: 3_000,
        deuterium: 500,
    },
    QuestCatalogEntry {
        req: QuestRequirement::LargeCargo(1),
        metal: 8_000,
        crystal: 8_000,
        deuterium: 2_000,
    },
    QuestCatalogEntry {
        req: QuestRequirement::LightFighter(5),
        metal: 8_000,
        crystal: 5_000,
        deuterium: 1_500,
    },
    QuestCatalogEntry {
        req: QuestRequirement::HeavyFighter(2),
        metal: 9_000,
        crystal: 8_000,
        deuterium: 3_000,
    },
    QuestCatalogEntry {
        req: QuestRequirement::Recycler(1),
        metal: 8_000,
        crystal: 7_000,
        deuterium: 2_500,
    },
    QuestCatalogEntry {
        req: QuestRequirement::EspionageProbe(5),
        metal: 5_000,
        crystal: 6_000,
        deuterium: 2_000,
    },
    QuestCatalogEntry {
        req: QuestRequirement::SolarPlant(8),
        metal: 8_000,
        crystal: 5_000,
        deuterium: 1_000,
    },
    QuestCatalogEntry {
        req: QuestRequirement::RoboticsFactory(3),
        metal: 8_000,
        crystal: 6_000,
        deuterium: 1_500,
    },
    QuestCatalogEntry {
        req: QuestRequirement::EnergyTech(2),
        metal: 6_000,
        crystal: 7_000,
        deuterium: 2_500,
    },
    QuestCatalogEntry {
        req: QuestRequirement::WeaponsTechnology(1),
        metal: 8_000,
        crystal: 5_000,
        deuterium: 2_000,
    },
    QuestCatalogEntry {
        req: QuestRequirement::ShieldingTechnology(1),
        metal: 7_000,
        crystal: 8_000,
        deuterium: 2_000,
    },
    QuestCatalogEntry {
        req: QuestRequirement::ArmorTechnology(1),
        metal: 8_000,
        crystal: 7_000,
        deuterium: 2_000,
    },
    QuestCatalogEntry {
        req: QuestRequirement::LightLaser(5),
        metal: 7_000,
        crystal: 5_000,
        deuterium: 1_000,
    },
];

const MONTHLY_ROTATING_QUESTS: [QuestCatalogEntry; 23] = [
    QuestCatalogEntry {
        req: QuestRequirement::ImpulseDrive(1),
        metal: 30_000,
        crystal: 25_000,
        deuterium: 12_000,
    },
    QuestCatalogEntry {
        req: QuestRequirement::Astrophysics(1),
        metal: 25_000,
        crystal: 35_000,
        deuterium: 15_000,
    },
    QuestCatalogEntry {
        req: QuestRequirement::Ships(25),
        metal: 45_000,
        crystal: 30_000,
        deuterium: 15_000,
    },
    QuestCatalogEntry {
        req: QuestRequirement::Defenses(25),
        metal: 40_000,
        crystal: 35_000,
        deuterium: 12_000,
    },
    QuestCatalogEntry {
        req: QuestRequirement::MetalMine(10),
        metal: 50_000,
        crystal: 15_000,
        deuterium: 3_000,
    },
    QuestCatalogEntry {
        req: QuestRequirement::CrystalMine(10),
        metal: 20_000,
        crystal: 50_000,
        deuterium: 3_000,
    },
    QuestCatalogEntry {
        req: QuestRequirement::SolarPlant(12),
        metal: 25_000,
        crystal: 25_000,
        deuterium: 5_000,
    },
    QuestCatalogEntry {
        req: QuestRequirement::ResearchLab(5),
        metal: 30_000,
        crystal: 40_000,
        deuterium: 15_000,
    },
    QuestCatalogEntry {
        req: QuestRequirement::Shipyard(5),
        metal: 45_000,
        crystal: 35_000,
        deuterium: 10_000,
    },
    QuestCatalogEntry {
        req: QuestRequirement::ComputerTech(5),
        metal: 30_000,
        crystal: 35_000,
        deuterium: 20_000,
    },
    QuestCatalogEntry {
        req: QuestRequirement::ColonyShip(1),
        metal: 60_000,
        crystal: 50_000,
        deuterium: 20_000,
    },
    QuestCatalogEntry {
        req: QuestRequirement::Battleship(1),
        metal: 70_000,
        crystal: 50_000,
        deuterium: 25_000,
    },
    QuestCatalogEntry {
        req: QuestRequirement::Cruiser(3),
        metal: 55_000,
        crystal: 45_000,
        deuterium: 18_000,
    },
    QuestCatalogEntry {
        req: QuestRequirement::Recycler(5),
        metal: 45_000,
        crystal: 35_000,
        deuterium: 18_000,
    },
    QuestCatalogEntry {
        req: QuestRequirement::HeavyLaser(10),
        metal: 35_000,
        crystal: 30_000,
        deuterium: 8_000,
    },
    QuestCatalogEntry {
        req: QuestRequirement::GaussCannon(3),
        metal: 45_000,
        crystal: 35_000,
        deuterium: 10_000,
    },
    QuestCatalogEntry {
        req: QuestRequirement::IonCannon(3),
        metal: 35_000,
        crystal: 45_000,
        deuterium: 12_000,
    },
    QuestCatalogEntry {
        req: QuestRequirement::PlasmaTurret(1),
        metal: 60_000,
        crystal: 45_000,
        deuterium: 18_000,
    },
    QuestCatalogEntry {
        req: QuestRequirement::FusionReactor(3),
        metal: 35_000,
        crystal: 35_000,
        deuterium: 16_000,
    },
    QuestCatalogEntry {
        req: QuestRequirement::NaniteFactory(1),
        metal: 80_000,
        crystal: 60_000,
        deuterium: 20_000,
    },
    QuestCatalogEntry {
        req: QuestRequirement::HyperspaceDrive(1),
        metal: 50_000,
        crystal: 55_000,
        deuterium: 25_000,
    },
    QuestCatalogEntry {
        req: QuestRequirement::IgrNetwork(1),
        metal: 45_000,
        crystal: 60_000,
        deuterium: 20_000,
    },
    QuestCatalogEntry {
        req: QuestRequirement::WeaponsTechnology(3),
        metal: 55_000,
        crystal: 45_000,
        deuterium: 20_000,
    },
];

fn rotating_quest(period: u8, quest_id: u8, epoch: i64) -> Result<QuestCatalogEntry> {
    let (slot, catalog): (u64, &[QuestCatalogEntry]) = match period {
        1 => {
            require!(quest_id >= 1, GameStateError::InvalidQuest);
            ((quest_id - 1) as u64, &DAILY_ROTATING_QUESTS)
        }
        2 => (quest_id as u64, &WEEKLY_ROTATING_QUESTS),
        3 => (quest_id as u64, &MONTHLY_ROTATING_QUESTS),
        _ => return Err(GameStateError::InvalidQuest.into()),
    };
    require!(slot < 12, GameStateError::InvalidQuest);
    let len = catalog.len() as u64;
    let seed = (epoch as u64)
        .wrapping_mul(0x9E37_79B9_7F4A_7C15)
        .wrapping_add((period as u64).wrapping_mul(0xBF58_476D_1CE4_E5B9));
    let offset = seed % len;
    let step = ((seed >> 32) % (len - 1)).saturating_add(1);
    Ok(catalog[((offset + slot.wrapping_mul(step)) % len) as usize])
}

fn requirement_met(req: QuestRequirement, planet: &PlanetState) -> bool {
    match req {
        QuestRequirement::MetalMine(v) => planet.metal_mine >= v,
        QuestRequirement::CrystalMine(v) => planet.crystal_mine >= v,
        QuestRequirement::DeuteriumSynthesizer(v) => planet.deuterium_synthesizer >= v,
        QuestRequirement::SolarPlant(v) => planet.solar_plant >= v,
        QuestRequirement::FusionReactor(v) => planet.fusion_reactor >= v,
        QuestRequirement::RoboticsFactory(v) => planet.robotics_factory >= v,
        QuestRequirement::NaniteFactory(v) => planet.nanite_factory >= v,
        QuestRequirement::Shipyard(v) => planet.shipyard >= v,
        QuestRequirement::ResearchLab(v) => planet.research_lab >= v,
        QuestRequirement::MetalStorage(v) => planet.metal_storage >= v,
        QuestRequirement::CrystalStorage(v) => planet.crystal_storage >= v,
        QuestRequirement::DeuteriumTank(v) => planet.deuterium_tank >= v,
        QuestRequirement::EnergyTech(v) => planet.energy_tech >= v,
        QuestRequirement::CombustionDrive(v) => planet.combustion_drive >= v,
        QuestRequirement::ImpulseDrive(v) => planet.impulse_drive >= v,
        QuestRequirement::HyperspaceDrive(v) => planet.hyperspace_drive >= v,
        QuestRequirement::ComputerTech(v) => planet.computer_tech >= v,
        QuestRequirement::Astrophysics(v) => planet.astrophysics >= v,
        QuestRequirement::IgrNetwork(v) => planet.igr_network >= v,
        QuestRequirement::WeaponsTechnology(v) => planet.weapons_technology >= v,
        QuestRequirement::ShieldingTechnology(v) => planet.shielding_technology >= v,
        QuestRequirement::ArmorTechnology(v) => planet.armor_technology >= v,
        QuestRequirement::Ships(v) => total_ships(planet) >= v,
        QuestRequirement::Defenses(v) => total_defenses(planet) >= v,
        QuestRequirement::SmallCargo(v) => planet.small_cargo >= v,
        QuestRequirement::LargeCargo(v) => planet.large_cargo >= v,
        QuestRequirement::LightFighter(v) => planet.light_fighter >= v,
        QuestRequirement::HeavyFighter(v) => planet.heavy_fighter >= v,
        QuestRequirement::Cruiser(v) => planet.cruiser >= v,
        QuestRequirement::Battleship(v) => planet.battleship >= v,
        QuestRequirement::Recycler(v) => planet.recycler >= v,
        QuestRequirement::EspionageProbe(v) => planet.espionage_probe >= v,
        QuestRequirement::ColonyShip(v) => planet.colony_ship >= v,
        QuestRequirement::RocketLauncher(v) => planet.rocket_launcher >= v,
        QuestRequirement::LightLaser(v) => planet.light_laser >= v,
        QuestRequirement::HeavyLaser(v) => planet.heavy_laser >= v,
        QuestRequirement::GaussCannon(v) => planet.gauss_cannon >= v,
        QuestRequirement::IonCannon(v) => planet.ion_cannon >= v,
        QuestRequirement::PlasmaTurret(v) => planet.plasma_turret >= v,
    }
}

fn quest_requirement_met(period: u8, quest_id: u8, epoch: i64, planet: &PlanetState) -> bool {
    if period != 0 {
        return rotating_quest(period, quest_id, epoch)
            .map(|quest| requirement_met(quest.req, planet))
            .unwrap_or(false);
    }
    match (period, quest_id) {
        (0, 0) => true,
        (0, 1) => planet.metal_mine >= 2,
        (0, 2) => planet.crystal_mine >= 2,
        (0, 3) => planet.solar_plant >= 2,
        (0, 4) => planet.deuterium_synthesizer >= 1,
        (0, 5) => planet.metal_storage >= 1,
        (0, 6) => planet.robotics_factory >= 1,
        (0, 7) => planet.shipyard >= 1,
        (0, 8) => planet.research_lab >= 1,
        (0, 9) => planet.energy_tech >= 1,
        (0, 10) => planet.combustion_drive >= 1,
        (0, 11) => total_ships(planet) >= 1,
        (0, 12) => planet.small_cargo >= 1,
        (0, 13) => planet.rocket_launcher >= 1,
        (0, 14) => planet.computer_tech >= 1,
        (0, 15) => planet.espionage_probe >= 1,
        (0, 16) => planet.light_laser >= 1,
        (0, 17) => planet.impulse_drive >= 1,
        (0, 18) => planet.astrophysics >= 1,
        (0, 19) => total_defenses(planet) >= 5,
        (0, 20) => planet.colony_ship >= 1,
        _ => false,
    }
}

fn quest_reward(period: u8, quest_id: u8, epoch: i64) -> Result<(u64, u64, u64)> {
    if period != 0 {
        let quest = rotating_quest(period, quest_id, epoch)?;
        return Ok((quest.metal, quest.crystal, quest.deuterium));
    }
    match (period, quest_id) {
        (0, 0) => Ok((500, 300, 0)),
        (0, 1) => Ok((700, 200, 0)),
        (0, 2) => Ok((400, 600, 0)),
        (0, 3) => Ok((350, 300, 0)),
        (0, 4) => Ok((400, 250, 150)),
        (0, 5) => Ok((300, 300, 0)),
        (0, 6) => Ok((800, 400, 0)),
        (0, 7) => Ok((1_000, 700, 100)),
        (0, 8) => Ok((800, 900, 200)),
        (0, 9) => Ok((600, 600, 200)),
        (0, 10) => Ok((600, 300, 300)),
        (0, 11) => Ok((900, 600, 150)),
        (0, 12) => Ok((1_000, 700, 150)),
        (0, 13) => Ok((800, 200, 0)),
        (0, 14) => Ok((700, 700, 250)),
        (0, 15) => Ok((600, 500, 250)),
        (0, 16) => Ok((800, 500, 0)),
        (0, 17) => Ok((1_500, 1_500, 600)),
        (0, 18) => Ok((1_500, 1_800, 700)),
        (0, 19) => Ok((1_200, 900, 200)),
        (0, 20) => Ok((2_000, 2_000, 800)),
        _ => Err(GameStateError::InvalidQuest.into()),
    }
}

fn total_ships(planet: &PlanetState) -> u64 {
    planet.small_cargo as u64
        + planet.large_cargo as u64
        + planet.light_fighter as u64
        + planet.heavy_fighter as u64
        + planet.cruiser as u64
        + planet.battleship as u64
        + planet.battlecruiser as u64
        + planet.bomber as u64
        + planet.destroyer as u64
        + planet.deathstar as u64
        + planet.recycler as u64
        + planet.espionage_probe as u64
        + planet.colony_ship as u64
        + planet.solar_satellite as u64
}

fn total_defenses(planet: &PlanetState) -> u64 {
    planet.rocket_launcher as u64
        + planet.light_laser as u64
        + planet.heavy_laser as u64
        + planet.gauss_cannon as u64
        + planet.ion_cannon as u64
        + planet.plasma_turret as u64
        + planet.small_shield_dome as u64
        + planet.large_shield_dome as u64
        + planet.anti_ballistic_missile as u64
        + planet.interplanetary_missile as u64
}

pub fn produce(ctx: Context<MutatePlanetState>, _now: i64) -> Result<()> {
    let now = chain_now()?;
    produce_planet(&mut ctx.accounts.planet_state, now)
}

pub fn produce_vault(ctx: Context<MutatePlanetStateVault>, _now: i64) -> Result<()> {
    require_active_vault(
        ctx.accounts.vault_signer.key(),
        &ctx.accounts.authorized_vault,
        ctx.accounts.planet_state.authority,
    )?;
    let now = chain_now()?;
    produce_planet(&mut ctx.accounts.planet_state, now)
}

pub fn start_build(ctx: Context<MutatePlanetState>, building_idx: u8, _now: i64) -> Result<()> {
    let now = chain_now()?;
    start_build_planet(&mut ctx.accounts.planet_state, building_idx, now)
}

pub fn start_build_vault(
    ctx: Context<MutatePlanetStateVault>,
    building_idx: u8,
    _now: i64,
) -> Result<()> {
    require_active_vault(
        ctx.accounts.vault_signer.key(),
        &ctx.accounts.authorized_vault,
        ctx.accounts.planet_state.authority,
    )?;
    let now = chain_now()?;
    start_build_planet(&mut ctx.accounts.planet_state, building_idx, now)
}

pub fn finish_build(ctx: Context<MutatePlanetState>, _now: i64) -> Result<()> {
    let now = chain_now()?;
    finish_build_planet(&mut ctx.accounts.planet_state, now)
}

pub fn finish_build_vault(ctx: Context<MutatePlanetStateVault>, _now: i64) -> Result<()> {
    require_active_vault(
        ctx.accounts.vault_signer.key(),
        &ctx.accounts.authorized_vault,
        ctx.accounts.planet_state.authority,
    )?;
    let now = chain_now()?;
    finish_build_planet(&mut ctx.accounts.planet_state, now)
}

pub fn start_research(ctx: Context<MutatePlanetState>, tech_idx: u8, _now: i64) -> Result<()> {
    let now = chain_now()?;
    start_research_planet(&mut ctx.accounts.planet_state, tech_idx, now)
}

pub fn start_research_vault(
    ctx: Context<MutatePlanetStateVault>,
    tech_idx: u8,
    _now: i64,
) -> Result<()> {
    require_active_vault(
        ctx.accounts.vault_signer.key(),
        &ctx.accounts.authorized_vault,
        ctx.accounts.planet_state.authority,
    )?;
    let now = chain_now()?;
    start_research_planet(&mut ctx.accounts.planet_state, tech_idx, now)
}

pub fn finish_research(ctx: Context<MutatePlanetState>, _now: i64) -> Result<()> {
    let now = chain_now()?;
    finish_research_planet(&mut ctx.accounts.planet_state, now)
}

pub fn finish_research_vault(ctx: Context<MutatePlanetStateVault>, _now: i64) -> Result<()> {
    require_active_vault(
        ctx.accounts.vault_signer.key(),
        &ctx.accounts.authorized_vault,
        ctx.accounts.planet_state.authority,
    )?;
    let now = chain_now()?;
    finish_research_planet(&mut ctx.accounts.planet_state, now)
}

pub fn build_ship(
    ctx: Context<MutatePlanetState>,
    ship_type: u8,
    quantity: u32,
    _now: i64,
) -> Result<()> {
    let now = chain_now()?;
    build_ship_planet(&mut ctx.accounts.planet_state, ship_type, quantity, now)
}

pub fn build_ship_vault(
    ctx: Context<MutatePlanetStateVault>,
    ship_type: u8,
    quantity: u32,
    _now: i64,
) -> Result<()> {
    require_active_vault(
        ctx.accounts.vault_signer.key(),
        &ctx.accounts.authorized_vault,
        ctx.accounts.planet_state.authority,
    )?;
    let now = chain_now()?;
    build_ship_planet(&mut ctx.accounts.planet_state, ship_type, quantity, now)
}

pub fn finish_ship_build(ctx: Context<MutatePlanetState>, _now: i64) -> Result<()> {
    let now = chain_now()?;
    finish_ship_build_planet(&mut ctx.accounts.planet_state, now)
}

pub fn finish_ship_build_vault(ctx: Context<MutatePlanetStateVault>, _now: i64) -> Result<()> {
    require_active_vault(
        ctx.accounts.vault_signer.key(),
        &ctx.accounts.authorized_vault,
        ctx.accounts.planet_state.authority,
    )?;
    let now = chain_now()?;
    finish_ship_build_planet(&mut ctx.accounts.planet_state, now)
}

pub fn build_defense(
    ctx: Context<MutatePlanetState>,
    defense_type: u8,
    quantity: u32,
    _now: i64,
) -> Result<()> {
    let now = chain_now()?;
    build_defense_planet(&mut ctx.accounts.planet_state, defense_type, quantity, now)
}

pub fn build_defense_vault(
    ctx: Context<MutatePlanetStateVault>,
    defense_type: u8,
    quantity: u32,
    _now: i64,
) -> Result<()> {
    require_active_vault(
        ctx.accounts.vault_signer.key(),
        &ctx.accounts.authorized_vault,
        ctx.accounts.planet_state.authority,
    )?;
    let now = chain_now()?;
    build_defense_planet(&mut ctx.accounts.planet_state, defense_type, quantity, now)
}

pub fn finish_defense_build(ctx: Context<MutatePlanetState>, _now: i64) -> Result<()> {
    let now = chain_now()?;
    finish_defense_build_planet(&mut ctx.accounts.planet_state, now)
}

pub fn finish_defense_build_vault(ctx: Context<MutatePlanetStateVault>, _now: i64) -> Result<()> {
    require_active_vault(
        ctx.accounts.vault_signer.key(),
        &ctx.accounts.authorized_vault,
        ctx.accounts.planet_state.authority,
    )?;
    let now = chain_now()?;
    finish_defense_build_planet(&mut ctx.accounts.planet_state, now)
}

pub fn launch_fleet(ctx: Context<MutatePlanetState>, params: LaunchFleetParams) -> Result<()> {
    launch_fleet_planet(&mut ctx.accounts.planet_state, params)
}

pub fn launch_fleet_vault(
    ctx: Context<MutatePlanetStateVault>,
    params: LaunchFleetParams,
) -> Result<()> {
    require_active_vault(
        ctx.accounts.vault_signer.key(),
        &ctx.accounts.authorized_vault,
        ctx.accounts.planet_state.authority,
    )?;
    launch_fleet_planet(&mut ctx.accounts.planet_state, params)
}

pub fn lock_resources_for_market(
    ctx: Context<MutatePlanetState>,
    resource_type: u8,
    amount: u64,
) -> Result<()> {
    require!(amount > 0, GameStateError::InvalidArgs);

    let resource_type = match resource_type {
        0 => ResourceType::Metal,
        1 => ResourceType::Crystal,
        2 => ResourceType::Deuterium,
        _ => return err!(GameStateError::InvalidArgs),
    };

    let planet = &mut ctx.accounts.planet_state;
    let now = Clock::get()?.unix_timestamp;
    settle_resources(planet, now)?;
    require!(
        now >= planet.market_unlocked_at,
        GameStateError::GameplayLocked
    );

    match resource_type {
        ResourceType::Metal => {
            require!(
                planet.metal >= amount,
                GameStateError::InsufficientResources
            );
            planet.metal = planet.metal.saturating_sub(amount);
        }
        ResourceType::Crystal => {
            require!(
                planet.crystal >= amount,
                GameStateError::InsufficientResources
            );
            planet.crystal = planet.crystal.saturating_sub(amount);
        }
        ResourceType::Deuterium => {
            require!(
                planet.deuterium >= amount,
                GameStateError::InsufficientResources
            );
            planet.deuterium = planet.deuterium.saturating_sub(amount);
        }
    }

    Ok(())
}

pub fn release_resources_from_market(
    ctx: Context<ReleaseResourcesFromMarket>,
    resource_type: u8,
    amount: u64,
) -> Result<()> {
    require!(amount > 0, GameStateError::InvalidArgs);
    require_market_authority(&ctx.accounts.market_authority)?;

    let resource_type = match resource_type {
        0 => ResourceType::Metal,
        1 => ResourceType::Crystal,
        2 => ResourceType::Deuterium,
        _ => return err!(GameStateError::InvalidArgs),
    };

    let seller = &mut ctx.accounts.seller_planet;
    let now = Clock::get()?.unix_timestamp;
    settle_resources(seller, now)?;

    match resource_type {
        ResourceType::Metal => seller.credit_resources(amount, 0, 0)?,
        ResourceType::Crystal => seller.credit_resources(0, amount, 0)?,
        ResourceType::Deuterium => seller.credit_resources(0, 0, amount)?,
    }

    Ok(())
}

pub fn transfer_resources_from_market(
    ctx: Context<TransferResourcesFromMarket>,
    resource_type: u8,
    amount: u64,
) -> Result<()> {
    require!(amount > 0, GameStateError::InvalidArgs);
    require_market_authority(&ctx.accounts.market_authority)?;

    let resource_type = match resource_type {
        0 => ResourceType::Metal,
        1 => ResourceType::Crystal,
        2 => ResourceType::Deuterium,
        _ => return err!(GameStateError::InvalidArgs),
    };

    let buyer = &mut ctx.accounts.buyer_planet;
    let now = Clock::get()?.unix_timestamp;
    settle_resources(buyer, now)?;
    require_keys_eq!(
        buyer.authority,
        ctx.accounts.buyer.key(),
        GameStateError::Unauthorized
    );
    require!(
        now >= buyer.market_unlocked_at,
        GameStateError::GameplayLocked
    );

    match resource_type {
        ResourceType::Metal => buyer.credit_resources(amount, 0, 0)?,
        ResourceType::Crystal => buyer.credit_resources(0, amount, 0)?,
        ResourceType::Deuterium => buyer.credit_resources(0, 0, amount)?,
    }

    msg!(
        "Market transfer: {} {} credited to buyer",
        amount,
        match resource_type {
            ResourceType::Metal => "metal",
            ResourceType::Crystal => "crystal",
            ResourceType::Deuterium => "deuterium",
        }
    );

    Ok(())
}

pub fn resolve_transport(ctx: Context<ResolveTransport>, slot: u8, _now: i64) -> Result<()> {
    msg!("resolve_transport: entered");
    msg!("resolve_transport: slot={}", slot);
    let now = chain_now()?;
    resolve_transport_planets(
        &mut ctx.accounts.source_planet,
        &mut ctx.accounts.destination_planet,
        slot as usize,
        now,
    )
}

pub fn resolve_transport_vault(
    ctx: Context<ResolveTransportVault>,
    slot: u8,
    _now: i64,
) -> Result<()> {
    msg!("resolve_transport_vault: entered");
    msg!("resolve_transport_vault: slot={}", slot);
    require_active_vault(
        ctx.accounts.vault_signer.key(),
        &ctx.accounts.authorized_vault,
        ctx.accounts.source_planet.authority,
    )?;
    msg!("resolve_transport_vault: vault ok");
    let now = chain_now()?;
    resolve_transport_planets(
        &mut ctx.accounts.source_planet,
        &mut ctx.accounts.destination_planet,
        slot as usize,
        now,
    )
}

pub fn resolve_transport_empty(ctx: Context<MutatePlanetState>, slot: u8, _now: i64) -> Result<()> {
    let now = chain_now()?;
    resolve_transport_empty_slot(&mut ctx.accounts.planet_state, slot as usize, now)
}

pub fn resolve_transport_empty_vault(
    ctx: Context<MutatePlanetStateVault>,
    slot: u8,
    _now: i64,
) -> Result<()> {
    require_active_vault(
        ctx.accounts.vault_signer.key(),
        &ctx.accounts.authorized_vault,
        ctx.accounts.planet_state.authority,
    )?;
    let now = chain_now()?;
    resolve_transport_empty_slot(&mut ctx.accounts.planet_state, slot as usize, now)
}

pub fn resolve_attack(ctx: Context<ResolveAttack>, slot: u8, _now: i64) -> Result<()> {
    let source_key = ctx.accounts.source_planet.key();
    let destination_key = ctx.accounts.destination_planet.key();
    let now = chain_now()?;
    resolve_attack_planets(
        &mut ctx.accounts.source_planet,
        &mut ctx.accounts.destination_planet,
        &mut ctx.accounts.destination_coords,
        source_key,
        destination_key,
        slot as usize,
        now,
    )
}

pub fn resolve_attack_vault(ctx: Context<ResolveAttackVault>, slot: u8, _now: i64) -> Result<()> {
    require_active_vault(
        ctx.accounts.vault_signer.key(),
        &ctx.accounts.authorized_vault,
        ctx.accounts.source_planet.authority,
    )?;
    let source_key = ctx.accounts.source_planet.key();
    let destination_key = ctx.accounts.destination_planet.key();
    let now = chain_now()?;
    resolve_attack_planets(
        &mut ctx.accounts.source_planet,
        &mut ctx.accounts.destination_planet,
        &mut ctx.accounts.destination_coords,
        source_key,
        destination_key,
        slot as usize,
        now,
    )
}

pub fn resolve_espionage(ctx: Context<ResolveAttack>, slot: u8, _now: i64) -> Result<()> {
    let source_key = ctx.accounts.source_planet.key();
    let destination_key = ctx.accounts.destination_planet.key();
    let now = chain_now()?;
    resolve_espionage_planets(
        &mut ctx.accounts.source_planet,
        &mut ctx.accounts.destination_planet,
        source_key,
        destination_key,
        slot as usize,
        now,
    )
}

pub fn resolve_espionage_vault(
    ctx: Context<ResolveAttackVault>,
    slot: u8,
    _now: i64,
) -> Result<()> {
    require_active_vault(
        ctx.accounts.vault_signer.key(),
        &ctx.accounts.authorized_vault,
        ctx.accounts.source_planet.authority,
    )?;
    let source_key = ctx.accounts.source_planet.key();
    let destination_key = ctx.accounts.destination_planet.key();
    let now = chain_now()?;
    resolve_espionage_planets(
        &mut ctx.accounts.source_planet,
        &mut ctx.accounts.destination_planet,
        source_key,
        destination_key,
        slot as usize,
        now,
    )
}

/// Wallet-signed: resolve a colonize mission.
/// The colony planet + coord lock must already exist (created by `initialize_colony`).
pub fn resolve_colonize(ctx: Context<ResolveColonize>, slot: u8, _now: i64) -> Result<()> {
    // Verify the coords PDA matches the mission target
    let mission = ctx.accounts.source_planet.mission(slot as usize);
    require_keys_eq!(
        ctx.accounts.colony_coords.planet,
        ctx.accounts.colony_planet.key(),
        GameStateError::InvalidDestination
    );
    require!(
        ctx.accounts.colony_coords.galaxy == mission.target_galaxy
            && ctx.accounts.colony_coords.system == mission.target_system
            && ctx.accounts.colony_coords.position == mission.target_position,
        GameStateError::InvalidDestination
    );
    let now = chain_now()?;
    resolve_colonize_planet(&mut ctx.accounts.source_planet, slot as usize, now)
}

/// Vault-signed: resolve a colonize mission.
pub fn resolve_colonize_vault(
    ctx: Context<ResolveColonizeVault>,
    slot: u8,
    _now: i64,
) -> Result<()> {
    msg!("resolve_colonize_vault: entered");
    msg!("resolve_colonize_vault: slot={}", slot);

    require_active_vault(
        ctx.accounts.vault_signer.key(),
        &ctx.accounts.authorized_vault,
        ctx.accounts.source_planet.authority,
    )?;
    msg!("resolve_colonize_vault: vault ok");

    let mission = ctx.accounts.source_planet.mission(slot as usize);
    msg!("resolve_colonize_vault: loaded mission");

    require_keys_eq!(
        ctx.accounts.colony_coords.planet,
        ctx.accounts.colony_planet.key(),
        GameStateError::InvalidDestination
    );
    msg!("resolve_colonize_vault: planet key matches");

    require!(
        ctx.accounts.colony_coords.galaxy == mission.target_galaxy
            && ctx.accounts.colony_coords.system == mission.target_system
            && ctx.accounts.colony_coords.position == mission.target_position,
        GameStateError::InvalidDestination
    );
    msg!("resolve_colonize_vault: coords match mission");

    let now = chain_now()?;
    resolve_colonize_planet(&mut ctx.accounts.source_planet, slot as usize, now)
}

/// Wallet-signed: transfer ownership of a single planet to a new authority.
///
/// Both the old and new authorities must have initialized their player profile.
/// After transfer, vault-signed gameplay by the new wallet works immediately
/// because `MutatePlanetStateVault` looks up `authorized_vault` via
/// `planet_state.authority`, which now points to the new wallet.
///
/// The planet PDA address does not change — it stays seeded by the old wallet.
/// The old wallet's wallet-signed fallback path for this planet stops working
/// (by design — only the new authority owns it).
pub fn transfer_planet(ctx: Context<TransferPlanet>) -> Result<()> {
    let planet = &mut ctx.accounts.planet_state;
    let coords = &mut ctx.accounts.planet_coords;
    let new_authority = ctx.accounts.new_authority.key();

    // Update ownership fields
    planet.authority = new_authority;
    coords.authority = new_authority;

    Ok(())
}

/// One-time admin setup for the global ANTIMATTER mint used to accelerate queues.
pub fn initialize_game_config(
    ctx: Context<InitializeGameConfig>,
    antimatter_mint: Pubkey,
) -> Result<()> {
    require_keys_eq!(
        ctx.accounts.admin.key(),
        PROTOCOL_AUTHORITY,
        GameStateError::Unauthorized
    );
    require_keys_eq!(
        antimatter_mint,
        PROTOCOL_ANTIMATTER_MINT,
        GameStateError::InvalidAntimatterMint
    );
    ctx.accounts.game_config.set_inner(GameConfig {
        admin: ctx.accounts.admin.key(),
        antimatter_mint,
        bump: ctx.bumps.game_config,
    });
    Ok(())
}

/// Admin-only: rotate the ANTIMATTER mint reference if needed.
pub fn update_antimatter_mint(
    ctx: Context<UpdateGameConfig>,
    antimatter_mint: Pubkey,
) -> Result<()> {
    require_keys_eq!(
        antimatter_mint,
        PROTOCOL_ANTIMATTER_MINT,
        GameStateError::InvalidAntimatterMint
    );
    ctx.accounts.game_config.antimatter_mint = antimatter_mint;
    Ok(())
}

/// Wallet-signed: burn 1 ANTIMATTER per second remaining to finish a building queue instantly.
pub fn accelerate_build_with_antimatter(ctx: Context<UseAntimatter>) -> Result<()> {
    require_keys_eq!(
        ctx.accounts.game_config.antimatter_mint,
        ctx.accounts.antimatter_mint.key(),
        GameStateError::InvalidAntimatterMint
    );
    accelerate_build_with_antimatter_inner(
        &mut ctx.accounts.planet_state,
        &ctx.accounts.antimatter_mint,
        &ctx.accounts.user_antimatter_account,
        &ctx.accounts.authority,
        &ctx.accounts.token_program,
    )?;
    Ok(())
}

/// Wallet-signed: burn 1 ANTIMATTER per second remaining to finish research instantly.
pub fn accelerate_research_with_antimatter(ctx: Context<UseAntimatter>) -> Result<()> {
    require_keys_eq!(
        ctx.accounts.game_config.antimatter_mint,
        ctx.accounts.antimatter_mint.key(),
        GameStateError::InvalidAntimatterMint
    );
    accelerate_research_with_antimatter_inner(
        &mut ctx.accounts.planet_state,
        &ctx.accounts.antimatter_mint,
        &ctx.accounts.user_antimatter_account,
        &ctx.accounts.authority,
        &ctx.accounts.token_program,
    )?;
    Ok(())
}

/// Wallet-signed: burn 1 ANTIMATTER per second remaining to finish ship production instantly.
pub fn accelerate_ship_build_with_antimatter(ctx: Context<UseAntimatter>) -> Result<()> {
    require_keys_eq!(
        ctx.accounts.game_config.antimatter_mint,
        ctx.accounts.antimatter_mint.key(),
        GameStateError::InvalidAntimatterMint
    );
    accelerate_ship_build_with_antimatter_inner(
        &mut ctx.accounts.planet_state,
        &ctx.accounts.antimatter_mint,
        &ctx.accounts.user_antimatter_account,
        &ctx.accounts.authority,
        &ctx.accounts.token_program,
    )?;
    Ok(())
}

/// Wallet-signed: burn ANTIMATTER to bring an active mission leg to the current timestamp.
/// `leg = 0` accelerates outbound travel to arrival; `leg = 1` accelerates return travel.
pub fn accelerate_mission_with_antimatter(
    ctx: Context<UseAntimatter>,
    slot: u8,
    leg: u8,
) -> Result<()> {
    require_keys_eq!(
        ctx.accounts.game_config.antimatter_mint,
        ctx.accounts.antimatter_mint.key(),
        GameStateError::InvalidAntimatterMint
    );
    accelerate_mission_with_antimatter_inner(
        &mut ctx.accounts.planet_state,
        &ctx.accounts.antimatter_mint,
        &ctx.accounts.user_antimatter_account,
        &ctx.accounts.authority,
        &ctx.accounts.token_program,
        slot,
        leg,
    )?;
    Ok(())
}
