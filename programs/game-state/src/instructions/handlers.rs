use anchor_lang::prelude::*;
use anchor_lang::solana_program::program_error::ProgramError;
use anchor_lang::solana_program::program_option::COption;
use anchor_spl::associated_token::get_associated_token_address;
use anchor_spl::token::{self, Mint, MintTo, TokenAccount, Transfer};

use crate::constants::*;
use crate::contexts::*;
use crate::error::GameStateError;
use crate::state::*;
use crate::utils::*;

const SECONDS_PER_DAY: i64 = 86_400;
const UNIX_TO_CIVIL_DAYS: i64 = 719_468;
const MARKET_LISTING_INDEX_PLANET_OFFSET: usize = 8;
const MARKET_LISTING_INDEX_ACTIVE_OFFSET: usize = 104;
const MARKET_OBLIGATION_PLANET_OFFSET: usize = 8;
const MARKET_OBLIGATION_ACTIVE_OFFERS_OFFSET: usize = 40;

fn daily_epoch(now: i64) -> i64 {
    now.div_euclid(SECONDS_PER_DAY)
}

fn weekly_epoch(now: i64) -> i64 {
    // Unix day 0 was Thursday; +3 makes weeks roll on Monday 00:00 UTC.
    (daily_epoch(now) + 3).div_euclid(7)
}

fn civil_year_month_from_days(days_since_unix_epoch: i64) -> (i64, i64) {
    let z = days_since_unix_epoch + UNIX_TO_CIVIL_DAYS;
    let era = if z >= 0 { z } else { z - 146_096 }.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096).div_euclid(365);
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2).div_euclid(153);
    let month = mp + if mp < 10 { 3 } else { -9 };
    let year = y + if month <= 2 { 1 } else { 0 };
    (year, month)
}

fn monthly_epoch(now: i64) -> i64 {
    let (year, month) = civil_year_month_from_days(daily_epoch(now));
    year * 12 + (month - 1)
}

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

/// Wallet-only: add vault authorization for a profile created by marketplace purchase.
pub fn initialize_vault_for_existing_player(
    ctx: Context<InitializeVaultForExistingPlayer>,
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

    let now = Clock::get()?.unix_timestamp;
    require!(
        expires_at == 0 || expires_at > now,
        GameStateError::InvalidArgs
    );

    let authority = ctx.accounts.authority.key();
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
pub fn initialize_homeworld<'info>(
    ctx: Context<'_, '_, '_, 'info, InitializePlanetVault<'info>>,
    params: InitializeHomeworldParams,
) -> Result<()> {
    let now = chain_now()?;
    let authority = ctx.accounts.player_profile.authority;
    require!(
        ctx.accounts.player_profile.planet_count == 0,
        GameStateError::InvalidArgs
    );
    let authorized_vault_info = ctx.accounts.authorized_vault.to_account_info();
    let authorized_vault: AuthorizedVault =
        read_program_account(&authorized_vault_info, ctx.program_id)?;
    let (expected_authorized_vault, _) =
        Pubkey::find_program_address(&[b"authorized_vault", authority.as_ref()], ctx.program_id);
    require_keys_eq!(
        ctx.accounts.authorized_vault.key(),
        expected_authorized_vault,
        GameStateError::InvalidVaultAuthorization
    );
    require_active_vault(
        ctx.accounts.vault_signer.key(),
        &authorized_vault,
        authority,
    )?;

    ensure_quest_accounts_for_authority_raw(
        &ctx.accounts.quest_state.to_account_info(),
        &ctx.accounts.quest_progress.to_account_info(),
        &ctx.accounts.quest_reward_targets.to_account_info(),
        &ctx.accounts.tutorial_quest_reward_targets.to_account_info(),
        &ctx.accounts.vault_signer.to_account_info(),
        &ctx.accounts.system_program.to_account_info(),
        authority,
        now,
        ctx.program_id,
    )?;
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
    )?;

    if let Some(owner_index_info) = ctx.remaining_accounts.first() {
        create_or_update_planet_owner_index(
            owner_index_info,
            &ctx.accounts.vault_signer.to_account_info(),
            &ctx.accounts.system_program.to_account_info(),
            authority,
            ctx.accounts.planet_state.planet_index,
            ctx.accounts.planet_state.key(),
            ctx.program_id,
        )?;
        if let Some(registry_info) = ctx.remaining_accounts.get(1) {
            create_or_update_planet_ownership_registry(
                registry_info,
                &ctx.accounts.vault_signer.to_account_info(),
                &ctx.accounts.system_program.to_account_info(),
                authority,
                ctx.accounts.planet_state.key(),
                owner_index_info.key(),
                ctx.program_id,
            )?;
        }
    }

    Ok(())
}

/// Vault-signed: initialize colony.
/// Creates both `planet_state` and `planet_coords` atomically.
/// If `planet_coords` already exists the tx fails — client shows "slot occupied".
pub fn initialize_colony<'info>(
    ctx: Context<'_, '_, '_, 'info, InitializeColonyVault<'info>>,
    params: InitializeColonyParams,
    slot: u8,
) -> Result<()> {
    let now = chain_now()?;
    require!(
        ctx.accounts.player_profile.planet_count > 0,
        GameStateError::InvalidArgs
    );
    require!(
        (slot as usize) < MAX_MISSIONS,
        GameStateError::InvalidMissionSlot
    );
    let authority = ctx.accounts.player_profile.authority;
    let mission = ctx.accounts.source_planet.mission(slot as usize);
    require!(
        mission.mission_type == MISSION_COLONIZE,
        GameStateError::InvalidMission
    );
    require!(!mission.applied, GameStateError::AlreadyResolved);
    require!(now >= mission.arrive_ts, GameStateError::MissionInFlight);
    require!(mission.colony_ship == 1, GameStateError::MissingColonyShip);
    require_colony_cargo_within_initial_caps(
        mission.cargo_metal,
        mission.cargo_crystal,
        mission.cargo_deuterium,
    )?;
    require!(
        mission.target_galaxy == params.galaxy
            && mission.target_system == params.system
            && mission.target_position == params.position,
        GameStateError::InvalidDestination
    );
    let authorized_vault_info = ctx.accounts.authorized_vault.to_account_info();
    let authorized_vault: AuthorizedVault =
        read_program_account(&authorized_vault_info, ctx.program_id)?;
    let (expected_authorized_vault, _) =
        Pubkey::find_program_address(&[b"authorized_vault", authority.as_ref()], ctx.program_id);
    require_keys_eq!(
        ctx.accounts.authorized_vault.key(),
        expected_authorized_vault,
        GameStateError::InvalidVaultAuthorization
    );
    require_active_vault(
        ctx.accounts.vault_signer.key(),
        &authorized_vault,
        authority,
    )?;

    ensure_quest_accounts_for_authority_raw(
        &ctx.accounts.quest_state.to_account_info(),
        &ctx.accounts.quest_progress.to_account_info(),
        &ctx.accounts.quest_reward_targets.to_account_info(),
        &ctx.accounts.tutorial_quest_reward_targets.to_account_info(),
        &ctx.accounts.vault_signer.to_account_info(),
        &ctx.accounts.system_program.to_account_info(),
        authority,
        now,
        ctx.program_id,
    )?;

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
        metal: mission.cargo_metal,
        crystal: mission.cargo_crystal,
        deuterium: mission.cargo_deuterium,
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
        small_cargo: mission.small_cargo,
        large_cargo: mission.large_cargo,
        light_fighter: mission.light_fighter,
        heavy_fighter: mission.heavy_fighter,
        cruiser: mission.cruiser,
        battleship: mission.battleship,
        battlecruiser: mission.battlecruiser,
        bomber: mission.bomber,
        destroyer: mission.destroyer,
        deathstar: mission.deathstar,
        recycler: mission.recycler,
        espionage_probe: mission.espionage_probe,
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
    )?;

    if let Some(owner_index_info) = ctx.remaining_accounts.get(1) {
        create_or_update_planet_owner_index(
            owner_index_info,
            &ctx.accounts.vault_signer.to_account_info(),
            &ctx.accounts.system_program.to_account_info(),
            authority,
            ctx.accounts.planet_state.planet_index,
            ctx.accounts.planet_state.key(),
            ctx.program_id,
        )?;
        if let Some(registry_info) = ctx.remaining_accounts.get(2) {
            create_or_update_planet_ownership_registry(
                registry_info,
                &ctx.accounts.vault_signer.to_account_info(),
                &ctx.accounts.system_program.to_account_info(),
                authority,
                ctx.accounts.planet_state.key(),
                owner_index_info.key(),
                ctx.program_id,
            )?;
        }
    }

    ctx.accounts.source_planet.clear_mission(slot as usize);
    ctx.accounts.source_planet.active_missions =
        ctx.accounts.source_planet.active_missions.saturating_sub(1);

    increment_quest_progress(
        Some(&ctx.accounts.quest_progress.to_account_info()),
        ctx.remaining_accounts.first(),
        authority,
        ctx.program_id,
        now,
        QuestProgressMetric::PlanetsColonized,
        1,
        Some(ctx.accounts.source_planet.key()),
    )
}

fn require_allocated_owner_index_slot(slot: u32, planet_count: u32) -> Result<()> {
    require!(slot < planet_count, GameStateError::InvalidArgs);
    Ok(())
}

pub fn sync_planet_owner_index_vault(
    ctx: Context<SyncPlanetOwnerIndexVault>,
    slot: u32,
) -> Result<()> {
    require_active_vault(
        ctx.accounts.vault_signer.key(),
        &ctx.accounts.authorized_vault,
        ctx.accounts.authority.key(),
    )?;
    require_keys_eq!(
        ctx.accounts.planet_state.authority,
        ctx.accounts.authority.key(),
        GameStateError::Unauthorized
    );
    require_allocated_owner_index_slot(slot, ctx.accounts.player_profile.planet_count)?;

    let owner_index = &mut ctx.accounts.owner_index;
    if owner_index.authority != Pubkey::default() {
        require_keys_eq!(
            owner_index.authority,
            ctx.accounts.authority.key(),
            GameStateError::Unauthorized
        );
        require!(
            !owner_index.active || owner_index.planet == ctx.accounts.planet_state.key(),
            GameStateError::InvalidArgs
        );
    }

    owner_index.authority = ctx.accounts.authority.key();
    owner_index.slot = slot;
    owner_index.planet = ctx.accounts.planet_state.key();
    owner_index.active = true;
    owner_index.bump = ctx.bumps.owner_index;

    let registry = &mut ctx.accounts.ownership_registry;
    if registry.planet != Pubkey::default() {
        require_keys_eq!(
            registry.planet,
            ctx.accounts.planet_state.key(),
            GameStateError::InvalidArgs
        );
        require_keys_eq!(
            registry.authority,
            ctx.accounts.authority.key(),
            GameStateError::Unauthorized
        );
    }
    registry.planet = ctx.accounts.planet_state.key();
    registry.authority = ctx.accounts.authority.key();
    registry.owner_index = owner_index.key();
    registry.bump = ctx.bumps.ownership_registry;
    Ok(())
}

pub fn initialize_public_homeworld(
    _ctx: Context<InitializePublicPlanetVault>,
    _params: InitializeHomeworldParams,
) -> Result<()> {
    err!(GameStateError::InvalidArgs)
}

pub fn initialize_public_colony(
    _ctx: Context<InitializePublicPlanetVault>,
    _params: InitializeColonyParams,
) -> Result<()> {
    err!(GameStateError::InvalidArgs)
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
        daily_epoch: daily_epoch(now),
        weekly_epoch: weekly_epoch(now),
        monthly_epoch: monthly_epoch(now),
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

fn ensure_quest_accounts_for_authority_raw<'info>(
    quest_state_info: &AccountInfo<'info>,
    quest_progress_info: &AccountInfo<'info>,
    quest_reward_targets_info: &AccountInfo<'info>,
    tutorial_quest_reward_targets_info: &AccountInfo<'info>,
    payer_info: &AccountInfo<'info>,
    system_program_info: &AccountInfo<'info>,
    authority: Pubkey,
    now: i64,
    program_id: &Pubkey,
) -> Result<()> {
    let (expected_quest_state, quest_state_bump) =
        Pubkey::find_program_address(&[b"quest_state", authority.as_ref()], program_id);
    require_keys_eq!(
        quest_state_info.key(),
        expected_quest_state,
        GameStateError::Unauthorized
    );
    if quest_state_info.owner == &anchor_lang::system_program::ID {
        create_program_pda_account(
            quest_state_info,
            payer_info,
            system_program_info,
            QUEST_STATE_SPACE,
            program_id,
            &[&[b"quest_state", authority.as_ref(), &[quest_state_bump]]],
        )?;
        let quest_state = QuestState {
            authority,
            tutorial_claimed_mask: 0,
            daily_epoch: daily_epoch(now),
            weekly_epoch: weekly_epoch(now),
            monthly_epoch: monthly_epoch(now),
            daily_claimed_mask: 0,
            weekly_claimed_mask: 0,
            monthly_claimed_mask: 0,
            daily_checkin_day: -1,
            daily_checkin_streak: 0,
            total_checkins: 0,
            last_updated_ts: now,
            bump: quest_state_bump,
        };
        write_program_account(quest_state_info, &quest_state)?;
    } else {
        let quest_state: QuestState = read_program_account(quest_state_info, program_id)?;
        require_keys_eq!(
            quest_state.authority,
            authority,
            GameStateError::Unauthorized
        );
    }

    let (expected_quest_progress, quest_progress_bump) =
        Pubkey::find_program_address(&[b"quest_progress", authority.as_ref()], program_id);
    require_keys_eq!(
        quest_progress_info.key(),
        expected_quest_progress,
        GameStateError::Unauthorized
    );
    if quest_progress_info.owner == &anchor_lang::system_program::ID {
        create_program_pda_account(
            quest_progress_info,
            payer_info,
            system_program_info,
            QUEST_PROGRESS_STATE_SPACE,
            program_id,
            &[&[
                b"quest_progress",
                authority.as_ref(),
                &[quest_progress_bump],
            ]],
        )?;
        let quest_progress = QuestProgressState {
            authority,
            daily_epoch: daily_epoch(now),
            weekly_epoch: weekly_epoch(now),
            monthly_epoch: monthly_epoch(now),
            daily_store_packs_bought: 0,
            weekly_store_packs_bought: 0,
            monthly_store_packs_bought: 0,
            daily_antimatter_spent: 0,
            weekly_antimatter_spent: 0,
            monthly_antimatter_spent: 0,
            daily_planets_colonized: 0,
            weekly_planets_colonized: 0,
            monthly_planets_colonized: 0,
            daily_attacks_resolved: 0,
            weekly_attacks_resolved: 0,
            monthly_attacks_resolved: 0,
            daily_transports_resolved: 0,
            weekly_transports_resolved: 0,
            monthly_transports_resolved: 0,
            daily_spy_missions_resolved: 0,
            weekly_spy_missions_resolved: 0,
            monthly_spy_missions_resolved: 0,
            last_updated_ts: now,
            bump: quest_progress_bump,
        };
        write_program_account(quest_progress_info, &quest_progress)?;
    } else {
        let quest_progress: QuestProgressState =
            read_program_account(quest_progress_info, program_id)?;
        require_keys_eq!(
            quest_progress.authority,
            authority,
            GameStateError::Unauthorized
        );
    }

    let (expected_quest_reward_targets, quest_reward_targets_bump) =
        Pubkey::find_program_address(&[b"quest_reward_targets", authority.as_ref()], program_id);
    require_keys_eq!(
        quest_reward_targets_info.key(),
        expected_quest_reward_targets,
        GameStateError::Unauthorized
    );
    if quest_reward_targets_info.owner == &anchor_lang::system_program::ID {
        create_program_pda_account(
            quest_reward_targets_info,
            payer_info,
            system_program_info,
            QUEST_REWARD_TARGET_STATE_SPACE,
            program_id,
            &[&[
                b"quest_reward_targets",
                authority.as_ref(),
                &[quest_reward_targets_bump],
            ]],
        )?;
        let mut quest_reward_targets = empty_quest_reward_targets(authority, now);
        quest_reward_targets.bump = quest_reward_targets_bump;
        write_program_account(quest_reward_targets_info, &quest_reward_targets)?;
    } else {
        let quest_reward_targets: QuestRewardTargetState =
            read_program_account(quest_reward_targets_info, program_id)?;
        require_keys_eq!(
            quest_reward_targets.authority,
            authority,
            GameStateError::Unauthorized
        );
    }

    let (expected_tutorial_quest_reward_targets, tutorial_quest_reward_targets_bump) =
        Pubkey::find_program_address(
            &[b"quest_tutorial_reward_targets", authority.as_ref()],
            program_id,
        );
    require_keys_eq!(
        tutorial_quest_reward_targets_info.key(),
        expected_tutorial_quest_reward_targets,
        GameStateError::Unauthorized
    );
    if tutorial_quest_reward_targets_info.owner == &anchor_lang::system_program::ID {
        create_program_pda_account(
            tutorial_quest_reward_targets_info,
            payer_info,
            system_program_info,
            TUTORIAL_QUEST_REWARD_TARGET_STATE_SPACE,
            program_id,
            &[&[
                b"quest_tutorial_reward_targets",
                authority.as_ref(),
                &[tutorial_quest_reward_targets_bump],
            ]],
        )?;
        let mut tutorial_quest_reward_targets =
            empty_tutorial_quest_reward_targets(authority, now);
        tutorial_quest_reward_targets.bump = tutorial_quest_reward_targets_bump;
        write_program_account(
            tutorial_quest_reward_targets_info,
            &tutorial_quest_reward_targets,
        )?;
    } else {
        let tutorial_quest_reward_targets: TutorialQuestRewardTargetState =
            read_program_account(tutorial_quest_reward_targets_info, program_id)?;
        require_keys_eq!(
            tutorial_quest_reward_targets.authority,
            authority,
            GameStateError::Unauthorized
        );
    }

    Ok(())
}

pub fn initialize_quest_progress(ctx: Context<InitializeQuestProgress>) -> Result<()> {
    let now = chain_now()?;
    ctx.accounts.quest_progress.set_inner(QuestProgressState {
        authority: ctx.accounts.authority.key(),
        daily_epoch: daily_epoch(now),
        weekly_epoch: weekly_epoch(now),
        monthly_epoch: monthly_epoch(now),
        daily_store_packs_bought: 0,
        weekly_store_packs_bought: 0,
        monthly_store_packs_bought: 0,
        daily_antimatter_spent: 0,
        weekly_antimatter_spent: 0,
        monthly_antimatter_spent: 0,
        daily_planets_colonized: 0,
        weekly_planets_colonized: 0,
        monthly_planets_colonized: 0,
        daily_attacks_resolved: 0,
        weekly_attacks_resolved: 0,
        monthly_attacks_resolved: 0,
        daily_transports_resolved: 0,
        weekly_transports_resolved: 0,
        monthly_transports_resolved: 0,
        daily_spy_missions_resolved: 0,
        weekly_spy_missions_resolved: 0,
        monthly_spy_missions_resolved: 0,
        last_updated_ts: now,
        bump: ctx.bumps.quest_progress,
    });
    Ok(())
}

pub fn initialize_quest_reward_targets(ctx: Context<InitializeQuestRewardTargets>) -> Result<()> {
    let now = chain_now()?;
    let mut targets = empty_quest_reward_targets(ctx.accounts.authority.key(), now);
    targets.bump = ctx.bumps.quest_reward_targets;
    ctx.accounts.quest_reward_targets.set_inner(targets);
    let mut tutorial_targets =
        empty_tutorial_quest_reward_targets(ctx.accounts.authority.key(), now);
    tutorial_targets.bump = ctx.bumps.tutorial_quest_reward_targets;
    ctx.accounts
        .tutorial_quest_reward_targets
        .set_inner(tutorial_targets);
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
    let planet_info = ctx.accounts.planet_state.to_account_info();
    let mut planet = read_planet_quest_fields(&planet_info, ctx.program_id)?;
    require_keys_eq!(
        planet.deposit.authority,
        ctx.accounts.authority.key(),
        GameStateError::Unauthorized
    );
    claim_daily_check_in_live(&mut ctx.accounts.quest_state, &mut planet.deposit, now)?;
    write_planet_deposit_fields(&planet_info, &planet.deposit)
}

pub fn claim_quest(ctx: Context<QuestAction>, period: u8, quest_id: u8) -> Result<()> {
    let now = chain_now()?;
    let quest_progress_info = ctx.accounts.quest_progress.to_account_info();
    let mut quest_progress = if period == 0 {
        empty_quest_progress(ctx.accounts.authority.key(), now)
    } else {
        validate_quest_progress_pda(
            &quest_progress_info,
            ctx.accounts.authority.key(),
            ctx.program_id,
        )?
    };
    let quest_reward_targets_info = ctx.remaining_accounts.first();
    let tutorial_quest_reward_targets_info = ctx.remaining_accounts.get(1);
    claim_quest_reward(
        &mut ctx.accounts.quest_state,
        &mut ctx.accounts.planet_state,
        &mut quest_progress,
        quest_reward_targets_info,
        tutorial_quest_reward_targets_info,
        ctx.program_id,
        period,
        quest_id,
        now,
    )?;
    if period != 0 {
        write_program_account(&quest_progress_info, &quest_progress)?;
    }
    Ok(())
}

pub fn claim_quest_vault(ctx: Context<QuestActionVault>, period: u8, quest_id: u8) -> Result<()> {
    msg!("claim_quest_vault: entered p={} q={}", period, quest_id);
    require_active_vault(
        ctx.accounts.vault_signer.key(),
        &ctx.accounts.authorized_vault,
        ctx.accounts.authority.key(),
    )?;
    msg!("claim_quest_vault: vault ok");
    let now = chain_now()?;
    let planet_info = ctx.accounts.planet_state.to_account_info();
    msg!("claim_quest_vault: reading planet");
    let mut planet = read_planet_quest_fields(&planet_info, ctx.program_id)?;
    msg!("claim_quest_vault: planet read");
    require_keys_eq!(
        planet.deposit.authority,
        ctx.accounts.authority.key(),
        GameStateError::Unauthorized
    );
    msg!("claim_quest_vault: authority ok");
    let quest_progress_info = ctx.accounts.quest_progress.to_account_info();
    let mut quest_progress = if period == 0 {
        empty_quest_progress(ctx.accounts.authority.key(), now)
    } else {
        validate_quest_progress_pda(
            &quest_progress_info,
            ctx.accounts.authority.key(),
            ctx.program_id,
        )?
    };
    let quest_reward_targets_info = ctx.remaining_accounts.first();
    let tutorial_quest_reward_targets_info = ctx.remaining_accounts.get(1);
    claim_quest_reward_live(
        &mut ctx.accounts.quest_state,
        &mut planet,
        &mut quest_progress,
        quest_reward_targets_info,
        tutorial_quest_reward_targets_info,
        planet_info.key(),
        ctx.accounts.authority.key(),
        ctx.program_id,
        period,
        quest_id,
        now,
    )?;
    msg!("claim_quest_vault: reward ok");
    if period != 0 {
        write_program_account(&quest_progress_info, &quest_progress)?;
    }
    write_planet_deposit_fields(&planet_info, &planet.deposit)
}

pub fn create_alliance(
    ctx: Context<CreateAlliance>,
    name: String,
    tag: String,
    image_url: String,
) -> Result<()> {
    require!(!name.trim().is_empty(), GameStateError::InvalidArgs);
    let trimmed_tag = tag.trim();
    let trimmed_image_url = image_url.trim();
    require!(
        !trimmed_tag.is_empty() && trimmed_tag.len() <= MAX_ALLIANCE_TAG_LEN,
        GameStateError::InvalidArgs
    );
    require!(
        trimmed_image_url.len() <= MAX_ALLIANCE_IMAGE_URL_LEN,
        GameStateError::InvalidArgs
    );
    if !trimmed_image_url.is_empty() {
        require!(
            trimmed_image_url.starts_with("https://") || trimmed_image_url.starts_with("http://"),
            GameStateError::InvalidArgs
        );
    }
    require!(
        ctx.accounts.store_config.enabled,
        GameStateError::StoreDisabled
    );
    require_keys_eq!(
        ctx.accounts.store_config.treasury_usdc_account,
        ctx.accounts.treasury_usdc_account.key(),
        GameStateError::InvalidUsdcAccount
    );
    require_keys_eq!(
        ctx.accounts.treasury_antimatter_account.owner,
        ctx.accounts.store_config.admin,
        GameStateError::InvalidAntimatterAccount
    );
    require_protocol_antimatter_treasury(
        ctx.accounts.treasury_antimatter_account.key(),
        ctx.accounts.store_config.admin,
        ctx.accounts.antimatter_mint.key(),
    )?;
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
    ctx.accounts.metadata.set_inner(AllianceMetadata {
        alliance: ctx.accounts.alliance.key(),
        tag: copy_name::<MAX_ALLIANCE_TAG_LEN>(trimmed_tag, ""),
        image_url: copy_name::<MAX_ALLIANCE_IMAGE_URL_LEN>(trimmed_image_url, ""),
        bump: ctx.bumps.metadata,
    });
    ctx.accounts.membership.set_inner(AllianceMembership {
        authority: ctx.accounts.authority.key(),
        alliance: ctx.accounts.alliance.key(),
        role: 2,
        joined_at: now,
        daily_epoch: daily_epoch(now),
        weekly_epoch: weekly_epoch(now),
        monthly_epoch: monthly_epoch(now),
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

pub fn request_join_alliance_vault(ctx: Context<RequestJoinAllianceVault>) -> Result<()> {
    require_active_vault(
        ctx.accounts.vault_signer.key(),
        &ctx.accounts.authorized_vault,
        ctx.accounts.authority.key(),
    )?;
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
    activate_alliance_membership(
        &mut ctx.accounts.membership,
        ctx.accounts.applicant.key(),
        ctx.accounts.alliance.key(),
        1,
        now,
        ctx.bumps.membership,
    )?;
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
    deactivate_alliance_membership(
        &mut ctx.accounts.alliance,
        &mut ctx.accounts.target_membership,
    )
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
    deactivate_alliance_membership(&mut ctx.accounts.alliance, &mut ctx.accounts.membership)
}

fn deactivate_alliance_membership(
    alliance: &mut AllianceState,
    membership: &mut AllianceMembership,
) -> Result<()> {
    require!(membership.role > 0, GameStateError::InvalidAllianceMember);
    require!(alliance.member_count > 0, GameStateError::InvalidAllianceMember);
    alliance.member_count -= 1;
    membership.role = 0;
    Ok(())
}

fn activate_alliance_membership(
    membership: &mut Account<AllianceMembership>,
    authority: Pubkey,
    alliance: Pubkey,
    role: u8,
    now: i64,
    bump: u8,
) -> Result<()> {
    activate_alliance_membership_state(membership, authority, alliance, role, now, bump)
}

fn activate_alliance_membership_state(
    membership: &mut AllianceMembership,
    authority: Pubkey,
    alliance: Pubkey,
    role: u8,
    now: i64,
    bump: u8,
) -> Result<()> {
    if membership.authority != Pubkey::default() {
        require_keys_eq!(membership.authority, authority, GameStateError::Unauthorized);
        require!(membership.role == 0, GameStateError::InvalidAllianceMember);
        if membership.alliance == alliance {
            sync_alliance_periods(membership, now);
            membership.role = role;
            membership.joined_at = now;
            return Ok(());
        }
    }

    *membership = AllianceMembership {
        authority,
        alliance,
        role,
        joined_at: now,
        daily_epoch: daily_epoch(now),
        weekly_epoch: weekly_epoch(now),
        monthly_epoch: monthly_epoch(now),
        daily_claimed_mask: 0,
        weekly_claimed_mask: 0,
        monthly_claimed_mask: 0,
        bump,
    };
    Ok(())
}

fn require_protocol_antimatter_treasury(
    treasury: Pubkey,
    admin: Pubkey,
    mint: Pubkey,
) -> Result<()> {
    let expected_treasury = get_associated_token_address(&admin, &mint);
    require_keys_eq!(
        treasury,
        expected_treasury,
        GameStateError::InvalidAntimatterAccount
    );
    Ok(())
}

pub fn claim_alliance_mission(
    _ctx: Context<AllianceMissionAction>,
    _period: u8,
    _mission_id: u8,
) -> Result<()> {
    err!(GameStateError::InvalidAllianceMission)
}

pub fn initialize_alliance_treasury(ctx: Context<InitializeAllianceTreasury>) -> Result<()> {
    ctx.accounts
        .alliance_treasury
        .set_inner(AllianceTreasuryState {
            alliance: ctx.accounts.alliance.key(),
            metal: 0,
            crystal: 0,
            deuterium: 0,
            antimatter: 0,
            logistics_hub: 0,
            research_grid: 0,
            defense_coordination: 0,
            trade_network: 0,
            total_metal_deposited: 0,
            total_crystal_deposited: 0,
            total_deuterium_deposited: 0,
            total_antimatter_deposited: 0,
            bump: ctx.bumps.alliance_treasury,
        });
    Ok(())
}

pub fn initialize_alliance_treasury_vault(
    ctx: Context<InitializeAllianceTreasuryVault>,
) -> Result<()> {
    require_active_vault(
        ctx.accounts.vault_signer.key(),
        &ctx.accounts.authorized_vault,
        ctx.accounts.authority.key(),
    )?;
    ctx.accounts
        .alliance_treasury
        .set_inner(AllianceTreasuryState {
            alliance: ctx.accounts.alliance.key(),
            metal: 0,
            crystal: 0,
            deuterium: 0,
            antimatter: 0,
            logistics_hub: 0,
            research_grid: 0,
            defense_coordination: 0,
            trade_network: 0,
            total_metal_deposited: 0,
            total_crystal_deposited: 0,
            total_deuterium_deposited: 0,
            total_antimatter_deposited: 0,
            bump: ctx.bumps.alliance_treasury,
        });
    Ok(())
}

fn credit_alliance_treasury(
    treasury: &mut AllianceTreasuryState,
    metal: u64,
    crystal: u64,
    deuterium: u64,
    antimatter: u64,
) -> Result<()> {
    let next_metal = treasury.metal.checked_add(metal).ok_or(GameStateError::InvalidArgs)?;
    let next_crystal = treasury.crystal.checked_add(crystal).ok_or(GameStateError::InvalidArgs)?;
    let next_deuterium = treasury
        .deuterium
        .checked_add(deuterium)
        .ok_or(GameStateError::InvalidArgs)?;
    let next_antimatter = treasury
        .antimatter
        .checked_add(antimatter)
        .ok_or(GameStateError::InvalidArgs)?;
    let next_total_metal = treasury
        .total_metal_deposited
        .checked_add(metal)
        .ok_or(GameStateError::InvalidArgs)?;
    let next_total_crystal = treasury
        .total_crystal_deposited
        .checked_add(crystal)
        .ok_or(GameStateError::InvalidArgs)?;
    let next_total_deuterium = treasury
        .total_deuterium_deposited
        .checked_add(deuterium)
        .ok_or(GameStateError::InvalidArgs)?;
    let next_total_antimatter = treasury
        .total_antimatter_deposited
        .checked_add(antimatter)
        .ok_or(GameStateError::InvalidArgs)?;

    treasury.metal = next_metal;
    treasury.crystal = next_crystal;
    treasury.deuterium = next_deuterium;
    treasury.antimatter = next_antimatter;
    treasury.total_metal_deposited = next_total_metal;
    treasury.total_crystal_deposited = next_total_crystal;
    treasury.total_deuterium_deposited = next_total_deuterium;
    treasury.total_antimatter_deposited = next_total_antimatter;
    Ok(())
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
    let alliance_info = ctx.accounts.alliance.to_account_info();
    let membership_info = ctx.accounts.membership.to_account_info();
    let alliance_treasury_info = ctx.accounts.alliance_treasury.to_account_info();
    let planet_state_info = ctx.accounts.planet_state.to_account_info();
    let game_config_info = ctx.accounts.game_config.to_account_info();
    let store_config_info = ctx.accounts.store_config.to_account_info();

    let mut alliance: AllianceState = read_program_account(&alliance_info, ctx.program_id)?;
    let mut membership: AllianceMembership =
        read_program_account(&membership_info, ctx.program_id)?;
    let mut alliance_treasury: AllianceTreasuryState =
        read_program_account(&alliance_treasury_info, ctx.program_id)?;
    let mut planet_deposit = read_planet_deposit_fields(&planet_state_info, ctx.program_id)?;
    let game_config: GameConfig = read_program_account(&game_config_info, ctx.program_id)?;
    let store_config: StoreConfig = read_program_account(&store_config_info, ctx.program_id)?;

    let (expected_membership, _) = Pubkey::find_program_address(
        &[
            b"alliance_membership",
            ctx.accounts.authority.key().as_ref(),
        ],
        ctx.program_id,
    );
    require_keys_eq!(
        ctx.accounts.membership.key(),
        expected_membership,
        GameStateError::InvalidAllianceMember
    );
    let (expected_treasury, _) = Pubkey::find_program_address(
        &[b"alliance_treasury", alliance_info.key.as_ref()],
        ctx.program_id,
    );
    require_keys_eq!(
        ctx.accounts.alliance_treasury.key(),
        expected_treasury,
        GameStateError::InvalidAllianceMember
    );
    let (expected_game_config, _) = Pubkey::find_program_address(&[b"game_config"], ctx.program_id);
    require_keys_eq!(
        ctx.accounts.game_config.key(),
        expected_game_config,
        GameStateError::Unauthorized
    );
    let (expected_store_config, _) =
        Pubkey::find_program_address(&[b"store_config"], ctx.program_id);
    require_keys_eq!(
        ctx.accounts.store_config.key(),
        expected_store_config,
        GameStateError::Unauthorized
    );
    require_keys_eq!(
        membership.authority,
        ctx.accounts.authority.key(),
        GameStateError::Unauthorized
    );
    require_keys_eq!(
        membership.alliance,
        ctx.accounts.alliance.key(),
        GameStateError::InvalidAllianceMember
    );
    require!(membership.role > 0, GameStateError::InvalidAllianceMember);
    require_keys_eq!(
        alliance_treasury.alliance,
        ctx.accounts.alliance.key(),
        GameStateError::InvalidAllianceMember
    );
    require_keys_eq!(
        planet_deposit.authority,
        ctx.accounts.authority.key(),
        GameStateError::Unauthorized
    );
    require_keys_eq!(
        game_config.antimatter_mint,
        ctx.accounts.antimatter_mint.key(),
        GameStateError::InvalidAntimatterMint
    );
    require!(
        metal > 0 || crystal > 0 || deuterium > 0 || antimatter > 0,
        GameStateError::InvalidArgs
    );
    let now = chain_now()?;
    sync_alliance_periods(&mut membership, now);
    let mission = alliance_deposit_mission(period, mission_id)?;
    require!(
        metal >= mission.metal
            && crystal >= mission.crystal
            && deuterium >= mission.deuterium
            && antimatter >= mission.antimatter,
        GameStateError::AllianceMissionRequirementsNotMet
    );

    let bit = 1u64 << mission_id;
    let claimed_mask = match period {
        1 => membership.daily_claimed_mask,
        2 => membership.weekly_claimed_mask,
        3 => membership.monthly_claimed_mask,
        _ => return err!(GameStateError::InvalidAllianceMission),
    };
    require!(
        claimed_mask & bit == 0,
        GameStateError::AllianceMissionAlreadyClaimed
    );

    settle_planet_deposit_fields(&mut planet_deposit, now)?;
    require!(
        planet_deposit.metal >= metal,
        GameStateError::InsufficientMetal
    );
    require!(
        planet_deposit.crystal >= crystal,
        GameStateError::InsufficientCrystal
    );
    require!(
        planet_deposit.deuterium >= deuterium,
        GameStateError::InsufficientDeuterium
    );

    if antimatter > 0 {
        let antimatter_mint_info = ctx.accounts.antimatter_mint.to_account_info();
        let user_antimatter_info = ctx.accounts.user_antimatter_account.to_account_info();
        let treasury_antimatter_info = ctx.accounts.treasury_antimatter_account.to_account_info();
        let antimatter_mint: Mint = read_token_account(&antimatter_mint_info)?;
        let user_antimatter_account: TokenAccount = read_token_account(&user_antimatter_info)?;
        let treasury_antimatter_account: TokenAccount =
            read_token_account(&treasury_antimatter_info)?;
        require!(
            antimatter_mint.decimals == ANTIMATTER_DECIMALS,
            GameStateError::InvalidAntimatterMint
        );
        require_keys_eq!(
            user_antimatter_account.mint,
            ctx.accounts.antimatter_mint.key(),
            GameStateError::InvalidAntimatterMint
        );
        require_keys_eq!(
            user_antimatter_account.owner,
            ctx.accounts.authority.key(),
            GameStateError::InvalidAntimatterAccount
        );
        require_keys_eq!(
            treasury_antimatter_account.mint,
            ctx.accounts.antimatter_mint.key(),
            GameStateError::InvalidAntimatterMint
        );
        require_keys_eq!(
            treasury_antimatter_account.owner,
            store_config.admin,
            GameStateError::InvalidAntimatterAccount
        );
        require_protocol_antimatter_treasury(
            ctx.accounts.treasury_antimatter_account.key(),
            store_config.admin,
            ctx.accounts.antimatter_mint.key(),
        )?;
        require!(
            user_antimatter_account.amount >= antimatter,
            GameStateError::InsufficientAntimatter
        );
        token::transfer(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: user_antimatter_info,
                    to: treasury_antimatter_info,
                    authority: ctx.accounts.authority.to_account_info(),
                },
            ),
            antimatter,
        )?;
    }

    planet_deposit.metal = planet_deposit.metal.saturating_sub(metal);
    planet_deposit.crystal = planet_deposit.crystal.saturating_sub(crystal);
    planet_deposit.deuterium = planet_deposit.deuterium.saturating_sub(deuterium);

    credit_alliance_treasury(&mut alliance_treasury, metal, crystal, deuterium, antimatter)?;

    match period {
        1 => membership.daily_claimed_mask |= bit,
        2 => membership.weekly_claimed_mask |= bit,
        3 => membership.monthly_claimed_mask |= bit,
        _ => unreachable!(),
    }

    let base_xp = alliance_deposit_xp(&mission, metal, crystal, deuterium, antimatter);
    let xp = apply_bps_bonus(base_xp, alliance_logistics_xp_bonus_bps(&alliance_treasury));
    alliance.xp = alliance.xp.checked_add(xp).ok_or(GameStateError::InvalidArgs)?;
    alliance.total_missions_completed = alliance
        .total_missions_completed
        .checked_add(1)
        .ok_or(GameStateError::InvalidArgs)?;
    refresh_alliance_level(&mut alliance);
    write_program_account(&alliance_info, &alliance)?;
    write_program_account(&membership_info, &membership)?;
    write_program_account(&alliance_treasury_info, &alliance_treasury)?;
    write_planet_deposit_fields(&planet_state_info, &planet_deposit)?;
    Ok(())
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
    let authorized_vault_info = ctx.accounts.authorized_vault.to_account_info();
    let alliance_info = ctx.accounts.alliance.to_account_info();
    let membership_info = ctx.accounts.membership.to_account_info();
    let alliance_treasury_info = ctx.accounts.alliance_treasury.to_account_info();
    let planet_state_info = ctx.accounts.planet_state.to_account_info();
    let game_config_info = ctx.accounts.game_config.to_account_info();
    let store_config_info = ctx.accounts.store_config.to_account_info();

    let authorized_vault: AuthorizedVault =
        read_program_account(&authorized_vault_info, ctx.program_id)?;
    let mut alliance: AllianceState = read_program_account(&alliance_info, ctx.program_id)?;
    let mut membership: AllianceMembership =
        read_program_account(&membership_info, ctx.program_id)?;
    let mut alliance_treasury: AllianceTreasuryState =
        read_program_account(&alliance_treasury_info, ctx.program_id)?;
    let mut planet_deposit = read_planet_deposit_fields(&planet_state_info, ctx.program_id)?;
    let game_config: GameConfig = read_program_account(&game_config_info, ctx.program_id)?;
    let store_config: StoreConfig = read_program_account(&store_config_info, ctx.program_id)?;

    let (expected_authorized_vault, _) = Pubkey::find_program_address(
        &[b"authorized_vault", ctx.accounts.authority.key().as_ref()],
        ctx.program_id,
    );
    require_keys_eq!(
        ctx.accounts.authorized_vault.key(),
        expected_authorized_vault,
        GameStateError::InvalidVaultAuthorization
    );
    let (expected_membership, _) = Pubkey::find_program_address(
        &[
            b"alliance_membership",
            ctx.accounts.authority.key().as_ref(),
        ],
        ctx.program_id,
    );
    require_keys_eq!(
        ctx.accounts.membership.key(),
        expected_membership,
        GameStateError::InvalidAllianceMember
    );
    let (expected_treasury, _) = Pubkey::find_program_address(
        &[b"alliance_treasury", alliance_info.key.as_ref()],
        ctx.program_id,
    );
    require_keys_eq!(
        ctx.accounts.alliance_treasury.key(),
        expected_treasury,
        GameStateError::InvalidAllianceMember
    );
    let (expected_game_config, _) = Pubkey::find_program_address(&[b"game_config"], ctx.program_id);
    require_keys_eq!(
        ctx.accounts.game_config.key(),
        expected_game_config,
        GameStateError::Unauthorized
    );
    let (expected_store_config, _) =
        Pubkey::find_program_address(&[b"store_config"], ctx.program_id);
    require_keys_eq!(
        ctx.accounts.store_config.key(),
        expected_store_config,
        GameStateError::Unauthorized
    );
    require_keys_eq!(
        membership.authority,
        ctx.accounts.authority.key(),
        GameStateError::Unauthorized
    );
    require_keys_eq!(
        membership.alliance,
        ctx.accounts.alliance.key(),
        GameStateError::InvalidAllianceMember
    );
    require!(membership.role > 0, GameStateError::InvalidAllianceMember);
    require_keys_eq!(
        alliance_treasury.alliance,
        ctx.accounts.alliance.key(),
        GameStateError::InvalidAllianceMember
    );
    require_keys_eq!(
        planet_deposit.authority,
        ctx.accounts.authority.key(),
        GameStateError::Unauthorized
    );
    require_keys_eq!(
        game_config.antimatter_mint,
        ctx.accounts.antimatter_mint.key(),
        GameStateError::InvalidAntimatterMint
    );
    require_active_vault(
        ctx.accounts.vault_signer.key(),
        &authorized_vault,
        ctx.accounts.authority.key(),
    )?;
    require!(
        metal > 0 || crystal > 0 || deuterium > 0 || antimatter > 0,
        GameStateError::InvalidArgs
    );
    require!(antimatter == 0, GameStateError::InvalidArgs);
    let now = chain_now()?;
    sync_alliance_periods(&mut membership, now);
    let mission = alliance_deposit_mission(period, mission_id)?;
    require!(
        metal >= mission.metal
            && crystal >= mission.crystal
            && deuterium >= mission.deuterium
            && antimatter >= mission.antimatter,
        GameStateError::AllianceMissionRequirementsNotMet
    );

    let bit = 1u64 << mission_id;
    let claimed_mask = match period {
        1 => membership.daily_claimed_mask,
        2 => membership.weekly_claimed_mask,
        3 => membership.monthly_claimed_mask,
        _ => return err!(GameStateError::InvalidAllianceMission),
    };
    require!(
        claimed_mask & bit == 0,
        GameStateError::AllianceMissionAlreadyClaimed
    );

    settle_planet_deposit_fields(&mut planet_deposit, now)?;
    require!(
        planet_deposit.metal >= metal,
        GameStateError::InsufficientMetal
    );
    require!(
        planet_deposit.crystal >= crystal,
        GameStateError::InsufficientCrystal
    );
    require!(
        planet_deposit.deuterium >= deuterium,
        GameStateError::InsufficientDeuterium
    );

    if antimatter > 0 {
        let antimatter_mint_info = ctx.accounts.antimatter_mint.to_account_info();
        let user_antimatter_info = ctx.accounts.user_antimatter_account.to_account_info();
        let treasury_antimatter_info = ctx.accounts.treasury_antimatter_account.to_account_info();
        let antimatter_mint: Mint = read_token_account(&antimatter_mint_info)?;
        let user_antimatter_account: TokenAccount = read_token_account(&user_antimatter_info)?;
        let treasury_antimatter_account: TokenAccount =
            read_token_account(&treasury_antimatter_info)?;
        require!(
            antimatter_mint.decimals == ANTIMATTER_DECIMALS,
            GameStateError::InvalidAntimatterMint
        );
        require_keys_eq!(
            user_antimatter_account.mint,
            ctx.accounts.antimatter_mint.key(),
            GameStateError::InvalidAntimatterMint
        );
        require_keys_eq!(
            user_antimatter_account.owner,
            ctx.accounts.vault_signer.key(),
            GameStateError::InvalidAntimatterAccount
        );
        require_keys_eq!(
            treasury_antimatter_account.mint,
            ctx.accounts.antimatter_mint.key(),
            GameStateError::InvalidAntimatterMint
        );
        require_keys_eq!(
            treasury_antimatter_account.owner,
            store_config.admin,
            GameStateError::InvalidAntimatterAccount
        );
        require_protocol_antimatter_treasury(
            ctx.accounts.treasury_antimatter_account.key(),
            store_config.admin,
            ctx.accounts.antimatter_mint.key(),
        )?;
        require!(
            user_antimatter_account.amount >= antimatter,
            GameStateError::InsufficientAntimatter
        );
        token::transfer(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: user_antimatter_info,
                    to: treasury_antimatter_info,
                    authority: ctx.accounts.vault_signer.to_account_info(),
                },
            ),
            antimatter,
        )?;
    }

    planet_deposit.metal = planet_deposit.metal.saturating_sub(metal);
    planet_deposit.crystal = planet_deposit.crystal.saturating_sub(crystal);
    planet_deposit.deuterium = planet_deposit.deuterium.saturating_sub(deuterium);

    credit_alliance_treasury(&mut alliance_treasury, metal, crystal, deuterium, antimatter)?;

    match period {
        1 => membership.daily_claimed_mask |= bit,
        2 => membership.weekly_claimed_mask |= bit,
        3 => membership.monthly_claimed_mask |= bit,
        _ => unreachable!(),
    }

    let base_xp = alliance_deposit_xp(&mission, metal, crystal, deuterium, antimatter);
    let xp = apply_bps_bonus(base_xp, alliance_logistics_xp_bonus_bps(&alliance_treasury));
    alliance.xp = alliance.xp.checked_add(xp).ok_or(GameStateError::InvalidArgs)?;
    alliance.total_missions_completed = alliance
        .total_missions_completed
        .checked_add(1)
        .ok_or(GameStateError::InvalidArgs)?;
    refresh_alliance_level(&mut alliance);

    write_program_account(&alliance_info, &alliance)?;
    write_program_account(&membership_info, &membership)?;
    write_program_account(&alliance_treasury_info, &alliance_treasury)?;
    write_planet_deposit_fields(&planet_state_info, &planet_deposit)?;
    Ok(())
}

fn read_program_account<T: AccountDeserialize>(
    account_info: &AccountInfo,
    program_id: &Pubkey,
) -> Result<T> {
    require_keys_eq!(
        *account_info.owner,
        *program_id,
        GameStateError::Unauthorized
    );
    let data = account_info.try_borrow_data()?;
    T::try_deserialize(&mut &data[..])
}

fn read_token_account<T: AccountDeserialize>(account_info: &AccountInfo) -> Result<T> {
    require_keys_eq!(*account_info.owner, token::ID, GameStateError::Unauthorized);
    let data = account_info.try_borrow_data()?;
    T::try_deserialize(&mut &data[..])
}

fn write_program_account<T: AccountSerialize>(
    account_info: &AccountInfo,
    account: &T,
) -> Result<()> {
    ensure_program_account_owner(account_info)?;
    let mut encoded = Vec::new();
    account.try_serialize(&mut encoded)?;
    let mut data = account_info.try_borrow_mut_data()?;
    require!(encoded.len() <= data.len(), GameStateError::InvalidArgs);
    data[..encoded.len()].copy_from_slice(&encoded);
    Ok(())
}

fn write_planet_owner_index(
    account_info: &AccountInfo,
    authority: Pubkey,
    slot: u32,
    planet: Pubkey,
    active: bool,
    bump: u8,
) -> Result<()> {
    write_program_account(
        account_info,
        &PlanetOwnerIndex {
            authority,
            slot,
            planet,
            active,
            bump,
        },
    )
}

fn create_or_update_planet_owner_index<'info>(
    account_info: &AccountInfo<'info>,
    payer_info: &AccountInfo<'info>,
    system_program_info: &AccountInfo<'info>,
    authority: Pubkey,
    slot: u32,
    planet: Pubkey,
    program_id: &Pubkey,
) -> Result<()> {
    let slot_bytes = slot.to_le_bytes();
    let (expected_index, bump) = Pubkey::find_program_address(
        &[b"planet_owner_index", authority.as_ref(), &slot_bytes],
        program_id,
    );
    require_keys_eq!(
        account_info.key(),
        expected_index,
        GameStateError::InvalidArgs
    );

    if account_info.data_is_empty() {
        let signer_seeds: &[&[&[u8]]] = &[&[
            b"planet_owner_index",
            authority.as_ref(),
            &slot_bytes,
            &[bump],
        ]];
        create_program_pda_account(
            account_info,
            payer_info,
            system_program_info,
            PLANET_OWNER_INDEX_SPACE,
            program_id,
            signer_seeds,
        )?;
    } else {
        let existing: PlanetOwnerIndex = read_program_account(account_info, program_id)?;
        require_keys_eq!(existing.authority, authority, GameStateError::Unauthorized);
        require!(existing.slot == slot, GameStateError::InvalidArgs);
        require!(
            !existing.active || existing.planet == planet,
            GameStateError::InvalidArgs
        );
    }

    write_planet_owner_index(account_info, authority, slot, planet, true, bump)
}

fn create_or_update_planet_ownership_registry<'info>(
    registry_info: &AccountInfo<'info>,
    payer_info: &AccountInfo<'info>,
    system_program_info: &AccountInfo<'info>,
    authority: Pubkey,
    planet: Pubkey,
    owner_index: Pubkey,
    program_id: &Pubkey,
) -> Result<()> {
    let (expected_registry, bump) =
        Pubkey::find_program_address(&[b"planet_ownership", planet.as_ref()], program_id);
    require_keys_eq!(registry_info.key(), expected_registry, GameStateError::InvalidArgs);

    if registry_info.data_is_empty() {
        let signer_seeds: &[&[&[u8]]] = &[&[
            b"planet_ownership",
            planet.as_ref(),
            &[bump],
        ]];
        create_program_pda_account(
            registry_info,
            payer_info,
            system_program_info,
            PLANET_OWNERSHIP_REGISTRY_SPACE,
            program_id,
            signer_seeds,
        )?;
    } else {
        let existing: PlanetOwnershipRegistry = read_program_account(registry_info, program_id)?;
        require_keys_eq!(existing.planet, planet, GameStateError::InvalidArgs);
        require_keys_eq!(existing.authority, authority, GameStateError::Unauthorized);
    }

    write_program_account(
        registry_info,
        &PlanetOwnershipRegistry {
            planet,
            authority,
            owner_index,
            bump,
        },
    )
}

fn deactivate_planet_owner_index(
    account_info: &AccountInfo<'_>,
    expected_index: Pubkey,
    authority: Pubkey,
    planet: Pubkey,
    program_id: &Pubkey,
) -> Result<()> {
    require_keys_eq!(account_info.key(), expected_index, GameStateError::InvalidArgs);
    let index: PlanetOwnerIndex = read_program_account(account_info, program_id)?;
    require_keys_eq!(index.authority, authority, GameStateError::Unauthorized);
    require_keys_eq!(index.planet, planet, GameStateError::InvalidArgs);
    require!(index.active, GameStateError::InvalidArgs);
    write_planet_owner_index(account_info, index.authority, index.slot, index.planet, false, index.bump)
}

fn read_checked_pubkey_at(data: &[u8], offset: usize) -> Result<Pubkey> {
    require!(data.len() >= offset + 32, GameStateError::InvalidArgs);
    let mut bytes = [0u8; 32];
    bytes.copy_from_slice(&data[offset..offset + 32]);
    Ok(Pubkey::new_from_array(bytes))
}

fn require_no_transfer_market_state(
    listing_index_info: &AccountInfo<'_>,
    market_obligation_info: &AccountInfo<'_>,
    planet: Pubkey,
) -> Result<()> {
    let (expected_listing_index, _) =
        Pubkey::find_program_address(&[b"planet_listing_index", planet.as_ref()], &MARKET_PROGRAM_ID);
    require_keys_eq!(
        listing_index_info.key(),
        expected_listing_index,
        GameStateError::InvalidArgs
    );
    if !listing_index_info.data_is_empty() {
        require_keys_eq!(
            *listing_index_info.owner,
            MARKET_PROGRAM_ID,
            GameStateError::UnauthorizedMarket
        );
        let data = listing_index_info.try_borrow_data()?;
        let listed_planet = read_checked_pubkey_at(&data, MARKET_LISTING_INDEX_PLANET_OFFSET)?;
        require_keys_eq!(listed_planet, planet, GameStateError::InvalidArgs);
        require!(
            data.len() > MARKET_LISTING_INDEX_ACTIVE_OFFSET,
            GameStateError::InvalidArgs
        );
        require!(
            data[MARKET_LISTING_INDEX_ACTIVE_OFFSET] == 0,
            GameStateError::PlanetListedForSale
        );
    }

    let (expected_market_obligation, _) = Pubkey::find_program_address(
        &[b"planet_market_obligation", planet.as_ref()],
        &MARKET_PROGRAM_ID,
    );
    require_keys_eq!(
        market_obligation_info.key(),
        expected_market_obligation,
        GameStateError::InvalidArgs
    );
    if !market_obligation_info.data_is_empty() {
        require_keys_eq!(
            *market_obligation_info.owner,
            MARKET_PROGRAM_ID,
            GameStateError::UnauthorizedMarket
        );
        let data = market_obligation_info.try_borrow_data()?;
        let obligation_planet = read_checked_pubkey_at(&data, MARKET_OBLIGATION_PLANET_OFFSET)?;
        require_keys_eq!(obligation_planet, planet, GameStateError::InvalidArgs);
        require!(
            data.len() >= MARKET_OBLIGATION_ACTIVE_OFFERS_OFFSET + 4,
            GameStateError::InvalidArgs
        );
        let active_offers = u32::from_le_bytes([
            data[MARKET_OBLIGATION_ACTIVE_OFFERS_OFFSET],
            data[MARKET_OBLIGATION_ACTIVE_OFFERS_OFFSET + 1],
            data[MARKET_OBLIGATION_ACTIVE_OFFERS_OFFSET + 2],
            data[MARKET_OBLIGATION_ACTIVE_OFFERS_OFFSET + 3],
        ]);
        require!(
            active_offers == 0,
            GameStateError::PlanetHasActiveMarketOffers
        );
    }

    Ok(())
}

struct PlanetDepositFields {
    authority: Pubkey,
    metal: u64,
    crystal: u64,
    deuterium: u64,
    metal_hour: u64,
    crystal_hour: u64,
    deuterium_hour: u64,
    energy_production: u64,
    energy_consumption: u64,
    metal_cap: u64,
    crystal_cap: u64,
    deuterium_cap: u64,
    last_update_ts: i64,
}

struct PlanetQuestFields {
    deposit: PlanetDepositFields,
    metal_mine: u8,
    crystal_mine: u8,
    deuterium_synthesizer: u8,
    solar_plant: u8,
    fusion_reactor: u8,
    robotics_factory: u8,
    nanite_factory: u8,
    shipyard: u8,
    metal_storage: u8,
    crystal_storage: u8,
    deuterium_tank: u8,
    research_lab: u8,
    energy_tech: u8,
    combustion_drive: u8,
    impulse_drive: u8,
    hyperspace_drive: u8,
    computer_tech: u8,
    astrophysics: u8,
    igr_network: u8,
    weapons_technology: u8,
    shielding_technology: u8,
    armor_technology: u8,
    small_cargo: u32,
    large_cargo: u32,
    light_fighter: u32,
    heavy_fighter: u32,
    cruiser: u32,
    battleship: u32,
    battlecruiser: u32,
    bomber: u32,
    destroyer: u32,
    deathstar: u32,
    recycler: u32,
    espionage_probe: u32,
    colony_ship: u32,
    solar_satellite: u32,
    rocket_launcher: u32,
    light_laser: u32,
    heavy_laser: u32,
    gauss_cannon: u32,
    ion_cannon: u32,
    plasma_turret: u32,
    small_shield_dome: u32,
    large_shield_dome: u32,
}

struct PlanetBuildFields {
    deposit: PlanetDepositFields,
    temperature: i16,
    max_fields: u16,
    used_fields: u16,
    metal_mine: u8,
    crystal_mine: u8,
    deuterium_synthesizer: u8,
    solar_plant: u8,
    fusion_reactor: u8,
    robotics_factory: u8,
    nanite_factory: u8,
    shipyard: u8,
    metal_storage: u8,
    crystal_storage: u8,
    deuterium_tank: u8,
    research_lab: u8,
    missile_silo: u8,
    energy_tech: u8,
    combustion_drive: u8,
    impulse_drive: u8,
    hyperspace_drive: u8,
    computer_tech: u8,
    astrophysics: u8,
    igr_network: u8,
    weapons_technology: u8,
    shielding_technology: u8,
    armor_technology: u8,
    small_cargo: u32,
    large_cargo: u32,
    light_fighter: u32,
    heavy_fighter: u32,
    cruiser: u32,
    battleship: u32,
    battlecruiser: u32,
    bomber: u32,
    destroyer: u32,
    deathstar: u32,
    recycler: u32,
    espionage_probe: u32,
    colony_ship: u32,
    solar_satellite: u32,
    research_queue_item: u8,
    research_queue_target: u8,
    research_finish_ts: i64,
    build_queue_item: u8,
    build_queue_target: u8,
    build_finish_ts: i64,
    ship_build_item: u8,
    ship_build_qty: u32,
    ship_build_finish_ts: i64,
    defense_build_item: u8,
    defense_build_qty: u32,
    defense_build_finish_ts: i64,
}

const PLANET_AUTHORITY_OFFSET: usize = 8;
const PLANET_TEMPERATURE_OFFSET: usize = 117;
const PLANET_MAX_FIELDS_OFFSET: usize = 119;
const PLANET_USED_FIELDS_OFFSET: usize = 121;
const PLANET_METAL_MINE_OFFSET: usize = 123;
const PLANET_CRYSTAL_MINE_OFFSET: usize = 124;
const PLANET_DEUTERIUM_SYNTHESIZER_OFFSET: usize = 125;
const PLANET_SOLAR_PLANT_OFFSET: usize = 126;
const PLANET_FUSION_REACTOR_OFFSET: usize = 127;
const PLANET_ROBOTICS_FACTORY_OFFSET: usize = 128;
const PLANET_NANITE_FACTORY_OFFSET: usize = 129;
const PLANET_SHIPYARD_OFFSET: usize = 130;
const PLANET_METAL_STORAGE_OFFSET: usize = 131;
const PLANET_CRYSTAL_STORAGE_OFFSET: usize = 132;
const PLANET_DEUTERIUM_TANK_OFFSET: usize = 133;
const PLANET_RESEARCH_LAB_OFFSET: usize = 134;
const PLANET_MISSILE_SILO_OFFSET: usize = 135;
const PLANET_ENERGY_TECH_OFFSET: usize = 136;
const PLANET_COMBUSTION_DRIVE_OFFSET: usize = 137;
const PLANET_IMPULSE_DRIVE_OFFSET: usize = 138;
const PLANET_HYPERSPACE_DRIVE_OFFSET: usize = 139;
const PLANET_COMPUTER_TECH_OFFSET: usize = 140;
const PLANET_ASTROPHYSICS_OFFSET: usize = 141;
const PLANET_IGR_NETWORK_OFFSET: usize = 142;
const PLANET_WEAPONS_TECHNOLOGY_OFFSET: usize = 143;
const PLANET_SHIELDING_TECHNOLOGY_OFFSET: usize = 144;
const PLANET_ARMOR_TECHNOLOGY_OFFSET: usize = 145;
const PLANET_RESEARCH_QUEUE_ITEM_OFFSET: usize = 146;
const PLANET_RESEARCH_QUEUE_TARGET_OFFSET: usize = 147;
const PLANET_RESEARCH_FINISH_TS_OFFSET: usize = 148;
const PLANET_BUILD_QUEUE_ITEM_OFFSET: usize = 156;
const PLANET_BUILD_QUEUE_TARGET_OFFSET: usize = 157;
const PLANET_BUILD_FINISH_TS_OFFSET: usize = 158;
const PLANET_METAL_OFFSET: usize = 166;
const PLANET_CRYSTAL_OFFSET: usize = 174;
const PLANET_DEUTERIUM_OFFSET: usize = 182;
const PLANET_METAL_HOUR_OFFSET: usize = 190;
const PLANET_CRYSTAL_HOUR_OFFSET: usize = 198;
const PLANET_DEUTERIUM_HOUR_OFFSET: usize = 206;
const PLANET_ENERGY_PRODUCTION_OFFSET: usize = 214;
const PLANET_ENERGY_CONSUMPTION_OFFSET: usize = 222;
const PLANET_METAL_CAP_OFFSET: usize = 230;
const PLANET_CRYSTAL_CAP_OFFSET: usize = 238;
const PLANET_DEUTERIUM_CAP_OFFSET: usize = 246;
const PLANET_LAST_UPDATE_TS_OFFSET: usize = 254;
const PLANET_SMALL_CARGO_OFFSET: usize = 310;
const PLANET_LARGE_CARGO_OFFSET: usize = 314;
const PLANET_LIGHT_FIGHTER_OFFSET: usize = 318;
const PLANET_HEAVY_FIGHTER_OFFSET: usize = 322;
const PLANET_CRUISER_OFFSET: usize = 326;
const PLANET_BATTLESHIP_OFFSET: usize = 330;
const PLANET_BATTLECRUISER_OFFSET: usize = 334;
const PLANET_BOMBER_OFFSET: usize = 338;
const PLANET_DESTROYER_OFFSET: usize = 342;
const PLANET_DEATHSTAR_OFFSET: usize = 346;
const PLANET_RECYCLER_OFFSET: usize = 350;
const PLANET_ESPIONAGE_PROBE_OFFSET: usize = 354;
const PLANET_COLONY_SHIP_OFFSET: usize = 358;
const PLANET_SOLAR_SATELLITE_OFFSET: usize = 362;
const PLANET_ROCKET_LAUNCHER_OFFSET: usize = 366;
const PLANET_LIGHT_LASER_OFFSET: usize = 370;
const PLANET_HEAVY_LASER_OFFSET: usize = 374;
const PLANET_GAUSS_CANNON_OFFSET: usize = 378;
const PLANET_ION_CANNON_OFFSET: usize = 382;
const PLANET_PLASMA_TURRET_OFFSET: usize = 386;
const PLANET_SMALL_SHIELD_DOME_OFFSET: usize = 390;
const PLANET_LARGE_SHIELD_DOME_OFFSET: usize = 394;
const PLANET_ACTIVE_MISSIONS_OFFSET: usize = 406;
const PLANET_MISSION_STATE_SIZE: usize = 142;
const PLANET_BUMP_OFFSET: usize =
    PLANET_ACTIVE_MISSIONS_OFFSET + 1 + PLANET_MISSION_STATE_SIZE * MAX_MISSIONS;
const PLANET_SHIP_BUILD_ITEM_OFFSET: usize = PLANET_BUMP_OFFSET + 1;
const PLANET_SHIP_BUILD_QTY_OFFSET: usize = PLANET_SHIP_BUILD_ITEM_OFFSET + 1;
const PLANET_SHIP_BUILD_FINISH_TS_OFFSET: usize = PLANET_SHIP_BUILD_QTY_OFFSET + 4;
const PLANET_DEFENSE_BUILD_ITEM_OFFSET: usize = PLANET_SHIP_BUILD_FINISH_TS_OFFSET + 8;
const PLANET_DEFENSE_BUILD_QTY_OFFSET: usize = PLANET_DEFENSE_BUILD_ITEM_OFFSET + 1;
const PLANET_DEFENSE_BUILD_FINISH_TS_OFFSET: usize = PLANET_DEFENSE_BUILD_QTY_OFFSET + 4;

fn require_planet_state_data(data: &[u8]) -> Result<()> {
    let discriminator = <PlanetState as anchor_lang::Discriminator>::DISCRIMINATOR;
    require!(
        data.len() >= PLANET_DEFENSE_BUILD_FINISH_TS_OFFSET + 8
            && data[..8] == discriminator,
        GameStateError::InvalidArgs
    );
    Ok(())
}

fn read_planet_deposit_fields(
    account_info: &AccountInfo,
    program_id: &Pubkey,
) -> Result<PlanetDepositFields> {
    require_keys_eq!(
        *account_info.owner,
        *program_id,
        GameStateError::Unauthorized
    );
    let data = account_info.try_borrow_data()?;
    require_planet_state_data(&data)?;
    Ok(PlanetDepositFields {
        authority: read_pubkey_at(&data, PLANET_AUTHORITY_OFFSET),
        metal: read_u64_at(&data, PLANET_METAL_OFFSET),
        crystal: read_u64_at(&data, PLANET_CRYSTAL_OFFSET),
        deuterium: read_u64_at(&data, PLANET_DEUTERIUM_OFFSET),
        metal_hour: read_u64_at(&data, PLANET_METAL_HOUR_OFFSET),
        crystal_hour: read_u64_at(&data, PLANET_CRYSTAL_HOUR_OFFSET),
        deuterium_hour: read_u64_at(&data, PLANET_DEUTERIUM_HOUR_OFFSET),
        energy_production: read_u64_at(&data, PLANET_ENERGY_PRODUCTION_OFFSET),
        energy_consumption: read_u64_at(&data, PLANET_ENERGY_CONSUMPTION_OFFSET),
        metal_cap: read_u64_at(&data, PLANET_METAL_CAP_OFFSET),
        crystal_cap: read_u64_at(&data, PLANET_CRYSTAL_CAP_OFFSET),
        deuterium_cap: read_u64_at(&data, PLANET_DEUTERIUM_CAP_OFFSET),
        last_update_ts: read_i64_at(&data, PLANET_LAST_UPDATE_TS_OFFSET),
    })
}

fn write_planet_deposit_fields(
    account_info: &AccountInfo,
    planet: &PlanetDepositFields,
) -> Result<()> {
    let mut data = account_info.try_borrow_mut_data()?;
    require_planet_state_data(&data)?;
    write_u64_at(&mut data, PLANET_METAL_OFFSET, planet.metal);
    write_u64_at(&mut data, PLANET_CRYSTAL_OFFSET, planet.crystal);
    write_u64_at(&mut data, PLANET_DEUTERIUM_OFFSET, planet.deuterium);
    write_i64_at(
        &mut data,
        PLANET_LAST_UPDATE_TS_OFFSET,
        planet.last_update_ts,
    );
    Ok(())
}

fn read_planet_quest_fields(
    account_info: &AccountInfo,
    program_id: &Pubkey,
) -> Result<PlanetQuestFields> {
    let deposit = read_planet_deposit_fields(account_info, program_id)?;
    let data = account_info.try_borrow_data()?;
    require_planet_state_data(&data)?;
    Ok(PlanetQuestFields {
        deposit,
        metal_mine: read_u8_at(&data, PLANET_METAL_MINE_OFFSET),
        crystal_mine: read_u8_at(&data, PLANET_CRYSTAL_MINE_OFFSET),
        deuterium_synthesizer: read_u8_at(&data, PLANET_DEUTERIUM_SYNTHESIZER_OFFSET),
        solar_plant: read_u8_at(&data, PLANET_SOLAR_PLANT_OFFSET),
        fusion_reactor: read_u8_at(&data, PLANET_FUSION_REACTOR_OFFSET),
        robotics_factory: read_u8_at(&data, PLANET_ROBOTICS_FACTORY_OFFSET),
        nanite_factory: read_u8_at(&data, PLANET_NANITE_FACTORY_OFFSET),
        shipyard: read_u8_at(&data, PLANET_SHIPYARD_OFFSET),
        metal_storage: read_u8_at(&data, PLANET_METAL_STORAGE_OFFSET),
        crystal_storage: read_u8_at(&data, PLANET_CRYSTAL_STORAGE_OFFSET),
        deuterium_tank: read_u8_at(&data, PLANET_DEUTERIUM_TANK_OFFSET),
        research_lab: read_u8_at(&data, PLANET_RESEARCH_LAB_OFFSET),
        energy_tech: read_u8_at(&data, PLANET_ENERGY_TECH_OFFSET),
        combustion_drive: read_u8_at(&data, PLANET_COMBUSTION_DRIVE_OFFSET),
        impulse_drive: read_u8_at(&data, PLANET_IMPULSE_DRIVE_OFFSET),
        hyperspace_drive: read_u8_at(&data, PLANET_HYPERSPACE_DRIVE_OFFSET),
        computer_tech: read_u8_at(&data, PLANET_COMPUTER_TECH_OFFSET),
        astrophysics: read_u8_at(&data, PLANET_ASTROPHYSICS_OFFSET),
        igr_network: read_u8_at(&data, PLANET_IGR_NETWORK_OFFSET),
        weapons_technology: read_u8_at(&data, PLANET_WEAPONS_TECHNOLOGY_OFFSET),
        shielding_technology: read_u8_at(&data, PLANET_SHIELDING_TECHNOLOGY_OFFSET),
        armor_technology: read_u8_at(&data, PLANET_ARMOR_TECHNOLOGY_OFFSET),
        small_cargo: read_u32_at(&data, PLANET_SMALL_CARGO_OFFSET),
        large_cargo: read_u32_at(&data, PLANET_LARGE_CARGO_OFFSET),
        light_fighter: read_u32_at(&data, PLANET_LIGHT_FIGHTER_OFFSET),
        heavy_fighter: read_u32_at(&data, PLANET_HEAVY_FIGHTER_OFFSET),
        cruiser: read_u32_at(&data, PLANET_CRUISER_OFFSET),
        battleship: read_u32_at(&data, PLANET_BATTLESHIP_OFFSET),
        battlecruiser: read_u32_at(&data, PLANET_BATTLECRUISER_OFFSET),
        bomber: read_u32_at(&data, PLANET_BOMBER_OFFSET),
        destroyer: read_u32_at(&data, PLANET_DESTROYER_OFFSET),
        deathstar: read_u32_at(&data, PLANET_DEATHSTAR_OFFSET),
        recycler: read_u32_at(&data, PLANET_RECYCLER_OFFSET),
        espionage_probe: read_u32_at(&data, PLANET_ESPIONAGE_PROBE_OFFSET),
        colony_ship: read_u32_at(&data, PLANET_COLONY_SHIP_OFFSET),
        solar_satellite: read_u32_at(&data, PLANET_SOLAR_SATELLITE_OFFSET),
        rocket_launcher: read_u32_at(&data, PLANET_ROCKET_LAUNCHER_OFFSET),
        light_laser: read_u32_at(&data, PLANET_LIGHT_LASER_OFFSET),
        heavy_laser: read_u32_at(&data, PLANET_HEAVY_LASER_OFFSET),
        gauss_cannon: read_u32_at(&data, PLANET_GAUSS_CANNON_OFFSET),
        ion_cannon: read_u32_at(&data, PLANET_ION_CANNON_OFFSET),
        plasma_turret: read_u32_at(&data, PLANET_PLASMA_TURRET_OFFSET),
        small_shield_dome: read_u32_at(&data, PLANET_SMALL_SHIELD_DOME_OFFSET),
        large_shield_dome: read_u32_at(&data, PLANET_LARGE_SHIELD_DOME_OFFSET),
    })
}

fn read_planet_build_fields(
    account_info: &AccountInfo,
    program_id: &Pubkey,
) -> Result<PlanetBuildFields> {
    let deposit = read_planet_deposit_fields(account_info, program_id)?;
    let data = account_info.try_borrow_data()?;
    require_planet_state_data(&data)?;
    Ok(PlanetBuildFields {
        deposit,
        temperature: read_i16_at(&data, PLANET_TEMPERATURE_OFFSET),
        max_fields: read_u16_at(&data, PLANET_MAX_FIELDS_OFFSET),
        used_fields: read_u16_at(&data, PLANET_USED_FIELDS_OFFSET),
        metal_mine: read_u8_at(&data, PLANET_METAL_MINE_OFFSET),
        crystal_mine: read_u8_at(&data, PLANET_CRYSTAL_MINE_OFFSET),
        deuterium_synthesizer: read_u8_at(&data, PLANET_DEUTERIUM_SYNTHESIZER_OFFSET),
        solar_plant: read_u8_at(&data, PLANET_SOLAR_PLANT_OFFSET),
        fusion_reactor: read_u8_at(&data, PLANET_FUSION_REACTOR_OFFSET),
        robotics_factory: read_u8_at(&data, PLANET_ROBOTICS_FACTORY_OFFSET),
        nanite_factory: read_u8_at(&data, PLANET_NANITE_FACTORY_OFFSET),
        shipyard: read_u8_at(&data, PLANET_SHIPYARD_OFFSET),
        metal_storage: read_u8_at(&data, PLANET_METAL_STORAGE_OFFSET),
        crystal_storage: read_u8_at(&data, PLANET_CRYSTAL_STORAGE_OFFSET),
        deuterium_tank: read_u8_at(&data, PLANET_DEUTERIUM_TANK_OFFSET),
        research_lab: read_u8_at(&data, PLANET_RESEARCH_LAB_OFFSET),
        missile_silo: read_u8_at(&data, PLANET_MISSILE_SILO_OFFSET),
        energy_tech: read_u8_at(&data, PLANET_ENERGY_TECH_OFFSET),
        combustion_drive: read_u8_at(&data, PLANET_COMBUSTION_DRIVE_OFFSET),
        impulse_drive: read_u8_at(&data, PLANET_IMPULSE_DRIVE_OFFSET),
        hyperspace_drive: read_u8_at(&data, PLANET_HYPERSPACE_DRIVE_OFFSET),
        computer_tech: read_u8_at(&data, PLANET_COMPUTER_TECH_OFFSET),
        astrophysics: read_u8_at(&data, PLANET_ASTROPHYSICS_OFFSET),
        igr_network: read_u8_at(&data, PLANET_IGR_NETWORK_OFFSET),
        weapons_technology: read_u8_at(&data, PLANET_WEAPONS_TECHNOLOGY_OFFSET),
        shielding_technology: read_u8_at(&data, PLANET_SHIELDING_TECHNOLOGY_OFFSET),
        armor_technology: read_u8_at(&data, PLANET_ARMOR_TECHNOLOGY_OFFSET),
        small_cargo: read_u32_at(&data, PLANET_SMALL_CARGO_OFFSET),
        large_cargo: read_u32_at(&data, PLANET_LARGE_CARGO_OFFSET),
        light_fighter: read_u32_at(&data, PLANET_LIGHT_FIGHTER_OFFSET),
        heavy_fighter: read_u32_at(&data, PLANET_HEAVY_FIGHTER_OFFSET),
        cruiser: read_u32_at(&data, PLANET_CRUISER_OFFSET),
        battleship: read_u32_at(&data, PLANET_BATTLESHIP_OFFSET),
        battlecruiser: read_u32_at(&data, PLANET_BATTLECRUISER_OFFSET),
        bomber: read_u32_at(&data, PLANET_BOMBER_OFFSET),
        destroyer: read_u32_at(&data, PLANET_DESTROYER_OFFSET),
        deathstar: read_u32_at(&data, PLANET_DEATHSTAR_OFFSET),
        recycler: read_u32_at(&data, PLANET_RECYCLER_OFFSET),
        espionage_probe: read_u32_at(&data, PLANET_ESPIONAGE_PROBE_OFFSET),
        colony_ship: read_u32_at(&data, PLANET_COLONY_SHIP_OFFSET),
        solar_satellite: read_u32_at(&data, PLANET_SOLAR_SATELLITE_OFFSET),
        research_queue_item: read_u8_at(&data, PLANET_RESEARCH_QUEUE_ITEM_OFFSET),
        research_queue_target: read_u8_at(&data, PLANET_RESEARCH_QUEUE_TARGET_OFFSET),
        research_finish_ts: read_i64_at(&data, PLANET_RESEARCH_FINISH_TS_OFFSET),
        build_queue_item: read_u8_at(&data, PLANET_BUILD_QUEUE_ITEM_OFFSET),
        build_queue_target: read_u8_at(&data, PLANET_BUILD_QUEUE_TARGET_OFFSET),
        build_finish_ts: read_i64_at(&data, PLANET_BUILD_FINISH_TS_OFFSET),
        ship_build_item: read_u8_at(&data, PLANET_SHIP_BUILD_ITEM_OFFSET),
        ship_build_qty: read_u32_at(&data, PLANET_SHIP_BUILD_QTY_OFFSET),
        ship_build_finish_ts: read_i64_at(&data, PLANET_SHIP_BUILD_FINISH_TS_OFFSET),
        defense_build_item: read_u8_at(&data, PLANET_DEFENSE_BUILD_ITEM_OFFSET),
        defense_build_qty: read_u32_at(&data, PLANET_DEFENSE_BUILD_QTY_OFFSET),
        defense_build_finish_ts: read_i64_at(&data, PLANET_DEFENSE_BUILD_FINISH_TS_OFFSET),
    })
}

fn write_planet_build_fields(account_info: &AccountInfo, planet: &PlanetBuildFields) -> Result<()> {
    write_planet_deposit_fields(account_info, &planet.deposit)?;
    let mut data = account_info.try_borrow_mut_data()?;
    require_planet_state_data(&data)?;
    write_u16_at(&mut data, PLANET_USED_FIELDS_OFFSET, planet.used_fields);
    write_u8_at(&mut data, PLANET_METAL_MINE_OFFSET, planet.metal_mine);
    write_u8_at(&mut data, PLANET_CRYSTAL_MINE_OFFSET, planet.crystal_mine);
    write_u8_at(
        &mut data,
        PLANET_DEUTERIUM_SYNTHESIZER_OFFSET,
        planet.deuterium_synthesizer,
    );
    write_u8_at(&mut data, PLANET_SOLAR_PLANT_OFFSET, planet.solar_plant);
    write_u8_at(
        &mut data,
        PLANET_FUSION_REACTOR_OFFSET,
        planet.fusion_reactor,
    );
    write_u8_at(
        &mut data,
        PLANET_ROBOTICS_FACTORY_OFFSET,
        planet.robotics_factory,
    );
    write_u8_at(
        &mut data,
        PLANET_NANITE_FACTORY_OFFSET,
        planet.nanite_factory,
    );
    write_u8_at(&mut data, PLANET_SHIPYARD_OFFSET, planet.shipyard);
    write_u8_at(&mut data, PLANET_METAL_STORAGE_OFFSET, planet.metal_storage);
    write_u8_at(
        &mut data,
        PLANET_CRYSTAL_STORAGE_OFFSET,
        planet.crystal_storage,
    );
    write_u8_at(
        &mut data,
        PLANET_DEUTERIUM_TANK_OFFSET,
        planet.deuterium_tank,
    );
    write_u8_at(&mut data, PLANET_RESEARCH_LAB_OFFSET, planet.research_lab);
    write_u8_at(&mut data, PLANET_MISSILE_SILO_OFFSET, planet.missile_silo);
    write_u8_at(&mut data, PLANET_ENERGY_TECH_OFFSET, planet.energy_tech);
    write_u8_at(
        &mut data,
        PLANET_COMBUSTION_DRIVE_OFFSET,
        planet.combustion_drive,
    );
    write_u8_at(&mut data, PLANET_IMPULSE_DRIVE_OFFSET, planet.impulse_drive);
    write_u8_at(
        &mut data,
        PLANET_HYPERSPACE_DRIVE_OFFSET,
        planet.hyperspace_drive,
    );
    write_u8_at(&mut data, PLANET_COMPUTER_TECH_OFFSET, planet.computer_tech);
    write_u8_at(&mut data, PLANET_ASTROPHYSICS_OFFSET, planet.astrophysics);
    write_u8_at(&mut data, PLANET_IGR_NETWORK_OFFSET, planet.igr_network);
    write_u8_at(
        &mut data,
        PLANET_WEAPONS_TECHNOLOGY_OFFSET,
        planet.weapons_technology,
    );
    write_u8_at(
        &mut data,
        PLANET_SHIELDING_TECHNOLOGY_OFFSET,
        planet.shielding_technology,
    );
    write_u8_at(
        &mut data,
        PLANET_ARMOR_TECHNOLOGY_OFFSET,
        planet.armor_technology,
    );
    write_u8_at(
        &mut data,
        PLANET_RESEARCH_QUEUE_ITEM_OFFSET,
        planet.research_queue_item,
    );
    write_u8_at(
        &mut data,
        PLANET_RESEARCH_QUEUE_TARGET_OFFSET,
        planet.research_queue_target,
    );
    write_i64_at(
        &mut data,
        PLANET_RESEARCH_FINISH_TS_OFFSET,
        planet.research_finish_ts,
    );
    write_u64_at(
        &mut data,
        PLANET_METAL_HOUR_OFFSET,
        planet.deposit.metal_hour,
    );
    write_u64_at(
        &mut data,
        PLANET_CRYSTAL_HOUR_OFFSET,
        planet.deposit.crystal_hour,
    );
    write_u64_at(
        &mut data,
        PLANET_DEUTERIUM_HOUR_OFFSET,
        planet.deposit.deuterium_hour,
    );
    write_u64_at(
        &mut data,
        PLANET_ENERGY_PRODUCTION_OFFSET,
        planet.deposit.energy_production,
    );
    write_u64_at(
        &mut data,
        PLANET_ENERGY_CONSUMPTION_OFFSET,
        planet.deposit.energy_consumption,
    );
    write_u64_at(&mut data, PLANET_METAL_CAP_OFFSET, planet.deposit.metal_cap);
    write_u64_at(
        &mut data,
        PLANET_CRYSTAL_CAP_OFFSET,
        planet.deposit.crystal_cap,
    );
    write_u64_at(
        &mut data,
        PLANET_DEUTERIUM_CAP_OFFSET,
        planet.deposit.deuterium_cap,
    );
    write_u8_at(
        &mut data,
        PLANET_BUILD_QUEUE_ITEM_OFFSET,
        planet.build_queue_item,
    );
    write_u8_at(
        &mut data,
        PLANET_BUILD_QUEUE_TARGET_OFFSET,
        planet.build_queue_target,
    );
    write_i64_at(
        &mut data,
        PLANET_BUILD_FINISH_TS_OFFSET,
        planet.build_finish_ts,
    );
    write_u32_at(&mut data, PLANET_SMALL_CARGO_OFFSET, planet.small_cargo);
    write_u32_at(&mut data, PLANET_LARGE_CARGO_OFFSET, planet.large_cargo);
    write_u32_at(&mut data, PLANET_LIGHT_FIGHTER_OFFSET, planet.light_fighter);
    write_u32_at(&mut data, PLANET_HEAVY_FIGHTER_OFFSET, planet.heavy_fighter);
    write_u32_at(&mut data, PLANET_CRUISER_OFFSET, planet.cruiser);
    write_u32_at(&mut data, PLANET_BATTLESHIP_OFFSET, planet.battleship);
    write_u32_at(&mut data, PLANET_BATTLECRUISER_OFFSET, planet.battlecruiser);
    write_u32_at(&mut data, PLANET_BOMBER_OFFSET, planet.bomber);
    write_u32_at(&mut data, PLANET_DESTROYER_OFFSET, planet.destroyer);
    write_u32_at(&mut data, PLANET_DEATHSTAR_OFFSET, planet.deathstar);
    write_u32_at(&mut data, PLANET_RECYCLER_OFFSET, planet.recycler);
    write_u32_at(
        &mut data,
        PLANET_ESPIONAGE_PROBE_OFFSET,
        planet.espionage_probe,
    );
    write_u32_at(&mut data, PLANET_COLONY_SHIP_OFFSET, planet.colony_ship);
    write_u32_at(
        &mut data,
        PLANET_SOLAR_SATELLITE_OFFSET,
        planet.solar_satellite,
    );
    write_u8_at(
        &mut data,
        PLANET_SHIP_BUILD_ITEM_OFFSET,
        planet.ship_build_item,
    );
    write_u32_at(
        &mut data,
        PLANET_SHIP_BUILD_QTY_OFFSET,
        planet.ship_build_qty,
    );
    write_i64_at(
        &mut data,
        PLANET_SHIP_BUILD_FINISH_TS_OFFSET,
        planet.ship_build_finish_ts,
    );
    write_u8_at(
        &mut data,
        PLANET_DEFENSE_BUILD_ITEM_OFFSET,
        planet.defense_build_item,
    );
    write_u32_at(
        &mut data,
        PLANET_DEFENSE_BUILD_QTY_OFFSET,
        planet.defense_build_qty,
    );
    write_i64_at(
        &mut data,
        PLANET_DEFENSE_BUILD_FINISH_TS_OFFSET,
        planet.defense_build_finish_ts,
    );
    Ok(())
}

fn settle_planet_deposit_fields(planet: &mut PlanetDepositFields, now: i64) -> Result<()> {
    require!(now >= 0, GameStateError::InvalidTimestamp);

    if planet.last_update_ts <= 0 {
        planet.last_update_ts = now;
        return Ok(());
    }

    require!(
        now >= planet.last_update_ts,
        GameStateError::InvalidTimestamp
    );

    if now == planet.last_update_ts {
        return Ok(());
    }

    let dt = (now - planet.last_update_ts).min(MAX_RESOURCE_SETTLEMENT_SECONDS) as u64;
    let eff_num = if planet.energy_consumption == 0 {
        10_000u64
    } else {
        (planet.energy_production * 10_000 / planet.energy_consumption).min(10_000)
    };

    let add_res = |current: u64, rate_per_hour: u64, cap: u64| -> u64 {
        let produced = rate_per_hour.saturating_mul(dt).saturating_mul(eff_num) / 3600 / 10_000;
        current.saturating_add(produced).min(cap)
    };

    planet.metal = add_res(planet.metal, planet.metal_hour, planet.metal_cap);
    planet.crystal = add_res(planet.crystal, planet.crystal_hour, planet.crystal_cap);
    planet.deuterium = add_res(
        planet.deuterium,
        planet.deuterium_hour,
        planet.deuterium_cap,
    );
    planet.last_update_ts = now;
    Ok(())
}

fn require_active_vault_for_live_planet(
    program_id: &Pubkey,
    vault_signer: Pubkey,
    authorized_vault: &AuthorizedVault,
    authorized_vault_key: Pubkey,
    planet_authority: Pubkey,
) -> Result<()> {
    let (expected, _) = Pubkey::find_program_address(
        &[b"authorized_vault", planet_authority.as_ref()],
        program_id,
    );
    require_keys_eq!(
        authorized_vault_key,
        expected,
        GameStateError::InvalidVaultAuthorization
    );
    require_active_vault(vault_signer, authorized_vault, planet_authority)
}

fn building_level_live(planet: &PlanetBuildFields, idx: u8) -> u8 {
    match idx {
        0 => planet.metal_mine,
        1 => planet.crystal_mine,
        2 => planet.deuterium_synthesizer,
        3 => planet.solar_plant,
        4 => planet.fusion_reactor,
        5 => planet.robotics_factory,
        6 => planet.nanite_factory,
        7 => planet.shipyard,
        8 => planet.metal_storage,
        9 => planet.crystal_storage,
        10 => planet.deuterium_tank,
        11 => planet.research_lab,
        12 => planet.missile_silo,
        _ => 0,
    }
}

fn set_building_level_live(planet: &mut PlanetBuildFields, idx: u8, level: u8) {
    match idx {
        0 => planet.metal_mine = level,
        1 => planet.crystal_mine = level,
        2 => planet.deuterium_synthesizer = level,
        3 => planet.solar_plant = level,
        4 => planet.fusion_reactor = level,
        5 => planet.robotics_factory = level,
        6 => planet.nanite_factory = level,
        7 => planet.shipyard = level,
        8 => planet.metal_storage = level,
        9 => planet.crystal_storage = level,
        10 => planet.deuterium_tank = level,
        11 => planet.research_lab = level,
        12 => planet.missile_silo = level,
        _ => {}
    }
}

fn research_level_live(planet: &PlanetBuildFields, idx: u8) -> u8 {
    match idx {
        0 => planet.energy_tech,
        1 => planet.combustion_drive,
        2 => planet.impulse_drive,
        3 => planet.hyperspace_drive,
        4 => planet.computer_tech,
        5 => planet.astrophysics,
        6 => planet.igr_network,
        7 => planet.weapons_technology,
        8 => planet.shielding_technology,
        9 => planet.armor_technology,
        _ => 0,
    }
}

fn set_research_level_live(planet: &mut PlanetBuildFields, idx: u8, level: u8) {
    match idx {
        0 => planet.energy_tech = level,
        1 => planet.combustion_drive = level,
        2 => planet.impulse_drive = level,
        3 => planet.hyperspace_drive = level,
        4 => planet.computer_tech = level,
        5 => planet.astrophysics = level,
        6 => planet.igr_network = level,
        7 => planet.weapons_technology = level,
        8 => planet.shielding_technology = level,
        9 => planet.armor_technology = level,
        _ => {}
    }
}

fn enforce_building_requirements_live(building_idx: u8, planet: &PlanetBuildFields) -> Result<()> {
    match building_idx {
        4 => require!(
            planet.deuterium_synthesizer >= 5 && planet.energy_tech >= 3,
            GameStateError::TechLocked
        ),
        6 => require!(
            planet.robotics_factory >= 10 && planet.computer_tech >= 10,
            GameStateError::TechLocked
        ),
        7 => require!(planet.robotics_factory >= 2, GameStateError::TechLocked),
        12 => require!(planet.shipyard >= 1, GameStateError::TechLocked),
        _ => {}
    }
    Ok(())
}

fn enforce_research_requirements_live(tech_idx: u8, planet: &PlanetBuildFields) -> Result<()> {
    match tech_idx {
        0 => require!(planet.research_lab >= 1, GameStateError::TechLocked),
        1 => require!(
            planet.research_lab >= 1 && planet.energy_tech >= 1,
            GameStateError::TechLocked
        ),
        2 => require!(
            planet.research_lab >= 2 && planet.energy_tech >= 1,
            GameStateError::TechLocked
        ),
        3 => require!(planet.research_lab >= 7, GameStateError::TechLocked),
        4 => require!(planet.research_lab >= 1, GameStateError::TechLocked),
        5 => require!(
            planet.research_lab >= 3 && planet.impulse_drive >= 3,
            GameStateError::TechLocked
        ),
        6 => require!(
            planet.research_lab >= 10 && planet.computer_tech >= 8,
            GameStateError::TechLocked
        ),
        7 => require!(planet.research_lab >= 4, GameStateError::TechLocked),
        8 => require!(
            planet.research_lab >= 6 && planet.energy_tech >= 3,
            GameStateError::TechLocked
        ),
        9 => require!(planet.research_lab >= 2, GameStateError::TechLocked),
        _ => {}
    }
    Ok(())
}

fn add_ship_live(planet: &mut PlanetBuildFields, ship_type: u8, quantity: u32) -> Result<()> {
    match ship_type {
        0 => planet.small_cargo = planet.small_cargo.saturating_add(quantity),
        1 => planet.large_cargo = planet.large_cargo.saturating_add(quantity),
        2 => planet.light_fighter = planet.light_fighter.saturating_add(quantity),
        3 => planet.heavy_fighter = planet.heavy_fighter.saturating_add(quantity),
        4 => planet.cruiser = planet.cruiser.saturating_add(quantity),
        5 => planet.battleship = planet.battleship.saturating_add(quantity),
        6 => planet.battlecruiser = planet.battlecruiser.saturating_add(quantity),
        7 => planet.bomber = planet.bomber.saturating_add(quantity),
        8 => planet.destroyer = planet.destroyer.saturating_add(quantity),
        9 => planet.deathstar = planet.deathstar.saturating_add(quantity),
        10 => planet.recycler = planet.recycler.saturating_add(quantity),
        11 => planet.espionage_probe = planet.espionage_probe.saturating_add(quantity),
        12 => planet.colony_ship = planet.colony_ship.saturating_add(quantity),
        13 => planet.solar_satellite = planet.solar_satellite.saturating_add(quantity),
        _ => return err!(GameStateError::InvalidShipType),
    }
    Ok(())
}

fn recalculate_rates_live(planet: &mut PlanetBuildFields) {
    planet.deposit.metal_hour = mine_rate(planet.metal_mine, 30);
    planet.deposit.crystal_hour = mine_rate(planet.crystal_mine, 20);

    let temp_factor = (240i32 - planet.temperature as i32).max(0) as u64;
    planet.deposit.deuterium_hour = if planet.deuterium_synthesizer == 0 {
        0
    } else {
        mine_rate(planet.deuterium_synthesizer, 10) * temp_factor / 200
    };

    let solar_prod = mine_rate(planet.solar_plant, 20);
    let satellite_prod = solar_satellite_energy_live(planet.temperature)
        .saturating_mul(planet.solar_satellite as u64);
    let fusion_prod = if planet.fusion_reactor == 0 {
        0
    } else {
        let base = mine_rate(planet.fusion_reactor, 30) * 180 / 100;
        base.saturating_mul(100 + planet.energy_tech as u64 * 10) / 100
    };

    planet.deposit.energy_production = solar_prod
        .saturating_add(satellite_prod)
        .saturating_add(fusion_prod);
    planet.deposit.energy_consumption = mine_rate(planet.metal_mine, 10)
        + mine_rate(planet.crystal_mine, 10)
        + mine_rate(planet.deuterium_synthesizer, 20);
    planet.deposit.metal_cap = store_cap(planet.metal_storage);
    planet.deposit.crystal_cap = store_cap(planet.crystal_storage);
    planet.deposit.deuterium_cap = store_cap(planet.deuterium_tank);
}

fn solar_satellite_energy_live(temperature: i16) -> u64 {
    ((temperature as i32 + 160).max(6) as u64 / 6).max(1)
}

fn start_build_live(planet: &mut PlanetBuildFields, building_idx: u8, now: i64) -> Result<()> {
    settle_planet_deposit_fields(&mut planet.deposit, now)?;
    let current = building_level_live(planet, building_idx);
    let next = current.checked_add(1).ok_or(GameStateError::InvalidArgs)?;
    let (cm, cc, cd) = upgrade_cost(building_idx, next as u64);

    require!(
        planet.build_finish_ts == 0 || now >= planet.build_finish_ts,
        GameStateError::QueueBusy
    );
    if building_idx == 7 {
        require!(
            planet.ship_build_item == 255
                || planet.ship_build_qty == 0
                || planet.ship_build_finish_ts <= 0,
            GameStateError::ShipyardQueueBusy
        );
        require!(
            planet.defense_build_item == 255
                || planet.defense_build_qty == 0
                || planet.defense_build_finish_ts <= 0,
            GameStateError::ShipyardQueueBusy
        );
    }
    if building_idx == 11 {
        require!(
            planet.research_queue_item == 255 || planet.research_finish_ts <= 0,
            GameStateError::ResearchQueueBusy
        );
    }
    require!(
        planet.used_fields < planet.max_fields,
        GameStateError::NoFields
    );
    enforce_building_requirements_live(building_idx, planet)?;
    require!(
        planet.deposit.metal >= cm,
        GameStateError::InsufficientMetal
    );
    require!(
        planet.deposit.crystal >= cc,
        GameStateError::InsufficientCrystal
    );
    require!(
        planet.deposit.deuterium >= cd,
        GameStateError::InsufficientDeuterium
    );

    planet.deposit.metal = planet.deposit.metal.saturating_sub(cm);
    planet.deposit.crystal = planet.deposit.crystal.saturating_sub(cc);
    planet.deposit.deuterium = planet.deposit.deuterium.saturating_sub(cd);

    let dur = build_seconds(building_idx, next as u64, planet.robotics_factory as u64);
    planet.build_queue_item = building_idx;
    planet.build_queue_target = next;
    planet.build_finish_ts = now.saturating_add(dur);
    planet.used_fields = planet.used_fields.saturating_add(1);
    Ok(())
}

fn finish_build_live(planet: &mut PlanetBuildFields, now: i64) -> Result<()> {
    require!(
        now >= planet.build_finish_ts,
        GameStateError::BuildNotFinished
    );
    settle_planet_deposit_fields(&mut planet.deposit, now)?;
    require!(planet.build_finish_ts > 0, GameStateError::NoBuild);

    set_building_level_live(planet, planet.build_queue_item, planet.build_queue_target);
    recalculate_rates_live(planet);
    planet.build_queue_item = 255;
    planet.build_queue_target = 0;
    planet.build_finish_ts = 0;
    Ok(())
}

fn start_research_live(planet: &mut PlanetBuildFields, tech_idx: u8, now: i64) -> Result<()> {
    settle_planet_deposit_fields(&mut planet.deposit, now)?;
    require!(tech_idx <= 9, GameStateError::InvalidTech);
    require!(planet.research_lab >= 1, GameStateError::LabTooLow);
    require!(
        !(planet.build_queue_item == 11 && planet.build_finish_ts > 0),
        GameStateError::ResearchQueueBusy
    );
    require!(
        planet.research_queue_item == 255,
        GameStateError::ResearchQueueBusy
    );

    let lab_req = research_lab_requirement(tech_idx);
    require!(planet.research_lab >= lab_req, GameStateError::LabTooLow);
    enforce_research_requirements_live(tech_idx, planet)?;

    let current = research_level_live(planet, tech_idx);
    let next = current.checked_add(1).ok_or(GameStateError::InvalidArgs)?;
    let (cm, cc, cd) = research_cost_for_level(tech_idx, current);

    require!(
        planet.deposit.metal >= cm,
        GameStateError::InsufficientMetal
    );
    require!(
        planet.deposit.crystal >= cc,
        GameStateError::InsufficientCrystal
    );
    require!(
        planet.deposit.deuterium >= cd,
        GameStateError::InsufficientDeuterium
    );

    planet.deposit.metal = planet.deposit.metal.saturating_sub(cm);
    planet.deposit.crystal = planet.deposit.crystal.saturating_sub(cc);
    planet.deposit.deuterium = planet.deposit.deuterium.saturating_sub(cd);

    let dur = research_seconds(next, planet.research_lab, planet.igr_network);
    planet.research_queue_item = tech_idx;
    planet.research_queue_target = next;
    planet.research_finish_ts = now.saturating_add(dur);
    Ok(())
}

fn finish_research_live(planet: &mut PlanetBuildFields, now: i64) -> Result<()> {
    require!(planet.research_finish_ts > 0, GameStateError::NoResearch);
    require!(
        now >= planet.research_finish_ts,
        GameStateError::ResearchNotFinished
    );
    settle_planet_deposit_fields(&mut planet.deposit, now)?;

    set_research_level_live(
        planet,
        planet.research_queue_item,
        planet.research_queue_target,
    );
    recalculate_rates_live(planet);
    planet.research_queue_item = 255;
    planet.research_queue_target = 0;
    planet.research_finish_ts = 0;
    Ok(())
}

fn finish_ship_build_live(planet: &mut PlanetBuildFields, now: i64) -> Result<()> {
    require!(
        now >= planet.ship_build_finish_ts,
        GameStateError::ShipBuildNotFinished
    );
    settle_planet_deposit_fields(&mut planet.deposit, now)?;
    require!(planet.ship_build_item != 255, GameStateError::NoShipBuild);
    require!(planet.ship_build_finish_ts > 0, GameStateError::NoShipBuild);

    let ship_type = planet.ship_build_item;
    let quantity = planet.ship_build_qty;
    add_ship_live(planet, ship_type, quantity)?;
    if ship_type == 13 {
        recalculate_rates_live(planet);
    }

    planet.ship_build_item = 255;
    planet.ship_build_qty = 0;
    planet.ship_build_finish_ts = 0;
    Ok(())
}

fn read_pubkey_at(data: &[u8], offset: usize) -> Pubkey {
    let mut bytes = [0u8; 32];
    bytes.copy_from_slice(&data[offset..offset + 32]);
    Pubkey::new_from_array(bytes)
}

fn read_u8_at(data: &[u8], offset: usize) -> u8 {
    data[offset]
}

fn read_u16_at(data: &[u8], offset: usize) -> u16 {
    let mut bytes = [0u8; 2];
    bytes.copy_from_slice(&data[offset..offset + 2]);
    u16::from_le_bytes(bytes)
}

fn read_u32_at(data: &[u8], offset: usize) -> u32 {
    let mut bytes = [0u8; 4];
    bytes.copy_from_slice(&data[offset..offset + 4]);
    u32::from_le_bytes(bytes)
}

fn read_u64_at(data: &[u8], offset: usize) -> u64 {
    let mut bytes = [0u8; 8];
    bytes.copy_from_slice(&data[offset..offset + 8]);
    u64::from_le_bytes(bytes)
}

fn read_i64_at(data: &[u8], offset: usize) -> i64 {
    let mut bytes = [0u8; 8];
    bytes.copy_from_slice(&data[offset..offset + 8]);
    i64::from_le_bytes(bytes)
}

fn read_i16_at(data: &[u8], offset: usize) -> i16 {
    let mut bytes = [0u8; 2];
    bytes.copy_from_slice(&data[offset..offset + 2]);
    i16::from_le_bytes(bytes)
}

fn write_u8_at(data: &mut [u8], offset: usize, value: u8) {
    data[offset] = value;
}

fn write_u16_at(data: &mut [u8], offset: usize, value: u16) {
    data[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
}

fn write_u32_at(data: &mut [u8], offset: usize, value: u32) {
    data[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

fn write_u64_at(data: &mut [u8], offset: usize, value: u64) {
    data[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
}

fn write_i64_at(data: &mut [u8], offset: usize, value: i64) {
    data[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
}

fn raw_game_error<T>(error: GameStateError) -> Result<T> {
    Err(ProgramError::Custom(6000 + error as u32).into())
}

fn start_ship_build_bytes(
    account_info: &AccountInfo,
    ship_type: u8,
    quantity: u32,
    now: i64,
) -> Result<()> {
    if quantity == 0 {
        return raw_game_error(GameStateError::InvalidArgs);
    }
    if now < 0 {
        return raw_game_error(GameStateError::InvalidTimestamp);
    }

    let mut data = account_info.try_borrow_mut_data()?;
    if require_planet_state_data(&data).is_err() {
        return raw_game_error(GameStateError::InvalidArgs);
    }

    let last_update_ts = read_i64_at(&data, PLANET_LAST_UPDATE_TS_OFFSET);
    if last_update_ts > now {
        return raw_game_error(GameStateError::InvalidTimestamp);
    }

    let mut metal = read_u64_at(&data, PLANET_METAL_OFFSET);
    let mut crystal = read_u64_at(&data, PLANET_CRYSTAL_OFFSET);
    let mut deuterium = read_u64_at(&data, PLANET_DEUTERIUM_OFFSET);

    if last_update_ts > 0 {
        let dt = (now - last_update_ts).min(MAX_RESOURCE_SETTLEMENT_SECONDS) as u64;
        if dt > 0 {
            let energy_production = read_u64_at(&data, PLANET_ENERGY_PRODUCTION_OFFSET);
            let energy_consumption = read_u64_at(&data, PLANET_ENERGY_CONSUMPTION_OFFSET);
            let (eff_num, eff_den) =
                if energy_consumption == 0 || energy_production >= energy_consumption {
                    (1u128, 1u128)
                } else {
                    (energy_production as u128, energy_consumption as u128)
                };
            let gain = |rate: u64| -> u64 {
                ((rate as u128)
                    .saturating_mul(dt as u128)
                    .saturating_mul(eff_num)
                    / 3600u128
                    / eff_den) as u64
            };
            metal = metal
                .saturating_add(gain(read_u64_at(&data, PLANET_METAL_HOUR_OFFSET)))
                .min(read_u64_at(&data, PLANET_METAL_CAP_OFFSET));
            crystal = crystal
                .saturating_add(gain(read_u64_at(&data, PLANET_CRYSTAL_HOUR_OFFSET)))
                .min(read_u64_at(&data, PLANET_CRYSTAL_CAP_OFFSET));
            deuterium = deuterium
                .saturating_add(gain(read_u64_at(&data, PLANET_DEUTERIUM_HOUR_OFFSET)))
                .min(read_u64_at(&data, PLANET_DEUTERIUM_CAP_OFFSET));
        }
    }

    let shipyard = read_u8_at(&data, PLANET_SHIPYARD_OFFSET);
    if shipyard < 1 {
        return raw_game_error(GameStateError::ShipyardTooLow);
    }
    let build_queue_item = read_u8_at(&data, PLANET_BUILD_QUEUE_ITEM_OFFSET);
    let build_finish_ts = read_i64_at(&data, PLANET_BUILD_FINISH_TS_OFFSET);
    if build_queue_item == 7 && build_finish_ts > 0 {
        return raw_game_error(GameStateError::ShipyardQueueBusy);
    }
    let queue_item = read_u8_at(&data, PLANET_SHIP_BUILD_ITEM_OFFSET);
    let queue_qty = read_u32_at(&data, PLANET_SHIP_BUILD_QTY_OFFSET);
    let queue_finish_ts = read_i64_at(&data, PLANET_SHIP_BUILD_FINISH_TS_OFFSET);
    msg!(
        "ship gate: type {} qty {} sy {} queue {} {} {}",
        ship_type,
        quantity,
        shipyard,
        queue_item,
        queue_qty,
        queue_finish_ts
    );
    let queue_empty = queue_item == 255 || (queue_qty == 0 && queue_finish_ts == 0);
    if !queue_empty {
        return raw_game_error(GameStateError::ShipyardQueueBusy);
    }
    let defense_queue_item = read_u8_at(&data, PLANET_DEFENSE_BUILD_ITEM_OFFSET);
    let defense_queue_qty = read_u32_at(&data, PLANET_DEFENSE_BUILD_QTY_OFFSET);
    let defense_queue_finish_ts = read_i64_at(&data, PLANET_DEFENSE_BUILD_FINISH_TS_OFFSET);
    let defense_queue_empty =
        defense_queue_item == 255 || (defense_queue_qty == 0 && defense_queue_finish_ts == 0);
    if !defense_queue_empty {
        return raw_game_error(GameStateError::ShipyardQueueBusy);
    }

    let tech_ok = match ship_type {
        0 => shipyard >= 2 && read_u8_at(&data, PLANET_COMBUSTION_DRIVE_OFFSET) >= 2,
        1 => shipyard >= 4 && read_u8_at(&data, PLANET_COMBUSTION_DRIVE_OFFSET) >= 6,
        2 => shipyard >= 1,
        3 => {
            shipyard >= 3
                && read_u8_at(&data, PLANET_ARMOR_TECHNOLOGY_OFFSET) >= 2
                && read_u8_at(&data, PLANET_IMPULSE_DRIVE_OFFSET) >= 2
        }
        4 => shipyard >= 5 && read_u8_at(&data, PLANET_IMPULSE_DRIVE_OFFSET) >= 4,
        5 => shipyard >= 7 && read_u8_at(&data, PLANET_HYPERSPACE_DRIVE_OFFSET) >= 4,
        6 => {
            shipyard >= 8
                && read_u8_at(&data, PLANET_HYPERSPACE_DRIVE_OFFSET) >= 5
                && read_u8_at(&data, PLANET_COMPUTER_TECH_OFFSET) >= 5
                && read_u8_at(&data, PLANET_WEAPONS_TECHNOLOGY_OFFSET) >= 5
        }
        7 => {
            shipyard >= 8
                && read_u8_at(&data, PLANET_IMPULSE_DRIVE_OFFSET) >= 6
                && read_u8_at(&data, PLANET_HYPERSPACE_DRIVE_OFFSET) >= 5
                && read_u8_at(&data, PLANET_WEAPONS_TECHNOLOGY_OFFSET) >= 5
        }
        8 => {
            shipyard >= 9
                && read_u8_at(&data, PLANET_HYPERSPACE_DRIVE_OFFSET) >= 6
                && read_u8_at(&data, PLANET_ARMOR_TECHNOLOGY_OFFSET) >= 6
        }
        9 => {
            shipyard >= 12
                && read_u8_at(&data, PLANET_HYPERSPACE_DRIVE_OFFSET) >= 7
                && read_u8_at(&data, PLANET_WEAPONS_TECHNOLOGY_OFFSET) >= 10
                && read_u8_at(&data, PLANET_ENERGY_TECH_OFFSET) >= 12
        }
        10 => {
            shipyard >= 4
                && read_u8_at(&data, PLANET_COMBUSTION_DRIVE_OFFSET) >= 6
                && read_u8_at(&data, PLANET_SHIELDING_TECHNOLOGY_OFFSET) >= 2
        }
        11 => shipyard >= 3 && read_u8_at(&data, PLANET_COMPUTER_TECH_OFFSET) >= 2,
        12 => {
            shipyard >= 4
                && read_u8_at(&data, PLANET_IMPULSE_DRIVE_OFFSET) >= 3
                && read_u8_at(&data, PLANET_ASTROPHYSICS_OFFSET) >= 3
        }
        13 => shipyard >= 1,
        _ => return raw_game_error(GameStateError::InvalidShipType),
    };
    msg!("ship gate: tech ok {}", tech_ok);
    if !tech_ok {
        return raw_game_error(GameStateError::TechLocked);
    }

    let (cm, cc, cd) = ship_cost(ship_type);
    if cm == 0 && cc == 0 && cd == 0 && ship_type != 11 {
        return raw_game_error(GameStateError::InvalidShipType);
    }
    let total_m = cm.saturating_mul(quantity as u64);
    let total_c = cc.saturating_mul(quantity as u64);
    let total_d = cd.saturating_mul(quantity as u64);
    if metal < total_m {
        return raw_game_error(GameStateError::InsufficientMetal);
    }
    if crystal < total_c {
        return raw_game_error(GameStateError::InsufficientCrystal);
    }
    if deuterium < total_d {
        return raw_game_error(GameStateError::InsufficientDeuterium);
    }

    metal = metal.saturating_sub(total_m);
    crystal = crystal.saturating_sub(total_c);
    deuterium = deuterium.saturating_sub(total_d);

    let nanite = read_u8_at(&data, PLANET_NANITE_FACTORY_OFFSET);
    let dur = ship_build_seconds(ship_type, quantity, shipyard, nanite);
    write_u64_at(&mut data, PLANET_METAL_OFFSET, metal);
    write_u64_at(&mut data, PLANET_CRYSTAL_OFFSET, crystal);
    write_u64_at(&mut data, PLANET_DEUTERIUM_OFFSET, deuterium);
    write_i64_at(&mut data, PLANET_LAST_UPDATE_TS_OFFSET, now);
    write_u8_at(&mut data, PLANET_SHIP_BUILD_ITEM_OFFSET, ship_type);
    write_u32_at(&mut data, PLANET_SHIP_BUILD_QTY_OFFSET, quantity);
    write_i64_at(
        &mut data,
        PLANET_SHIP_BUILD_FINISH_TS_OFFSET,
        now.saturating_add(dur),
    );
    Ok(())
}

fn start_defense_build_bytes(
    account_info: &AccountInfo,
    defense_type: u8,
    quantity: u32,
    now: i64,
) -> Result<()> {
    require!(quantity > 0, GameStateError::InvalidArgs);
    require!(now >= 0, GameStateError::InvalidTimestamp);

    let mut data = account_info.try_borrow_mut_data()?;
    require_planet_state_data(&data)?;

    let last_update_ts = read_i64_at(&data, PLANET_LAST_UPDATE_TS_OFFSET);
    require!(last_update_ts <= now, GameStateError::InvalidTimestamp);

    let mut metal = read_u64_at(&data, PLANET_METAL_OFFSET);
    let mut crystal = read_u64_at(&data, PLANET_CRYSTAL_OFFSET);
    let mut deuterium = read_u64_at(&data, PLANET_DEUTERIUM_OFFSET);

    if last_update_ts > 0 {
        let dt = (now - last_update_ts).min(MAX_RESOURCE_SETTLEMENT_SECONDS) as u64;
        if dt > 0 {
            let energy_production = read_u64_at(&data, PLANET_ENERGY_PRODUCTION_OFFSET);
            let energy_consumption = read_u64_at(&data, PLANET_ENERGY_CONSUMPTION_OFFSET);
            let (eff_num, eff_den) =
                if energy_consumption == 0 || energy_production >= energy_consumption {
                    (1u128, 1u128)
                } else {
                    (energy_production as u128, energy_consumption as u128)
                };
            let gain = |rate: u64| -> u64 {
                ((rate as u128)
                    .saturating_mul(dt as u128)
                    .saturating_mul(eff_num)
                    / 3600u128
                    / eff_den) as u64
            };
            metal = metal
                .saturating_add(gain(read_u64_at(&data, PLANET_METAL_HOUR_OFFSET)))
                .min(read_u64_at(&data, PLANET_METAL_CAP_OFFSET));
            crystal = crystal
                .saturating_add(gain(read_u64_at(&data, PLANET_CRYSTAL_HOUR_OFFSET)))
                .min(read_u64_at(&data, PLANET_CRYSTAL_CAP_OFFSET));
            deuterium = deuterium
                .saturating_add(gain(read_u64_at(&data, PLANET_DEUTERIUM_HOUR_OFFSET)))
                .min(read_u64_at(&data, PLANET_DEUTERIUM_CAP_OFFSET));
        }
    }

    let shipyard = read_u8_at(&data, PLANET_SHIPYARD_OFFSET);
    require!(shipyard >= 1, GameStateError::ShipyardTooLow);
    let build_queue_item = read_u8_at(&data, PLANET_BUILD_QUEUE_ITEM_OFFSET);
    let build_finish_ts = read_i64_at(&data, PLANET_BUILD_FINISH_TS_OFFSET);
    require!(
        !(build_queue_item == 7 && build_finish_ts > 0),
        GameStateError::ShipyardQueueBusy
    );

    let queue_item = read_u8_at(&data, PLANET_DEFENSE_BUILD_ITEM_OFFSET);
    let queue_qty = read_u32_at(&data, PLANET_DEFENSE_BUILD_QTY_OFFSET);
    let queue_finish_ts = read_i64_at(&data, PLANET_DEFENSE_BUILD_FINISH_TS_OFFSET);
    let queue_empty = queue_item == 255 || (queue_qty == 0 && queue_finish_ts == 0);
    if !queue_empty {
        return err!(GameStateError::ShipyardQueueBusy);
    }
    let ship_queue_item = read_u8_at(&data, PLANET_SHIP_BUILD_ITEM_OFFSET);
    let ship_queue_qty = read_u32_at(&data, PLANET_SHIP_BUILD_QTY_OFFSET);
    let ship_queue_finish_ts = read_i64_at(&data, PLANET_SHIP_BUILD_FINISH_TS_OFFSET);
    let ship_queue_empty =
        ship_queue_item == 255 || (ship_queue_qty == 0 && ship_queue_finish_ts == 0);
    if !ship_queue_empty {
        return err!(GameStateError::ShipyardQueueBusy);
    }

    let tech_ok = match defense_type {
        0 => true,
        1 => shipyard >= 2,
        2 => shipyard >= 4,
        3 => shipyard >= 6 && read_u8_at(&data, PLANET_WEAPONS_TECHNOLOGY_OFFSET) >= 3,
        4 => shipyard >= 4 && read_u8_at(&data, PLANET_SHIELDING_TECHNOLOGY_OFFSET) >= 2,
        5 => {
            shipyard >= 8
                && read_u8_at(&data, PLANET_SHIELDING_TECHNOLOGY_OFFSET) >= 8
                && read_u8_at(&data, PLANET_WEAPONS_TECHNOLOGY_OFFSET) >= 10
                && read_u8_at(&data, PLANET_ENERGY_TECH_OFFSET) >= 8
        }
        6 => {
            require!(quantity == 1, GameStateError::InvalidDefenseType);
            read_u8_at(&data, PLANET_SHIELDING_TECHNOLOGY_OFFSET) >= 2
                && read_u32_at(&data, PLANET_SMALL_SHIELD_DOME_OFFSET) == 0
        }
        7 => {
            require!(quantity == 1, GameStateError::InvalidDefenseType);
            read_u8_at(&data, PLANET_SHIELDING_TECHNOLOGY_OFFSET) >= 6
                && read_u32_at(&data, PLANET_LARGE_SHIELD_DOME_OFFSET) == 0
        }
        8 | 9 => return err!(GameStateError::InvalidDefenseType),
        _ => return err!(GameStateError::InvalidDefenseType),
    };
    if !tech_ok {
        return err!(GameStateError::TechLocked);
    }

    let (cm, cc, cd) = defense_cost(defense_type);
    require!(
        cm != 0 || cc != 0 || cd != 0,
        GameStateError::InvalidDefenseType
    );
    let total_m = cm.saturating_mul(quantity as u64);
    let total_c = cc.saturating_mul(quantity as u64);
    let total_d = cd.saturating_mul(quantity as u64);
    require!(metal >= total_m, GameStateError::InsufficientMetal);
    require!(crystal >= total_c, GameStateError::InsufficientCrystal);
    require!(deuterium >= total_d, GameStateError::InsufficientDeuterium);

    metal = metal.saturating_sub(total_m);
    crystal = crystal.saturating_sub(total_c);
    deuterium = deuterium.saturating_sub(total_d);

    let nanite = read_u8_at(&data, PLANET_NANITE_FACTORY_OFFSET);
    let dur = defense_build_seconds(defense_type, quantity, shipyard, nanite);
    write_u64_at(&mut data, PLANET_METAL_OFFSET, metal);
    write_u64_at(&mut data, PLANET_CRYSTAL_OFFSET, crystal);
    write_u64_at(&mut data, PLANET_DEUTERIUM_OFFSET, deuterium);
    write_i64_at(&mut data, PLANET_LAST_UPDATE_TS_OFFSET, now);
    write_u8_at(&mut data, PLANET_DEFENSE_BUILD_ITEM_OFFSET, defense_type);
    write_u32_at(&mut data, PLANET_DEFENSE_BUILD_QTY_OFFSET, quantity);
    write_i64_at(
        &mut data,
        PLANET_DEFENSE_BUILD_FINISH_TS_OFFSET,
        now.saturating_add(dur),
    );
    Ok(())
}

fn defense_count_offset(defense_type: u8) -> Result<usize> {
    match defense_type {
        0 => Ok(PLANET_ROCKET_LAUNCHER_OFFSET),
        1 => Ok(PLANET_LIGHT_LASER_OFFSET),
        2 => Ok(PLANET_HEAVY_LASER_OFFSET),
        3 => Ok(PLANET_GAUSS_CANNON_OFFSET),
        4 => Ok(PLANET_ION_CANNON_OFFSET),
        5 => Ok(PLANET_PLASMA_TURRET_OFFSET),
        6 => Ok(PLANET_SMALL_SHIELD_DOME_OFFSET),
        7 => Ok(PLANET_LARGE_SHIELD_DOME_OFFSET),
        _ => err!(GameStateError::InvalidDefenseType),
    }
}

fn finish_defense_build_bytes(account_info: &AccountInfo, now: i64) -> Result<()> {
    require!(now >= 0, GameStateError::InvalidTimestamp);
    let mut data = account_info.try_borrow_mut_data()?;
    require_planet_state_data(&data)?;

    let defense_type = read_u8_at(&data, PLANET_DEFENSE_BUILD_ITEM_OFFSET);
    let quantity = read_u32_at(&data, PLANET_DEFENSE_BUILD_QTY_OFFSET);
    let finish_ts = read_i64_at(&data, PLANET_DEFENSE_BUILD_FINISH_TS_OFFSET);
    require!(
        quantity > 0 && finish_ts > 0,
        GameStateError::NoDefenseBuild
    );
    require!(defense_type != 255, GameStateError::NoDefenseBuild);
    require!(now >= finish_ts, GameStateError::DefenseBuildNotFinished);

    let offset = defense_count_offset(defense_type)?;
    let current = read_u32_at(&data, offset);
    write_u32_at(&mut data, offset, current.saturating_add(quantity));
    write_u8_at(&mut data, PLANET_DEFENSE_BUILD_ITEM_OFFSET, 255);
    write_u32_at(&mut data, PLANET_DEFENSE_BUILD_QTY_OFFSET, 0);
    write_i64_at(&mut data, PLANET_DEFENSE_BUILD_FINISH_TS_OFFSET, 0);
    Ok(())
}

pub fn upgrade_alliance_building(
    ctx: Context<UpgradeAllianceBuilding>,
    building_id: u8,
) -> Result<()> {
    require!(
        ctx.accounts.membership.role == 2,
        GameStateError::AllianceLeaderRequired
    );
    let current = alliance_building_level(&ctx.accounts.alliance_treasury, building_id)?;
    require!(
        current < ALLIANCE_BUILDING_MAX_LEVEL,
        GameStateError::InvalidAllianceBuilding
    );
    let next = current.saturating_add(1);
    let (metal, crystal, deuterium, antimatter, base_xp) =
        alliance_building_cost(building_id, next)?;
    let (metal, crystal, deuterium, antimatter) = apply_alliance_upgrade_discounts(
        metal,
        crystal,
        deuterium,
        antimatter,
        &ctx.accounts.alliance_treasury,
    );
    let xp = apply_bps_bonus(
        base_xp,
        alliance_research_grid_xp_bonus_bps(&ctx.accounts.alliance_treasury),
    );
    require!(
        ctx.accounts.alliance_treasury.metal >= metal
            && ctx.accounts.alliance_treasury.crystal >= crystal
            && ctx.accounts.alliance_treasury.deuterium >= deuterium
            && ctx.accounts.alliance_treasury.antimatter >= antimatter,
        GameStateError::AllianceTreasuryNotEnoughResources
    );

    ctx.accounts.alliance_treasury.metal =
        ctx.accounts.alliance_treasury.metal.saturating_sub(metal);
    ctx.accounts.alliance_treasury.crystal = ctx
        .accounts
        .alliance_treasury
        .crystal
        .saturating_sub(crystal);
    ctx.accounts.alliance_treasury.deuterium = ctx
        .accounts
        .alliance_treasury
        .deuterium
        .saturating_sub(deuterium);
    ctx.accounts.alliance_treasury.antimatter = ctx
        .accounts
        .alliance_treasury
        .antimatter
        .saturating_sub(antimatter);
    set_alliance_building_level(&mut ctx.accounts.alliance_treasury, building_id, next)?;

    ctx.accounts.alliance.xp = ctx.accounts.alliance.xp.saturating_add(xp);
    refresh_alliance_level(&mut ctx.accounts.alliance);
    Ok(())
}

fn sync_alliance_periods(membership: &mut AllianceMembership, now: i64) {
    let daily_epoch = daily_epoch(now);
    let weekly_epoch = weekly_epoch(now);
    let monthly_epoch = monthly_epoch(now);

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

fn refresh_alliance_level(alliance: &mut AllianceState) {
    let mut low = alliance.level;
    let mut high = u16::MAX;
    while low < high {
        let mid = low + (high - low) / 2 + 1;
        if alliance.xp >= alliance_level_threshold(mid) {
            low = mid;
        } else {
            high = mid - 1;
        }
    }
    alliance.level = low;
    alliance.max_members = alliance_max_members(alliance.level);
}

fn sync_quest_periods(quest: &mut Account<QuestState>, now: i64) {
    let daily_epoch = daily_epoch(now);
    let weekly_epoch = weekly_epoch(now);
    let monthly_epoch = monthly_epoch(now);

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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum QuestProgressMetric {
    StorePacksBought,
    AntimatterSpent,
    PlanetsColonized,
    AttacksResolved,
    TransportsResolved,
    SpyMissionsResolved,
}

fn sync_quest_progress_periods(progress: &mut QuestProgressState, now: i64) {
    let daily_epoch = daily_epoch(now);
    let weekly_epoch = weekly_epoch(now);
    let monthly_epoch = monthly_epoch(now);

    if progress.daily_epoch != daily_epoch {
        progress.daily_epoch = daily_epoch;
        progress.daily_store_packs_bought = 0;
        progress.daily_antimatter_spent = 0;
        progress.daily_planets_colonized = 0;
        progress.daily_attacks_resolved = 0;
        progress.daily_transports_resolved = 0;
        progress.daily_spy_missions_resolved = 0;
    }
    if progress.weekly_epoch != weekly_epoch {
        progress.weekly_epoch = weekly_epoch;
        progress.weekly_store_packs_bought = 0;
        progress.weekly_antimatter_spent = 0;
        progress.weekly_planets_colonized = 0;
        progress.weekly_attacks_resolved = 0;
        progress.weekly_transports_resolved = 0;
        progress.weekly_spy_missions_resolved = 0;
    }
    if progress.monthly_epoch != monthly_epoch {
        progress.monthly_epoch = monthly_epoch;
        progress.monthly_store_packs_bought = 0;
        progress.monthly_antimatter_spent = 0;
        progress.monthly_planets_colonized = 0;
        progress.monthly_attacks_resolved = 0;
        progress.monthly_transports_resolved = 0;
        progress.monthly_spy_missions_resolved = 0;
    }
}

fn validate_quest_progress_pda(
    account_info: &AccountInfo,
    authority: Pubkey,
    program_id: &Pubkey,
) -> Result<QuestProgressState> {
    let (expected, _) =
        Pubkey::find_program_address(&[b"quest_progress", authority.as_ref()], program_id);
    require_keys_eq!(account_info.key(), expected, GameStateError::Unauthorized);
    require!(
        account_info.data_len() >= QUEST_PROGRESS_STATE_SPACE,
        GameStateError::InvalidArgs
    );
    let progress: QuestProgressState = read_program_account(account_info, program_id)?;
    require_keys_eq!(progress.authority, authority, GameStateError::Unauthorized);
    Ok(progress)
}

fn empty_quest_progress(authority: Pubkey, now: i64) -> QuestProgressState {
    QuestProgressState {
        authority,
        daily_epoch: daily_epoch(now),
        weekly_epoch: weekly_epoch(now),
        monthly_epoch: monthly_epoch(now),
        daily_store_packs_bought: 0,
        weekly_store_packs_bought: 0,
        monthly_store_packs_bought: 0,
        daily_antimatter_spent: 0,
        weekly_antimatter_spent: 0,
        monthly_antimatter_spent: 0,
        daily_planets_colonized: 0,
        weekly_planets_colonized: 0,
        monthly_planets_colonized: 0,
        daily_attacks_resolved: 0,
        weekly_attacks_resolved: 0,
        monthly_attacks_resolved: 0,
        daily_transports_resolved: 0,
        weekly_transports_resolved: 0,
        monthly_transports_resolved: 0,
        daily_spy_missions_resolved: 0,
        weekly_spy_missions_resolved: 0,
        monthly_spy_missions_resolved: 0,
        last_updated_ts: now,
        bump: 0,
    }
}

fn empty_quest_reward_targets(authority: Pubkey, now: i64) -> QuestRewardTargetState {
    QuestRewardTargetState {
        authority,
        daily_epoch: daily_epoch(now),
        weekly_epoch: weekly_epoch(now),
        monthly_epoch: monthly_epoch(now),
        daily_store_packs_planet: Pubkey::default(),
        weekly_store_packs_planet: Pubkey::default(),
        monthly_store_packs_planet: Pubkey::default(),
        daily_antimatter_planet: Pubkey::default(),
        weekly_antimatter_planet: Pubkey::default(),
        monthly_antimatter_planet: Pubkey::default(),
        daily_colonized_planet: Pubkey::default(),
        weekly_colonized_planet: Pubkey::default(),
        monthly_colonized_planet: Pubkey::default(),
        daily_attacks_planet: Pubkey::default(),
        weekly_attacks_planet: Pubkey::default(),
        monthly_attacks_planet: Pubkey::default(),
        daily_transports_planet: Pubkey::default(),
        weekly_transports_planet: Pubkey::default(),
        monthly_transports_planet: Pubkey::default(),
        daily_spy_planet: Pubkey::default(),
        weekly_spy_planet: Pubkey::default(),
        monthly_spy_planet: Pubkey::default(),
        last_updated_ts: now,
        bump: 0,
    }
}

fn empty_tutorial_quest_reward_targets(
    authority: Pubkey,
    now: i64,
) -> TutorialQuestRewardTargetState {
    TutorialQuestRewardTargetState {
        authority,
        tutorial_quest_planets: [Pubkey::default(); 21],
        last_updated_ts: now,
        bump: 0,
    }
}

fn sync_quest_reward_target_periods(targets: &mut QuestRewardTargetState, now: i64) {
    let daily_epoch = daily_epoch(now);
    let weekly_epoch = weekly_epoch(now);
    let monthly_epoch = monthly_epoch(now);

    if targets.daily_epoch != daily_epoch {
        targets.daily_epoch = daily_epoch;
        targets.daily_store_packs_planet = Pubkey::default();
        targets.daily_antimatter_planet = Pubkey::default();
        targets.daily_colonized_planet = Pubkey::default();
        targets.daily_attacks_planet = Pubkey::default();
        targets.daily_transports_planet = Pubkey::default();
        targets.daily_spy_planet = Pubkey::default();
    }
    if targets.weekly_epoch != weekly_epoch {
        targets.weekly_epoch = weekly_epoch;
        targets.weekly_store_packs_planet = Pubkey::default();
        targets.weekly_antimatter_planet = Pubkey::default();
        targets.weekly_colonized_planet = Pubkey::default();
        targets.weekly_attacks_planet = Pubkey::default();
        targets.weekly_transports_planet = Pubkey::default();
        targets.weekly_spy_planet = Pubkey::default();
    }
    if targets.monthly_epoch != monthly_epoch {
        targets.monthly_epoch = monthly_epoch;
        targets.monthly_store_packs_planet = Pubkey::default();
        targets.monthly_antimatter_planet = Pubkey::default();
        targets.monthly_colonized_planet = Pubkey::default();
        targets.monthly_attacks_planet = Pubkey::default();
        targets.monthly_transports_planet = Pubkey::default();
        targets.monthly_spy_planet = Pubkey::default();
    }
}

fn validate_quest_reward_targets_pda(
    account_info: &AccountInfo,
    authority: Pubkey,
    program_id: &Pubkey,
) -> Result<QuestRewardTargetState> {
    let (expected, _) =
        Pubkey::find_program_address(&[b"quest_reward_targets", authority.as_ref()], program_id);
    require_keys_eq!(account_info.key(), expected, GameStateError::Unauthorized);
    require!(
        account_info.data_len() >= QUEST_REWARD_TARGET_STATE_SPACE,
        GameStateError::InvalidArgs
    );
    let targets: QuestRewardTargetState = read_program_account(account_info, program_id)?;
    require_keys_eq!(targets.authority, authority, GameStateError::Unauthorized);
    Ok(targets)
}

fn validate_tutorial_quest_reward_targets_pda(
    account_info: &AccountInfo,
    authority: Pubkey,
    program_id: &Pubkey,
) -> Result<TutorialQuestRewardTargetState> {
    let (expected, _) = Pubkey::find_program_address(
        &[b"quest_tutorial_reward_targets", authority.as_ref()],
        program_id,
    );
    require_keys_eq!(account_info.key(), expected, GameStateError::Unauthorized);
    require!(
        account_info.data_len() >= TUTORIAL_QUEST_REWARD_TARGET_STATE_SPACE,
        GameStateError::InvalidArgs
    );
    let targets: TutorialQuestRewardTargetState = read_program_account(account_info, program_id)?;
    require_keys_eq!(targets.authority, authority, GameStateError::Unauthorized);
    Ok(targets)
}

fn quest_target_slot_mut<'a>(
    targets: &'a mut QuestRewardTargetState,
    period: u8,
    metric: QuestProgressMetric,
) -> Result<&'a mut Pubkey> {
    match (period, metric) {
        (1, QuestProgressMetric::StorePacksBought) => Ok(&mut targets.daily_store_packs_planet),
        (2, QuestProgressMetric::StorePacksBought) => Ok(&mut targets.weekly_store_packs_planet),
        (3, QuestProgressMetric::StorePacksBought) => Ok(&mut targets.monthly_store_packs_planet),
        (1, QuestProgressMetric::AntimatterSpent) => Ok(&mut targets.daily_antimatter_planet),
        (2, QuestProgressMetric::AntimatterSpent) => Ok(&mut targets.weekly_antimatter_planet),
        (3, QuestProgressMetric::AntimatterSpent) => Ok(&mut targets.monthly_antimatter_planet),
        (1, QuestProgressMetric::PlanetsColonized) => Ok(&mut targets.daily_colonized_planet),
        (2, QuestProgressMetric::PlanetsColonized) => Ok(&mut targets.weekly_colonized_planet),
        (3, QuestProgressMetric::PlanetsColonized) => Ok(&mut targets.monthly_colonized_planet),
        (1, QuestProgressMetric::AttacksResolved) => Ok(&mut targets.daily_attacks_planet),
        (2, QuestProgressMetric::AttacksResolved) => Ok(&mut targets.weekly_attacks_planet),
        (3, QuestProgressMetric::AttacksResolved) => Ok(&mut targets.monthly_attacks_planet),
        (1, QuestProgressMetric::TransportsResolved) => Ok(&mut targets.daily_transports_planet),
        (2, QuestProgressMetric::TransportsResolved) => Ok(&mut targets.weekly_transports_planet),
        (3, QuestProgressMetric::TransportsResolved) => Ok(&mut targets.monthly_transports_planet),
        (1, QuestProgressMetric::SpyMissionsResolved) => Ok(&mut targets.daily_spy_planet),
        (2, QuestProgressMetric::SpyMissionsResolved) => Ok(&mut targets.weekly_spy_planet),
        (3, QuestProgressMetric::SpyMissionsResolved) => Ok(&mut targets.monthly_spy_planet),
        _ => err!(GameStateError::InvalidQuest),
    }
}

fn tutorial_quest_target_slot_mut<'a>(
    targets: &'a mut TutorialQuestRewardTargetState,
    quest_id: u8,
) -> Result<&'a mut Pubkey> {
    match quest_id {
        0..=20 => Ok(&mut targets.tutorial_quest_planets[quest_id as usize]),
        _ => err!(GameStateError::InvalidQuest),
    }
}

fn validate_or_lock_tutorial_quest_reward_target(
    tutorial_targets_info: Option<&AccountInfo>,
    authority: Pubkey,
    claiming_planet: Pubkey,
    program_id: Pubkey,
    now: i64,
    quest_id: u8,
) -> Result<()> {
    let Some(info) = tutorial_targets_info else {
        return err!(GameStateError::InvalidArgs);
    };
    let mut targets = validate_tutorial_quest_reward_targets_pda(info, authority, &program_id)?;
    let slot = tutorial_quest_target_slot_mut(&mut targets, quest_id)?;
    if *slot == Pubkey::default() {
        *slot = claiming_planet;
        targets.last_updated_ts = now;
        write_program_account(info, &targets)?;
        return Ok(());
    }
    require_keys_eq!(*slot, claiming_planet, GameStateError::WrongQuestRewardPlanet);
    Ok(())
}

fn recurring_requirement_metric(req: QuestRequirement) -> QuestProgressMetric {
    match req {
        QuestRequirement::MetalMine(_)
        | QuestRequirement::CrystalMine(_)
        | QuestRequirement::MetalStorage(_)
        | QuestRequirement::CrystalStorage(_)
        | QuestRequirement::DeuteriumTank(_) => QuestProgressMetric::StorePacksBought,
        QuestRequirement::SolarPlant(_)
        | QuestRequirement::SmallCargo(_)
        | QuestRequirement::LargeCargo(_) => QuestProgressMetric::TransportsResolved,
        QuestRequirement::ResearchLab(_)
        | QuestRequirement::ComputerTech(_)
        | QuestRequirement::EspionageProbe(_)
        | QuestRequirement::IgrNetwork(_) => QuestProgressMetric::SpyMissionsResolved,
        QuestRequirement::Astrophysics(_) | QuestRequirement::ColonyShip(_) => {
            QuestProgressMetric::PlanetsColonized
        }
        QuestRequirement::EnergyTech(_)
        | QuestRequirement::DeuteriumSynthesizer(_)
        | QuestRequirement::FusionReactor(_)
        | QuestRequirement::RoboticsFactory(_)
        | QuestRequirement::NaniteFactory(_)
        | QuestRequirement::Shipyard(_)
        | QuestRequirement::CombustionDrive(_)
        | QuestRequirement::ImpulseDrive(_)
        | QuestRequirement::HyperspaceDrive(_)
        | QuestRequirement::WeaponsTechnology(_)
        | QuestRequirement::ShieldingTechnology(_)
        | QuestRequirement::ArmorTechnology(_) => QuestProgressMetric::AntimatterSpent,
        _ => QuestProgressMetric::AttacksResolved,
    }
}

fn period_has_metric_quest(period: u8, epoch: i64, metric: QuestProgressMetric) -> Result<bool> {
    for slot in 0u8..12 {
        let quest_id = if period == 1 { slot + 1 } else { slot };
        let quest = rotating_quest(period, quest_id, epoch)?;
        if recurring_requirement_metric(quest.req) == metric {
            return Ok(true);
        }
    }
    Ok(false)
}

fn lock_matching_quest_reward_targets(
    targets: &mut QuestRewardTargetState,
    now: i64,
    metric: QuestProgressMetric,
    planet_key: Pubkey,
) -> Result<()> {
    let periods = [
        (1u8, daily_epoch(now)),
        (2u8, weekly_epoch(now)),
        (3u8, monthly_epoch(now)),
    ];
    for (period, epoch) in periods {
        if !period_has_metric_quest(period, epoch, metric)? {
            continue;
        }
        let slot = quest_target_slot_mut(targets, period, metric)?;
        if *slot == Pubkey::default() {
            *slot = planet_key;
        }
    }
    Ok(())
}

fn lock_quest_reward_target(
    targets_info: Option<&AccountInfo>,
    authority: Pubkey,
    program_id: &Pubkey,
    now: i64,
    metric: QuestProgressMetric,
    planet_key: Pubkey,
) -> Result<()> {
    let Some(info) = targets_info else {
        return err!(GameStateError::InvalidArgs);
    };
    let mut targets = validate_quest_reward_targets_pda(info, authority, program_id)?;
    sync_quest_reward_target_periods(&mut targets, now);
    lock_matching_quest_reward_targets(&mut targets, now, metric, planet_key)?;
    targets.last_updated_ts = now;
    write_program_account(info, &targets)
}

fn increment_quest_progress(
    progress_info: Option<&AccountInfo>,
    targets_info: Option<&AccountInfo>,
    authority: Pubkey,
    program_id: &Pubkey,
    now: i64,
    metric: QuestProgressMetric,
    amount: u64,
    source_planet: Option<Pubkey>,
) -> Result<()> {
    let Some(info) = progress_info else {
        return Ok(());
    };
    let mut progress = validate_quest_progress_pda(info, authority, program_id)?;
    sync_quest_progress_periods(&mut progress, now);
    match metric {
        QuestProgressMetric::StorePacksBought => {
            let amount = amount.min(u32::MAX as u64) as u32;
            progress.daily_store_packs_bought =
                progress.daily_store_packs_bought.saturating_add(amount);
            progress.weekly_store_packs_bought =
                progress.weekly_store_packs_bought.saturating_add(amount);
            progress.monthly_store_packs_bought =
                progress.monthly_store_packs_bought.saturating_add(amount);
        }
        QuestProgressMetric::AntimatterSpent => {
            progress.daily_antimatter_spent =
                progress.daily_antimatter_spent.saturating_add(amount);
            progress.weekly_antimatter_spent =
                progress.weekly_antimatter_spent.saturating_add(amount);
            progress.monthly_antimatter_spent =
                progress.monthly_antimatter_spent.saturating_add(amount);
        }
        QuestProgressMetric::PlanetsColonized => {
            let amount = amount.min(u32::MAX as u64) as u32;
            progress.daily_planets_colonized =
                progress.daily_planets_colonized.saturating_add(amount);
            progress.weekly_planets_colonized =
                progress.weekly_planets_colonized.saturating_add(amount);
            progress.monthly_planets_colonized =
                progress.monthly_planets_colonized.saturating_add(amount);
        }
        QuestProgressMetric::AttacksResolved => {
            let amount = amount.min(u32::MAX as u64) as u32;
            progress.daily_attacks_resolved =
                progress.daily_attacks_resolved.saturating_add(amount);
            progress.weekly_attacks_resolved =
                progress.weekly_attacks_resolved.saturating_add(amount);
            progress.monthly_attacks_resolved =
                progress.monthly_attacks_resolved.saturating_add(amount);
        }
        QuestProgressMetric::TransportsResolved => {
            let amount = amount.min(u32::MAX as u64) as u32;
            progress.daily_transports_resolved =
                progress.daily_transports_resolved.saturating_add(amount);
            progress.weekly_transports_resolved =
                progress.weekly_transports_resolved.saturating_add(amount);
            progress.monthly_transports_resolved =
                progress.monthly_transports_resolved.saturating_add(amount);
        }
        QuestProgressMetric::SpyMissionsResolved => {
            let amount = amount.min(u32::MAX as u64) as u32;
            progress.daily_spy_missions_resolved =
                progress.daily_spy_missions_resolved.saturating_add(amount);
            progress.weekly_spy_missions_resolved =
                progress.weekly_spy_missions_resolved.saturating_add(amount);
            progress.monthly_spy_missions_resolved = progress
                .monthly_spy_missions_resolved
                .saturating_add(amount);
        }
    }
    progress.last_updated_ts = now;
    write_program_account(info, &progress)?;
    if let Some(planet_key) = source_planet {
        lock_quest_reward_target(targets_info, authority, program_id, now, metric, planet_key)?;
    }
    Ok(())
}

fn claim_daily_check_in(
    quest: &mut Account<QuestState>,
    planet: &mut Account<PlanetState>,
    now: i64,
) -> Result<()> {
    sync_quest_periods(quest, now);
    let day = daily_epoch(now);
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
    progress: &mut QuestProgressState,
    targets_info: Option<&AccountInfo>,
    tutorial_targets_info: Option<&AccountInfo>,
    program_id: &Pubkey,
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
    if period == 0 {
        require!(
            quest_requirement_met(period, quest_id, epoch, planet),
            GameStateError::QuestRequirementsNotMet
        );
        validate_or_lock_tutorial_quest_reward_target(
            tutorial_targets_info,
            quest.authority,
            planet.to_account_info().key(),
            *program_id,
            now,
            quest_id,
        )?;
    } else {
        sync_quest_progress_periods(progress, now);
        require!(
            recurring_quest_requirement_met(period, quest_id, epoch, &progress)?,
            GameStateError::QuestRequirementsNotMet
        );
        validate_or_lock_quest_reward_target(
            targets_info,
            quest.authority,
            planet.to_account_info().key(),
            *program_id,
            now,
            period,
            quest_id,
            epoch,
        )?;
    }

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

fn claim_daily_check_in_live(
    quest: &mut Account<QuestState>,
    planet: &mut PlanetDepositFields,
    now: i64,
) -> Result<()> {
    sync_quest_periods(quest, now);
    let day = daily_epoch(now);
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
    award_resources_live(
        planet,
        now,
        500u64.saturating_add(streak_bonus),
        300u64.saturating_add(streak_bonus / 2),
        100u64.saturating_add(streak_bonus / 5),
    )
}

fn claim_quest_reward_live(
    quest: &mut Account<QuestState>,
    planet: &mut PlanetQuestFields,
    progress: &mut QuestProgressState,
    targets_info: Option<&AccountInfo>,
    tutorial_targets_info: Option<&AccountInfo>,
    planet_key: Pubkey,
    authority: Pubkey,
    program_id: &Pubkey,
    period: u8,
    quest_id: u8,
    now: i64,
) -> Result<()> {
    if period == 1 && quest_id == 0 {
        return claim_daily_check_in_live(quest, &mut planet.deposit, now);
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
    if period == 0 {
        require!(
            quest_requirement_met_live(period, quest_id, epoch, planet),
            GameStateError::QuestRequirementsNotMet
        );
        validate_or_lock_tutorial_quest_reward_target(
            tutorial_targets_info,
            authority,
            planet_key,
            *program_id,
            now,
            quest_id,
        )?;
    } else {
        sync_quest_progress_periods(progress, now);
        require!(
            recurring_quest_requirement_met(period, quest_id, epoch, &progress)?,
            GameStateError::QuestRequirementsNotMet
        );
        validate_or_lock_quest_reward_target(
            targets_info,
            authority,
            planet_key,
            *program_id,
            now,
            period,
            quest_id,
            epoch,
        )?;
    }

    let (metal, crystal, deuterium) = quest_reward(period, quest_id, epoch)?;
    match period {
        0 => quest.tutorial_claimed_mask |= bit,
        1 => quest.daily_claimed_mask |= bit,
        2 => quest.weekly_claimed_mask |= bit,
        3 => quest.monthly_claimed_mask |= bit,
        _ => unreachable!(),
    }
    quest.last_updated_ts = now;

    award_resources_live(&mut planet.deposit, now, metal, crystal, deuterium)
}

fn award_resources_live(
    planet: &mut PlanetDepositFields,
    now: i64,
    metal: u64,
    crystal: u64,
    deuterium: u64,
) -> Result<()> {
    settle_planet_deposit_fields(planet, now)?;
    planet.metal = planet
        .metal
        .checked_add(metal)
        .filter(|amount| *amount <= planet.metal_cap)
        .ok_or(GameStateError::ResourceCapExceeded)?;
    planet.crystal = planet
        .crystal
        .checked_add(crystal)
        .filter(|amount| *amount <= planet.crystal_cap)
        .ok_or(GameStateError::ResourceCapExceeded)?;
    planet.deuterium = planet
        .deuterium
        .checked_add(deuterium)
        .filter(|amount| *amount <= planet.deuterium_cap)
        .ok_or(GameStateError::ResourceCapExceeded)?;
    Ok(())
}

fn ensure_resource_room_live(
    planet: &PlanetDepositFields,
    metal: u64,
    crystal: u64,
    deuterium: u64,
) -> Result<()> {
    planet
        .metal
        .checked_add(metal)
        .filter(|amount| *amount <= planet.metal_cap)
        .ok_or(GameStateError::ResourceCapExceeded)?;
    planet
        .crystal
        .checked_add(crystal)
        .filter(|amount| *amount <= planet.crystal_cap)
        .ok_or(GameStateError::ResourceCapExceeded)?;
    planet
        .deuterium
        .checked_add(deuterium)
        .filter(|amount| *amount <= planet.deuterium_cap)
        .ok_or(GameStateError::ResourceCapExceeded)?;
    Ok(())
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

fn ensure_program_account_owner(account_info: &AccountInfo) -> Result<()> {
    require_keys_eq!(
        *account_info.owner,
        crate::ID,
        GameStateError::Unauthorized
    );
    Ok(())
}

pub fn rotate_store_config_admin(
    ctx: Context<RotateStoreConfigAdmin>,
    new_admin: Pubkey,
) -> Result<()> {
    require_keys_eq!(new_admin, ctx.accounts.new_admin.key(), GameStateError::InvalidArgs);
    require!(new_admin != Pubkey::default(), GameStateError::InvalidArgs);
    require!(new_admin != ctx.accounts.admin.key(), GameStateError::InvalidArgs);
    require_keys_eq!(
        ctx.accounts.old_antimatter_treasury.key(),
        get_associated_token_address(&ctx.accounts.admin.key(), &PROTOCOL_ANTIMATTER_MINT),
        GameStateError::InvalidAntimatterAccount
    );
    require!(ctx.accounts.old_antimatter_treasury.amount == 0, GameStateError::InvalidArgs);
    require_keys_eq!(
        ctx.accounts.old_antimatter_treasury.mint,
        PROTOCOL_ANTIMATTER_MINT,
        GameStateError::InvalidAntimatterMint
    );
    ctx.accounts.store_config.admin = new_admin;
    Ok(())
}

fn sync_store_periods(store: &mut Account<StorePurchaseState>, now: i64) {
    let daily_epoch = daily_epoch(now);
    let weekly_epoch = weekly_epoch(now);
    let monthly_epoch = monthly_epoch(now);

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
            crystal: 0,
            deuterium: 0,
            shield_seconds: 0,
        }),
        (1, 1) => Ok(StorePack {
            price_usdc: 1_000_000,
            metal: 0,
            crystal: 2_000,
            deuterium: 0,
            shield_seconds: 0,
        }),
        (1, 2) => Ok(StorePack {
            price_usdc: 1_000_000,
            metal: 0,
            crystal: 0,
            deuterium: 750,
            shield_seconds: 0,
        }),
        (1, 3) => Ok(StorePack {
            price_usdc: 3_000_000,
            metal: 3_000,
            crystal: 2_000,
            deuterium: 750,
            shield_seconds: 0,
        }),
        (1, 4) => Ok(StorePack {
            price_usdc: 5_000_000,
            metal: 8_000,
            crystal: 5_000,
            deuterium: 2_000,
            shield_seconds: 0,
        }),
        (1, 5) => Ok(StorePack {
            price_usdc: 7_500_000,
            metal: 14_000,
            crystal: 9_000,
            deuterium: 3_500,
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
            price_usdc: 5_000_000,
            metal: 35_000,
            crystal: 0,
            deuterium: 0,
            shield_seconds: 0,
        }),
        (2, 1) => Ok(StorePack {
            price_usdc: 5_000_000,
            metal: 0,
            crystal: 24_000,
            deuterium: 0,
            shield_seconds: 0,
        }),
        (2, 2) => Ok(StorePack {
            price_usdc: 5_000_000,
            metal: 0,
            crystal: 0,
            deuterium: 10_000,
            shield_seconds: 0,
        }),
        (2, 3) => Ok(StorePack {
            price_usdc: 15_000_000,
            metal: 35_000,
            crystal: 24_000,
            deuterium: 10_000,
            shield_seconds: 0,
        }),
        (2, 4) => Ok(StorePack {
            price_usdc: 30_000_000,
            metal: 80_000,
            crystal: 55_000,
            deuterium: 25_000,
            shield_seconds: 0,
        }),
        (2, 5) => Ok(StorePack {
            price_usdc: 45_000_000,
            metal: 140_000,
            crystal: 90_000,
            deuterium: 40_000,
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
            price_usdc: 20_000_000,
            metal: 180_000,
            crystal: 0,
            deuterium: 0,
            shield_seconds: 0,
        }),
        (3, 1) => Ok(StorePack {
            price_usdc: 20_000_000,
            metal: 0,
            crystal: 125_000,
            deuterium: 0,
            shield_seconds: 0,
        }),
        (3, 2) => Ok(StorePack {
            price_usdc: 20_000_000,
            metal: 0,
            crystal: 0,
            deuterium: 60_000,
            shield_seconds: 0,
        }),
        (3, 3) => Ok(StorePack {
            price_usdc: 60_000_000,
            metal: 180_000,
            crystal: 125_000,
            deuterium: 60_000,
            shield_seconds: 0,
        }),
        (3, 4) => Ok(StorePack {
            price_usdc: 100_000_000,
            metal: 400_000,
            crystal: 275_000,
            deuterium: 140_000,
            shield_seconds: 0,
        }),
        (3, 5) => Ok(StorePack {
            price_usdc: 150_000_000,
            metal: 700_000,
            crystal: 450_000,
            deuterium: 220_000,
            shield_seconds: 0,
        }),
        (3, 6) => Ok(StorePack {
            price_usdc: 200_000_000,
            metal: 1_000_000,
            crystal: 650_000,
            deuterium: 320_000,
            shield_seconds: 0,
        }),
        (3, 7) => Ok(StorePack {
            price_usdc: 250_000_000,
            metal: 1_350_000,
            crystal: 900_000,
            deuterium: 450_000,
            shield_seconds: 0,
        }),
        (3, 8) => Ok(StorePack {
            price_usdc: 300_000_000,
            metal: 1_750_000,
            crystal: 1_150_000,
            deuterium: 600_000,
            shield_seconds: 0,
        }),
        (3, 9) => Ok(StorePack {
            price_usdc: 400_000_000,
            metal: 2_500_000,
            crystal: 1_700_000,
            deuterium: 850_000,
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
        purchase_state.daily_epoch = daily_epoch(now);
        purchase_state.weekly_epoch = weekly_epoch(now);
        purchase_state.monthly_epoch = monthly_epoch(now);
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
    require!(pack.shield_seconds == 0, GameStateError::InvalidStorePack);
    let planet_info = ctx.accounts.planet_state.to_account_info();
    let mut planet = read_planet_deposit_fields(&planet_info, ctx.program_id)?;
    require_keys_eq!(
        planet.authority,
        ctx.accounts.authority.key(),
        GameStateError::Unauthorized
    );
    settle_planet_deposit_fields(&mut planet, now)?;
    ensure_resource_room_live(&planet, pack.metal, pack.crystal, pack.deuterium)?;
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

    award_resources_live(&mut planet, now, pack.metal, pack.crystal, pack.deuterium)?;
    increment_quest_progress(
        ctx.remaining_accounts.first(),
        ctx.remaining_accounts.get(1),
        ctx.accounts.authority.key(),
        ctx.program_id,
        now,
        QuestProgressMetric::StorePacksBought,
        1,
        Some(planet_info.key()),
    )?;
    write_planet_deposit_fields(&planet_info, &planet)
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
struct AllianceDepositMissionEntry {
    metal: u64,
    crystal: u64,
    deuterium: u64,
    antimatter: u64,
    xp: u64,
}

fn alliance_deposit_xp(
    mission: &AllianceDepositMissionEntry,
    metal: u64,
    crystal: u64,
    deuterium: u64,
    antimatter: u64,
) -> u64 {
    let resource_xp = metal.saturating_add(crystal).saturating_add(deuterium)
        / ALLIANCE_DEPOSIT_XP_PER_RESOURCE_UNIT;
    let antimatter_xp = antimatter / ALLIANCE_DEPOSIT_XP_PER_ANTIMATTER_UNIT;
    let variable_cap = mission
        .xp
        .saturating_mul(ALLIANCE_DEPOSIT_VARIABLE_XP_MULTIPLIER);
    mission
        .xp
        .saturating_add(resource_xp.saturating_add(antimatter_xp).min(variable_cap))
}

const DAILY_ALLIANCE_DEPOSIT_MISSIONS: [AllianceDepositMissionEntry; 4] = [
    AllianceDepositMissionEntry {
        metal: 1_000,
        crystal: 0,
        deuterium: 0,
        antimatter: 0,
        xp: 80,
    },
    AllianceDepositMissionEntry {
        metal: 0,
        crystal: 1_000,
        deuterium: 0,
        antimatter: 0,
        xp: 80,
    },
    AllianceDepositMissionEntry {
        metal: 0,
        crystal: 0,
        deuterium: 500,
        antimatter: 0,
        xp: 90,
    },
    AllianceDepositMissionEntry {
        metal: 0,
        crystal: 0,
        deuterium: 0,
        antimatter: 100 * ANTIMATTER_SCALE,
        xp: 150,
    },
];

const WEEKLY_ALLIANCE_DEPOSIT_MISSIONS: [AllianceDepositMissionEntry; 4] = [
    AllianceDepositMissionEntry {
        metal: 10_000,
        crystal: 6_000,
        deuterium: 0,
        antimatter: 0,
        xp: 420,
    },
    AllianceDepositMissionEntry {
        metal: 0,
        crystal: 8_000,
        deuterium: 3_000,
        antimatter: 0,
        xp: 440,
    },
    AllianceDepositMissionEntry {
        metal: 6_000,
        crystal: 0,
        deuterium: 5_000,
        antimatter: 0,
        xp: 450,
    },
    AllianceDepositMissionEntry {
        metal: 0,
        crystal: 0,
        deuterium: 0,
        antimatter: 1_000 * ANTIMATTER_SCALE,
        xp: 900,
    },
];

const MONTHLY_ALLIANCE_DEPOSIT_MISSIONS: [AllianceDepositMissionEntry; 4] = [
    AllianceDepositMissionEntry {
        metal: 60_000,
        crystal: 40_000,
        deuterium: 20_000,
        antimatter: 0,
        xp: 1_800,
    },
    AllianceDepositMissionEntry {
        metal: 120_000,
        crystal: 0,
        deuterium: 0,
        antimatter: 0,
        xp: 1_400,
    },
    AllianceDepositMissionEntry {
        metal: 0,
        crystal: 90_000,
        deuterium: 35_000,
        antimatter: 0,
        xp: 1_650,
    },
    AllianceDepositMissionEntry {
        metal: 0,
        crystal: 0,
        deuterium: 0,
        antimatter: 5_000 * ANTIMATTER_SCALE,
        xp: 3_500,
    },
];

fn alliance_deposit_mission(period: u8, mission_id: u8) -> Result<AllianceDepositMissionEntry> {
    let catalog = match period {
        1 => &DAILY_ALLIANCE_DEPOSIT_MISSIONS[..],
        2 => &WEEKLY_ALLIANCE_DEPOSIT_MISSIONS[..],
        3 => &MONTHLY_ALLIANCE_DEPOSIT_MISSIONS[..],
        _ => return err!(GameStateError::InvalidAllianceMission),
    };
    catalog
        .get(mission_id as usize)
        .copied()
        .ok_or_else(|| GameStateError::InvalidAllianceMission.into())
}

fn alliance_building_level(treasury: &AllianceTreasuryState, building_id: u8) -> Result<u8> {
    match building_id {
        0 => Ok(treasury.logistics_hub),
        1 => Ok(treasury.research_grid),
        2 => Ok(treasury.defense_coordination),
        3 => Ok(treasury.trade_network),
        _ => err!(GameStateError::InvalidAllianceBuilding),
    }
}

fn set_alliance_building_level(
    treasury: &mut AllianceTreasuryState,
    building_id: u8,
    level: u8,
) -> Result<()> {
    match building_id {
        0 => treasury.logistics_hub = level,
        1 => treasury.research_grid = level,
        2 => treasury.defense_coordination = level,
        3 => treasury.trade_network = level,
        _ => return err!(GameStateError::InvalidAllianceBuilding),
    }
    Ok(())
}

fn apply_bps_bonus(value: u64, bps: u64) -> u64 {
    value.saturating_add(value.saturating_mul(bps) / 10_000)
}

fn apply_bps_discount(value: u64, bps: u64) -> u64 {
    if value == 0 || bps == 0 {
        return value;
    }
    let discount = value.saturating_mul(bps.min(9_000)) / 10_000;
    value.saturating_sub(discount).max(1)
}

fn alliance_logistics_xp_bonus_bps(treasury: &AllianceTreasuryState) -> u64 {
    treasury.logistics_hub as u64 * 500
}

fn alliance_research_grid_xp_bonus_bps(treasury: &AllianceTreasuryState) -> u64 {
    treasury.research_grid as u64 * 500
}

fn alliance_trade_cost_discount_bps(treasury: &AllianceTreasuryState) -> u64 {
    treasury.trade_network as u64 * ALLIANCE_TRADE_DISCOUNT_BPS_PER_LEVEL
}

fn alliance_defense_deuterium_discount_bps(treasury: &AllianceTreasuryState) -> u64 {
    treasury.defense_coordination as u64 * ALLIANCE_DEFENSE_DISCOUNT_BPS_PER_LEVEL
}

fn apply_alliance_upgrade_discounts(
    metal: u64,
    crystal: u64,
    deuterium: u64,
    antimatter: u64,
    treasury: &AllianceTreasuryState,
) -> (u64, u64, u64, u64) {
    let trade_bps = alliance_trade_cost_discount_bps(treasury);
    let defense_bps = alliance_defense_deuterium_discount_bps(treasury);
    (
        apply_bps_discount(metal, trade_bps),
        apply_bps_discount(crystal, trade_bps),
        apply_bps_discount(deuterium, defense_bps),
        apply_bps_discount(antimatter, trade_bps),
    )
}

fn alliance_building_cost(building_id: u8, next_level: u8) -> Result<(u64, u64, u64, u64, u64)> {
    let level = next_level as u64;
    let scale = level.saturating_mul(level);
    match building_id {
        0 => Ok((
            5_000u64.saturating_mul(scale),
            2_000u64.saturating_mul(scale),
            1_000u64.saturating_mul(scale),
            50u64.saturating_mul(level).saturating_mul(ANTIMATTER_SCALE),
            250u64.saturating_mul(level),
        )),
        1 => Ok((
            2_000u64.saturating_mul(scale),
            5_000u64.saturating_mul(scale),
            2_000u64.saturating_mul(scale),
            75u64.saturating_mul(level).saturating_mul(ANTIMATTER_SCALE),
            300u64.saturating_mul(level),
        )),
        2 => Ok((
            4_000u64.saturating_mul(scale),
            4_000u64.saturating_mul(scale),
            1_500u64.saturating_mul(scale),
            60u64.saturating_mul(level).saturating_mul(ANTIMATTER_SCALE),
            275u64.saturating_mul(level),
        )),
        3 => Ok((
            3_000u64.saturating_mul(scale),
            3_000u64.saturating_mul(scale),
            3_000u64.saturating_mul(scale),
            100u64
                .saturating_mul(level)
                .saturating_mul(ANTIMATTER_SCALE),
            350u64.saturating_mul(level),
        )),
        _ => err!(GameStateError::InvalidAllianceBuilding),
    }
}

const DAILY_ROTATING_QUESTS: [QuestCatalogEntry; 25] = [
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
        req: QuestRequirement::LargeCargo(1),
        metal: 1_200,
        crystal: 900,
        deuterium: 250,
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
        req: QuestRequirement::ColonyShip(1),
        metal: 1_400,
        crystal: 1_200,
        deuterium: 400,
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

fn gcd_u64(mut a: u64, mut b: u64) -> u64 {
    while b != 0 {
        let next = a % b;
        a = b;
        b = next;
    }
    a
}

fn coprime_quest_step(seed: u64, len: u64) -> u64 {
    let mut step = ((seed >> 32) % (len - 1)).saturating_add(1);
    while gcd_u64(step, len) != 1 {
        step = if step + 1 >= len { 1 } else { step + 1 };
    }
    step
}

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
    require!(len >= 12, GameStateError::InvalidQuest);
    let seed = (epoch as u64)
        .wrapping_mul(0x9E37_79B9_7F4A_7C15)
        .wrapping_add((period as u64).wrapping_mul(0xBF58_476D_1CE4_E5B9));
    let offset = seed % len;
    let step = coprime_quest_step(seed, len);
    let mut seen = [0u64; 23];
    let mut seen_len = 0usize;
    let mut selected = 0u64;
    for i in 0..len {
        let idx = ((offset + i.wrapping_mul(step)) % len) as usize;
        let quest = catalog
            .get(idx)
            .copied()
            .ok_or(GameStateError::InvalidQuest)?;
        let signature = recurring_requirement_signature(period, quest.req);
        let mut duplicate = false;
        for existing in seen.iter().take(seen_len) {
            if *existing == signature {
                duplicate = true;
                break;
            }
        }
        if duplicate {
            continue;
        }
        if selected == slot {
            return Ok(quest);
        }
        require!(seen_len < seen.len(), GameStateError::InvalidQuest);
        seen[seen_len] = signature;
        seen_len += 1;
        selected += 1;
    }
    Err(GameStateError::InvalidQuest.into())
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

fn requirement_met_live(req: QuestRequirement, planet: &PlanetQuestFields) -> bool {
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
        QuestRequirement::Ships(v) => u64::from(total_ships_live(planet)) >= v,
        QuestRequirement::Defenses(v) => u64::from(total_defenses_live(planet)) >= v,
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

fn quest_requirement_met_live(
    period: u8,
    quest_id: u8,
    epoch: i64,
    planet: &PlanetQuestFields,
) -> bool {
    if period != 0 {
        return rotating_quest(period, quest_id, epoch)
            .map(|quest| requirement_met_live(quest.req, planet))
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
        (0, 11) => total_ships_live(planet) >= 1,
        (0, 12) => planet.small_cargo >= 1,
        (0, 13) => planet.rocket_launcher >= 1,
        (0, 14) => planet.computer_tech >= 1,
        (0, 15) => planet.espionage_probe >= 1,
        (0, 16) => planet.light_laser >= 1,
        (0, 17) => planet.impulse_drive >= 1,
        (0, 18) => planet.astrophysics >= 1,
        (0, 19) => total_defenses_live(planet) >= 5,
        (0, 20) => planet.colony_ship >= 1,
        _ => false,
    }
}

fn progress_u32_for_period(period: u8, daily: u32, weekly: u32, monthly: u32) -> Result<u32> {
    match period {
        1 => Ok(daily),
        2 => Ok(weekly),
        3 => Ok(monthly),
        _ => err!(GameStateError::InvalidQuest),
    }
}

fn progress_u64_for_period(period: u8, daily: u64, weekly: u64, monthly: u64) -> Result<u64> {
    match period {
        1 => Ok(daily),
        2 => Ok(weekly),
        3 => Ok(monthly),
        _ => err!(GameStateError::InvalidQuest),
    }
}

fn recurring_requirement_current(
    period: u8,
    req: QuestRequirement,
    progress: &QuestProgressState,
) -> Result<u64> {
    match req {
        QuestRequirement::MetalMine(_)
        | QuestRequirement::CrystalMine(_)
        | QuestRequirement::MetalStorage(_)
        | QuestRequirement::CrystalStorage(_)
        | QuestRequirement::DeuteriumTank(_) => progress_u32_for_period(
            period,
            progress.daily_store_packs_bought,
            progress.weekly_store_packs_bought,
            progress.monthly_store_packs_bought,
        )
        .map(u64::from),
        QuestRequirement::SolarPlant(_)
        | QuestRequirement::SmallCargo(_)
        | QuestRequirement::LargeCargo(_) => progress_u32_for_period(
            period,
            progress.daily_transports_resolved,
            progress.weekly_transports_resolved,
            progress.monthly_transports_resolved,
        )
        .map(u64::from),
        QuestRequirement::ResearchLab(_)
        | QuestRequirement::ComputerTech(_)
        | QuestRequirement::EspionageProbe(_)
        | QuestRequirement::IgrNetwork(_) => progress_u32_for_period(
            period,
            progress.daily_spy_missions_resolved,
            progress.weekly_spy_missions_resolved,
            progress.monthly_spy_missions_resolved,
        )
        .map(u64::from),
        QuestRequirement::Astrophysics(_) | QuestRequirement::ColonyShip(_) => {
            progress_u32_for_period(
                period,
                progress.daily_planets_colonized,
                progress.weekly_planets_colonized,
                progress.monthly_planets_colonized,
            )
            .map(u64::from)
        }
        QuestRequirement::EnergyTech(_)
        | QuestRequirement::DeuteriumSynthesizer(_)
        | QuestRequirement::FusionReactor(_)
        | QuestRequirement::RoboticsFactory(_)
        | QuestRequirement::NaniteFactory(_)
        | QuestRequirement::Shipyard(_)
        | QuestRequirement::CombustionDrive(_)
        | QuestRequirement::ImpulseDrive(_)
        | QuestRequirement::HyperspaceDrive(_)
        | QuestRequirement::WeaponsTechnology(_)
        | QuestRequirement::ShieldingTechnology(_)
        | QuestRequirement::ArmorTechnology(_) => progress_u64_for_period(
            period,
            progress.daily_antimatter_spent,
            progress.weekly_antimatter_spent,
            progress.monthly_antimatter_spent,
        ),
        QuestRequirement::Ships(_)
        | QuestRequirement::Defenses(_)
        | QuestRequirement::LightFighter(_)
        | QuestRequirement::HeavyFighter(_)
        | QuestRequirement::Cruiser(_)
        | QuestRequirement::Battleship(_)
        | QuestRequirement::Recycler(_)
        | QuestRequirement::RocketLauncher(_)
        | QuestRequirement::LightLaser(_)
        | QuestRequirement::HeavyLaser(_)
        | QuestRequirement::GaussCannon(_)
        | QuestRequirement::IonCannon(_)
        | QuestRequirement::PlasmaTurret(_) => progress_u32_for_period(
            period,
            progress.daily_attacks_resolved,
            progress.weekly_attacks_resolved,
            progress.monthly_attacks_resolved,
        )
        .map(u64::from),
    }
}

fn recurring_requirement_required(period: u8, req: QuestRequirement) -> u64 {
    let raw = match req {
        QuestRequirement::MetalMine(v)
        | QuestRequirement::CrystalMine(v)
        | QuestRequirement::DeuteriumSynthesizer(v)
        | QuestRequirement::SolarPlant(v)
        | QuestRequirement::FusionReactor(v)
        | QuestRequirement::RoboticsFactory(v)
        | QuestRequirement::NaniteFactory(v)
        | QuestRequirement::Shipyard(v)
        | QuestRequirement::ResearchLab(v)
        | QuestRequirement::MetalStorage(v)
        | QuestRequirement::CrystalStorage(v)
        | QuestRequirement::DeuteriumTank(v)
        | QuestRequirement::EnergyTech(v)
        | QuestRequirement::CombustionDrive(v)
        | QuestRequirement::ImpulseDrive(v)
        | QuestRequirement::HyperspaceDrive(v)
        | QuestRequirement::ComputerTech(v)
        | QuestRequirement::Astrophysics(v)
        | QuestRequirement::IgrNetwork(v)
        | QuestRequirement::WeaponsTechnology(v)
        | QuestRequirement::ShieldingTechnology(v)
        | QuestRequirement::ArmorTechnology(v) => v as u64,
        QuestRequirement::SmallCargo(v)
        | QuestRequirement::LargeCargo(v)
        | QuestRequirement::LightFighter(v)
        | QuestRequirement::HeavyFighter(v)
        | QuestRequirement::Cruiser(v)
        | QuestRequirement::Battleship(v)
        | QuestRequirement::Recycler(v)
        | QuestRequirement::EspionageProbe(v)
        | QuestRequirement::ColonyShip(v)
        | QuestRequirement::RocketLauncher(v)
        | QuestRequirement::LightLaser(v)
        | QuestRequirement::HeavyLaser(v)
        | QuestRequirement::GaussCannon(v)
        | QuestRequirement::IonCannon(v)
        | QuestRequirement::PlasmaTurret(v) => v as u64,
        QuestRequirement::Ships(v) | QuestRequirement::Defenses(v) => v,
    };
    match req {
        QuestRequirement::EnergyTech(_)
        | QuestRequirement::DeuteriumSynthesizer(_)
        | QuestRequirement::FusionReactor(_)
        | QuestRequirement::RoboticsFactory(_)
        | QuestRequirement::NaniteFactory(_)
        | QuestRequirement::Shipyard(_)
        | QuestRequirement::CombustionDrive(_)
        | QuestRequirement::ImpulseDrive(_)
        | QuestRequirement::HyperspaceDrive(_)
        | QuestRequirement::WeaponsTechnology(_)
        | QuestRequirement::ShieldingTechnology(_)
        | QuestRequirement::ArmorTechnology(_) => {
            let base = match period {
                1 => 50,
                2 => 250,
                3 => 1_000,
                _ => 0,
            };
            raw.max(1)
                .saturating_mul(base)
                .saturating_mul(ANTIMATTER_SCALE)
        }
        QuestRequirement::Astrophysics(_) | QuestRequirement::ColonyShip(_) => raw.clamp(1, 3),
        _ => raw.max(1),
    }
}

fn recurring_requirement_signature(period: u8, req: QuestRequirement) -> u64 {
    let category = match req {
        QuestRequirement::MetalMine(_)
        | QuestRequirement::CrystalMine(_)
        | QuestRequirement::MetalStorage(_)
        | QuestRequirement::CrystalStorage(_)
        | QuestRequirement::DeuteriumTank(_) => 1,
        QuestRequirement::SolarPlant(_)
        | QuestRequirement::SmallCargo(_)
        | QuestRequirement::LargeCargo(_) => 2,
        QuestRequirement::ResearchLab(_)
        | QuestRequirement::ComputerTech(_)
        | QuestRequirement::EspionageProbe(_)
        | QuestRequirement::IgrNetwork(_) => 3,
        QuestRequirement::Astrophysics(_) | QuestRequirement::ColonyShip(_) => 4,
        QuestRequirement::EnergyTech(_)
        | QuestRequirement::DeuteriumSynthesizer(_)
        | QuestRequirement::FusionReactor(_)
        | QuestRequirement::RoboticsFactory(_)
        | QuestRequirement::NaniteFactory(_)
        | QuestRequirement::Shipyard(_)
        | QuestRequirement::CombustionDrive(_)
        | QuestRequirement::ImpulseDrive(_)
        | QuestRequirement::HyperspaceDrive(_)
        | QuestRequirement::WeaponsTechnology(_)
        | QuestRequirement::ShieldingTechnology(_)
        | QuestRequirement::ArmorTechnology(_) => 5,
        _ => 6,
    };
    let required = recurring_requirement_required(period, req);
    ((category as u64) << 56) | (required & 0x00FF_FFFF_FFFF_FFFF)
}

fn recurring_quest_requirement_met(
    period: u8,
    quest_id: u8,
    epoch: i64,
    progress: &QuestProgressState,
) -> Result<bool> {
    let quest = rotating_quest(period, quest_id, epoch)?;
    let current = recurring_requirement_current(period, quest.req, progress)?;
    let required = recurring_requirement_required(period, quest.req);
    Ok(current >= required)
}

fn validate_or_lock_quest_reward_target(
    targets_info: Option<&AccountInfo>,
    authority: Pubkey,
    claiming_planet: Pubkey,
    program_id: Pubkey,
    now: i64,
    period: u8,
    quest_id: u8,
    epoch: i64,
) -> Result<()> {
    let Some(info) = targets_info else {
        return err!(GameStateError::InvalidArgs);
    };
    let quest = rotating_quest(period, quest_id, epoch)?;
    let metric = recurring_requirement_metric(quest.req);
    let mut targets = validate_quest_reward_targets_pda(info, authority, &program_id)?;
    sync_quest_reward_target_periods(&mut targets, now);
    let slot = quest_target_slot_mut(&mut targets, period, metric)?;
    if *slot == Pubkey::default() {
        *slot = claiming_planet;
        targets.last_updated_ts = now;
        write_program_account(info, &targets)?;
        return Ok(());
    }
    require_keys_eq!(
        *slot,
        claiming_planet,
        GameStateError::WrongQuestRewardPlanet
    );
    Ok(())
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
        (0, 20) => Ok((5_000, 5_000, 2_000)),
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

fn total_ships_live(planet: &PlanetQuestFields) -> u32 {
    planet
        .small_cargo
        .saturating_add(planet.large_cargo)
        .saturating_add(planet.light_fighter)
        .saturating_add(planet.heavy_fighter)
        .saturating_add(planet.cruiser)
        .saturating_add(planet.battleship)
        .saturating_add(planet.battlecruiser)
        .saturating_add(planet.bomber)
        .saturating_add(planet.destroyer)
        .saturating_add(planet.deathstar)
        .saturating_add(planet.recycler)
        .saturating_add(planet.espionage_probe)
        .saturating_add(planet.colony_ship)
        .saturating_add(planet.solar_satellite)
}

fn total_defenses_live(planet: &PlanetQuestFields) -> u32 {
    planet
        .rocket_launcher
        .saturating_add(planet.light_laser)
        .saturating_add(planet.heavy_laser)
        .saturating_add(planet.gauss_cannon)
        .saturating_add(planet.ion_cannon)
        .saturating_add(planet.plasma_turret)
        .saturating_add(planet.small_shield_dome)
        .saturating_add(planet.large_shield_dome)
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
}

pub fn produce(ctx: Context<MutatePlanetState>, _now: i64) -> Result<()> {
    let now = chain_now()?;
    produce_planet(&mut ctx.accounts.planet_state, now)
}

pub fn produce_vault(ctx: Context<MutatePlanetStateVault>, _now: i64) -> Result<()> {
    let planet_info = ctx.accounts.planet_state.to_account_info();
    let mut planet = read_planet_build_fields(&planet_info, ctx.program_id)?;
    require_active_vault_for_live_planet(
        ctx.program_id,
        ctx.accounts.vault_signer.key(),
        &ctx.accounts.authorized_vault,
        ctx.accounts.authorized_vault.key(),
        planet.deposit.authority,
    )?;
    let now = chain_now()?;
    settle_planet_deposit_fields(&mut planet.deposit, now)?;
    write_planet_build_fields(&planet_info, &planet)
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
    let planet_info = ctx.accounts.planet_state.to_account_info();
    let mut planet = read_planet_build_fields(&planet_info, ctx.program_id)?;
    require_active_vault_for_live_planet(
        ctx.program_id,
        ctx.accounts.vault_signer.key(),
        &ctx.accounts.authorized_vault,
        ctx.accounts.authorized_vault.key(),
        planet.deposit.authority,
    )?;
    let now = chain_now()?;
    start_build_live(&mut planet, building_idx, now)?;
    write_planet_build_fields(&planet_info, &planet)
}

pub fn finish_build(ctx: Context<MutatePlanetState>, _now: i64) -> Result<()> {
    let now = chain_now()?;
    finish_build_planet(&mut ctx.accounts.planet_state, now)
}

pub fn finish_build_vault(ctx: Context<MutatePlanetStateVault>, _now: i64) -> Result<()> {
    let planet_info = ctx.accounts.planet_state.to_account_info();
    let mut planet = read_planet_build_fields(&planet_info, ctx.program_id)?;
    require_active_vault_for_live_planet(
        ctx.program_id,
        ctx.accounts.vault_signer.key(),
        &ctx.accounts.authorized_vault,
        ctx.accounts.authorized_vault.key(),
        planet.deposit.authority,
    )?;
    let now = chain_now()?;
    finish_build_live(&mut planet, now)?;
    write_planet_build_fields(&planet_info, &planet)
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
    let planet_info = ctx.accounts.planet_state.to_account_info();
    let mut planet = read_planet_build_fields(&planet_info, ctx.program_id)?;
    require_active_vault_for_live_planet(
        ctx.program_id,
        ctx.accounts.vault_signer.key(),
        &ctx.accounts.authorized_vault,
        ctx.accounts.authorized_vault.key(),
        planet.deposit.authority,
    )?;
    let now = chain_now()?;
    start_research_live(&mut planet, tech_idx, now)?;
    write_planet_build_fields(&planet_info, &planet)
}

pub fn finish_research(ctx: Context<MutatePlanetState>, _now: i64) -> Result<()> {
    let now = chain_now()?;
    finish_research_planet(&mut ctx.accounts.planet_state, now)
}

pub fn finish_research_vault(ctx: Context<MutatePlanetStateVault>, _now: i64) -> Result<()> {
    let planet_info = ctx.accounts.planet_state.to_account_info();
    let mut planet = read_planet_build_fields(&planet_info, ctx.program_id)?;
    require_active_vault_for_live_planet(
        ctx.program_id,
        ctx.accounts.vault_signer.key(),
        &ctx.accounts.authorized_vault,
        ctx.accounts.authorized_vault.key(),
        planet.deposit.authority,
    )?;
    let now = chain_now()?;
    finish_research_live(&mut planet, now)?;
    write_planet_build_fields(&planet_info, &planet)
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
    let planet_info = ctx.accounts.planet_state.to_account_info();
    let authority = {
        if *planet_info.owner != *ctx.program_id {
            return raw_game_error(GameStateError::Unauthorized);
        }
        let data = planet_info.try_borrow_data()?;
        if require_planet_state_data(&data).is_err() {
            return raw_game_error(GameStateError::InvalidArgs);
        }
        read_pubkey_at(&data, PLANET_AUTHORITY_OFFSET)
    };
    require_active_vault_for_live_planet(
        ctx.program_id,
        ctx.accounts.vault_signer.key(),
        &ctx.accounts.authorized_vault,
        ctx.accounts.authorized_vault.key(),
        authority,
    )?;
    let now = chain_now()?;
    start_ship_build_bytes(&planet_info, ship_type, quantity, now)
}

pub fn finish_ship_build(ctx: Context<MutatePlanetState>, _now: i64) -> Result<()> {
    let now = chain_now()?;
    finish_ship_build_planet(&mut ctx.accounts.planet_state, now)
}

pub fn finish_ship_build_vault(ctx: Context<MutatePlanetStateVault>, _now: i64) -> Result<()> {
    let planet_info = ctx.accounts.planet_state.to_account_info();
    let mut planet = read_planet_build_fields(&planet_info, ctx.program_id)?;
    require_active_vault_for_live_planet(
        ctx.program_id,
        ctx.accounts.vault_signer.key(),
        &ctx.accounts.authorized_vault,
        ctx.accounts.authorized_vault.key(),
        planet.deposit.authority,
    )?;
    let now = chain_now()?;
    finish_ship_build_live(&mut planet, now)?;
    write_planet_build_fields(&planet_info, &planet)
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
    let planet_info = ctx.accounts.planet_state.to_account_info();
    let authority = {
        require_keys_eq!(
            *planet_info.owner,
            *ctx.program_id,
            GameStateError::Unauthorized
        );
        let data = planet_info.try_borrow_data()?;
        require_planet_state_data(&data)?;
        read_pubkey_at(&data, PLANET_AUTHORITY_OFFSET)
    };
    require_active_vault_for_live_planet(
        ctx.program_id,
        ctx.accounts.vault_signer.key(),
        &ctx.accounts.authorized_vault,
        ctx.accounts.authorized_vault.key(),
        authority,
    )?;
    let now = chain_now()?;
    start_defense_build_bytes(&planet_info, defense_type, quantity, now)
}

pub fn finish_defense_build(ctx: Context<MutatePlanetState>, _now: i64) -> Result<()> {
    let now = chain_now()?;
    finish_defense_build_planet(&mut ctx.accounts.planet_state, now)
}

pub fn finish_defense_build_vault(ctx: Context<MutatePlanetStateVault>, _now: i64) -> Result<()> {
    let planet_info = ctx.accounts.planet_state.to_account_info();
    let authority = {
        require_keys_eq!(
            *planet_info.owner,
            *ctx.program_id,
            GameStateError::Unauthorized
        );
        let data = planet_info.try_borrow_data()?;
        require_planet_state_data(&data)?;
        read_pubkey_at(&data, PLANET_AUTHORITY_OFFSET)
    };
    require_active_vault_for_live_planet(
        ctx.program_id,
        ctx.accounts.vault_signer.key(),
        &ctx.accounts.authorized_vault,
        ctx.accounts.authorized_vault.key(),
        authority,
    )?;
    let now = chain_now()?;
    finish_defense_build_bytes(&planet_info, now)
}

pub fn launch_fleet(ctx: Context<MutatePlanetState>, params: LaunchFleetParams) -> Result<()> {
    launch_fleet_planet(&mut ctx.accounts.planet_state, params)
}

pub fn launch_fleet_vault(
    ctx: Context<MutatePlanetStateVault>,
    params: LaunchFleetParams,
) -> Result<()> {
    let planet_info = ctx.accounts.planet_state.to_account_info();
    let mut planet: PlanetState = read_program_account(&planet_info, ctx.program_id)?;
    require_active_vault_for_live_planet(
        ctx.program_id,
        ctx.accounts.vault_signer.key(),
        &ctx.accounts.authorized_vault,
        ctx.accounts.authorized_vault.key(),
        planet.authority,
    )?;
    launch_fleet_planet(&mut planet, params)?;
    write_program_account(&planet_info, &planet)
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
    require_keys_neq!(
        ctx.accounts.source_planet.key(),
        ctx.accounts.destination_planet.key(),
        GameStateError::InvalidDestination
    );
    let now = chain_now()?;
    let slot_idx = slot as usize;
    require!(slot_idx < MAX_MISSIONS, GameStateError::InvalidMissionSlot);
    let count_progress = !ctx.accounts.source_planet.mission(slot_idx).applied;
    resolve_transport_planets(
        &mut ctx.accounts.source_planet,
        &mut ctx.accounts.destination_planet,
        slot_idx,
        now,
    )?;
    if count_progress {
        increment_quest_progress(
            ctx.remaining_accounts.first(),
            ctx.remaining_accounts.get(1),
            ctx.accounts.authority.key(),
            ctx.program_id,
            now,
            QuestProgressMetric::TransportsResolved,
            1,
            Some(ctx.accounts.source_planet.key()),
        )?;
    }
    Ok(())
}

pub fn resolve_transport_vault(
    ctx: Context<ResolveTransportVault>,
    slot: u8,
    _now: i64,
) -> Result<()> {
    msg!("resolve_transport_vault: entered");
    msg!("resolve_transport_vault: slot={}", slot);
    require_keys_neq!(
        ctx.accounts.source_planet.key(),
        ctx.accounts.destination_planet.key(),
        GameStateError::InvalidDestination
    );
    require_active_vault(
        ctx.accounts.vault_signer.key(),
        &ctx.accounts.authorized_vault,
        ctx.accounts.source_planet.authority,
    )?;
    msg!("resolve_transport_vault: vault ok");
    let now = chain_now()?;
    let slot_idx = slot as usize;
    require!(slot_idx < MAX_MISSIONS, GameStateError::InvalidMissionSlot);
    let count_progress = !ctx.accounts.source_planet.mission(slot_idx).applied;
    resolve_transport_planets(
        &mut ctx.accounts.source_planet,
        &mut ctx.accounts.destination_planet,
        slot_idx,
        now,
    )?;
    if count_progress {
        increment_quest_progress(
            ctx.remaining_accounts.first(),
            ctx.remaining_accounts.get(1),
            ctx.accounts.source_planet.authority,
            ctx.program_id,
            now,
            QuestProgressMetric::TransportsResolved,
            1,
            Some(ctx.accounts.source_planet.key()),
        )?;
    }
    Ok(())
}

pub fn resolve_transport_empty(ctx: Context<MutatePlanetState>, slot: u8, _now: i64) -> Result<()> {
    let now = chain_now()?;
    let slot_idx = slot as usize;
    require!(slot_idx < MAX_MISSIONS, GameStateError::InvalidMissionSlot);
    let count_progress = !ctx.accounts.planet_state.mission(slot_idx).applied;
    resolve_transport_empty_slot(
        &mut ctx.accounts.planet_state,
        slot_idx,
        now,
        ctx.remaining_accounts.first(),
        ctx.program_id,
    )?;
    if count_progress {
        increment_quest_progress(
            ctx.remaining_accounts.get(1),
            ctx.remaining_accounts.get(2),
            ctx.accounts.authority.key(),
            ctx.program_id,
            now,
            QuestProgressMetric::TransportsResolved,
            1,
            Some(ctx.accounts.planet_state.key()),
        )?;
    }
    Ok(())
}

pub fn resolve_transport_empty_vault(
    ctx: Context<MutatePlanetStateVault>,
    slot: u8,
    _now: i64,
) -> Result<()> {
    let planet_info = ctx.accounts.planet_state.to_account_info();
    let mut planet: PlanetState = read_program_account(&planet_info, ctx.program_id)?;
    require_active_vault(
        ctx.accounts.vault_signer.key(),
        &ctx.accounts.authorized_vault,
        planet.authority,
    )?;
    let now = chain_now()?;
    let slot_idx = slot as usize;
    require!(slot_idx < MAX_MISSIONS, GameStateError::InvalidMissionSlot);
    let count_progress = !planet.mission(slot_idx).applied;
    resolve_transport_empty_slot(
        &mut planet,
        slot_idx,
        now,
        ctx.remaining_accounts.first(),
        ctx.program_id,
    )?;
    let authority = planet.authority;
    write_program_account(&planet_info, &planet)?;
    if count_progress {
        increment_quest_progress(
            ctx.remaining_accounts.get(1),
            ctx.remaining_accounts.get(2),
            authority,
            ctx.program_id,
            now,
            QuestProgressMetric::TransportsResolved,
            1,
            Some(planet_info.key()),
        )?;
    }
    Ok(())
}

pub fn resolve_attack(ctx: Context<ResolveAttack>, slot: u8, _now: i64) -> Result<()> {
    let source_key = ctx.accounts.source_planet.key();
    let destination_key = ctx.accounts.destination_planet.key();
    let now = chain_now()?;
    let slot_idx = slot as usize;
    require!(slot_idx < MAX_MISSIONS, GameStateError::InvalidMissionSlot);
    let count_progress = !ctx.accounts.source_planet.mission(slot_idx).applied;
    resolve_attack_planets(
        &mut ctx.accounts.source_planet,
        &mut ctx.accounts.destination_planet,
        &mut ctx.accounts.destination_coords,
        source_key,
        destination_key,
        slot_idx,
        now,
    )?;
    if count_progress {
        increment_quest_progress(
            ctx.remaining_accounts.first(),
            ctx.remaining_accounts.get(1),
            ctx.accounts.authority.key(),
            ctx.program_id,
            now,
            QuestProgressMetric::AttacksResolved,
            1,
            Some(ctx.accounts.source_planet.key()),
        )?;
    }
    Ok(())
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
    let slot_idx = slot as usize;
    require!(slot_idx < MAX_MISSIONS, GameStateError::InvalidMissionSlot);
    let count_progress = !ctx.accounts.source_planet.mission(slot_idx).applied;
    resolve_attack_planets(
        &mut ctx.accounts.source_planet,
        &mut ctx.accounts.destination_planet,
        &mut ctx.accounts.destination_coords,
        source_key,
        destination_key,
        slot_idx,
        now,
    )?;
    if count_progress {
        increment_quest_progress(
            ctx.remaining_accounts.first(),
            ctx.remaining_accounts.get(1),
            ctx.accounts.source_planet.authority,
            ctx.program_id,
            now,
            QuestProgressMetric::AttacksResolved,
            1,
            Some(ctx.accounts.source_planet.key()),
        )?;
    }
    Ok(())
}

pub fn resolve_espionage(ctx: Context<ResolveAttack>, slot: u8, _now: i64) -> Result<()> {
    let source_key = ctx.accounts.source_planet.key();
    let destination_key = ctx.accounts.destination_planet.key();
    let now = chain_now()?;
    let slot_idx = slot as usize;
    require!(slot_idx < MAX_MISSIONS, GameStateError::InvalidMissionSlot);
    let count_progress = !ctx.accounts.source_planet.mission(slot_idx).applied;
    resolve_espionage_planets(
        &mut ctx.accounts.source_planet,
        &mut ctx.accounts.destination_planet,
        source_key,
        destination_key,
        slot_idx,
        now,
    )?;
    if count_progress {
        increment_quest_progress(
            ctx.remaining_accounts.first(),
            ctx.remaining_accounts.get(1),
            ctx.accounts.authority.key(),
            ctx.program_id,
            now,
            QuestProgressMetric::SpyMissionsResolved,
            1,
            Some(ctx.accounts.source_planet.key()),
        )?;
    }
    Ok(())
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
    let slot_idx = slot as usize;
    require!(slot_idx < MAX_MISSIONS, GameStateError::InvalidMissionSlot);
    let count_progress = !ctx.accounts.source_planet.mission(slot_idx).applied;
    resolve_espionage_planets(
        &mut ctx.accounts.source_planet,
        &mut ctx.accounts.destination_planet,
        source_key,
        destination_key,
        slot_idx,
        now,
    )?;
    if count_progress {
        increment_quest_progress(
            ctx.remaining_accounts.first(),
            ctx.remaining_accounts.get(1),
            ctx.accounts.source_planet.authority,
            ctx.program_id,
            now,
            QuestProgressMetric::SpyMissionsResolved,
            1,
            Some(ctx.accounts.source_planet.key()),
        )?;
    }
    Ok(())
}

/// Legacy two-step colonization is disabled.
/// `initialize_colony` now proves and consumes the source mission slot atomically.
pub fn resolve_colonize(_ctx: Context<ResolveColonize>, _slot: u8, _now: i64) -> Result<()> {
    err!(GameStateError::InvalidMission)
}

/// Legacy two-step colonization is disabled.
pub fn resolve_colonize_vault(
    _ctx: Context<ResolveColonizeVault>,
    _slot: u8,
    _now: i64,
) -> Result<()> {
    err!(GameStateError::InvalidMission)
}

/// Wallet-signed: transfer ownership of a single planet to a new authority.
///
/// Both the old and new authorities must have initialized their player profile.
/// After transfer, vault-signed gameplay by the new wallet works immediately
/// because `MutatePlanetStateVault` looks up `authorized_vault` via
/// `planet_state.authority`, which now points to the new wallet. Indexers and
/// public planet sync also follow `planet_state.player`, so that profile link is
/// moved with the authority.
///
/// The planet PDA address does not change — it stays seeded by the old wallet.
/// The old wallet's wallet-signed fallback path for this planet stops working
/// (by design — only the new authority owns it).
fn require_distinct_planet_transfer_authorities(
    old_authority: Pubkey,
    new_authority: Pubkey,
) -> Result<()> {
    require_keys_neq!(
        old_authority,
        new_authority,
        GameStateError::Unauthorized
    );
    Ok(())
}

pub fn transfer_planet<'info>(
    ctx: Context<'_, '_, '_, 'info, TransferPlanet<'info>>,
) -> Result<()> {
    let planet_key = ctx.accounts.planet_state.key();
    let planet = &mut ctx.accounts.planet_state;
    let coords = &mut ctx.accounts.planet_coords;
    let old_authority = ctx.accounts.authority.key();
    let new_authority = ctx.accounts.new_authority.key();

    require_distinct_planet_transfer_authorities(old_authority, new_authority)?;
    require!(
        planet.active_missions == 0,
        GameStateError::PlanetHasActiveMissions
    );
    require!(ctx.remaining_accounts.len() >= 4, GameStateError::InvalidArgs);
    require_no_transfer_market_state(
        &ctx.remaining_accounts[2],
        &ctx.remaining_accounts[3],
        planet_key,
    )?;
    let new_owner_slot = ctx.accounts.new_player_profile.planet_count;
    create_or_update_planet_owner_index(
        &ctx.remaining_accounts[0],
        &ctx.accounts.authority.to_account_info(),
        &ctx.accounts.system_program.to_account_info(),
        new_authority,
        new_owner_slot,
        planet_key,
        ctx.program_id,
    )?;
    deactivate_planet_owner_index(
        &ctx.remaining_accounts[1],
        ctx.accounts.ownership_registry.owner_index,
        old_authority,
        planet_key,
        ctx.program_id,
    )?;

    ctx.accounts.ownership_registry.authority = new_authority;
    ctx.accounts.ownership_registry.owner_index = ctx.remaining_accounts[0].key();

    // Update ownership fields. The planet PDA address intentionally stays fixed.
    planet.authority = new_authority;
    planet.player = ctx.accounts.new_player_profile.key();
    coords.authority = new_authority;
    ctx.accounts.new_player_profile.planet_count = ctx
        .accounts
        .new_player_profile
        .planet_count
        .checked_add(1)
        .ok_or(GameStateError::PlanetCountOverflow)?;

    Ok(())
}

pub fn transfer_planet_from_market<'info>(
    ctx: Context<'_, '_, '_, 'info, TransferPlanetFromMarket<'info>>,
) -> Result<()> {
    let (expected_market_authority, _) =
        Pubkey::find_program_address(&[b"market_authority"], &MARKET_PROGRAM_ID);
    require_keys_eq!(
        ctx.accounts.market_authority.key(),
        expected_market_authority,
        GameStateError::Unauthorized
    );

    let planet_key = ctx.accounts.planet_state.key();
    let planet = &mut ctx.accounts.planet_state;
    let coords = &mut ctx.accounts.planet_coords;
    let seller = ctx.accounts.seller.key();
    let new_authority = ctx.accounts.new_authority.key();

    require_keys_eq!(planet.authority, seller, GameStateError::Unauthorized);
    require_keys_eq!(coords.authority, seller, GameStateError::Unauthorized);
    if ctx.accounts.new_player_profile.authority == Pubkey::default() {
        ctx.accounts.new_player_profile.authority = new_authority;
        ctx.accounts.new_player_profile.bump = ctx.bumps.new_player_profile;
    }
    require!(
        planet.active_missions == 0,
        GameStateError::PlanetHasActiveMissions
    );

    require!(ctx.remaining_accounts.len() >= 2, GameStateError::InvalidArgs);
    let new_owner_slot = ctx.accounts.new_player_profile.planet_count;
    create_or_update_planet_owner_index(
        &ctx.remaining_accounts[0],
        &ctx.accounts.new_authority.to_account_info(),
        &ctx.accounts.system_program.to_account_info(),
        new_authority,
        new_owner_slot,
        planet_key,
        ctx.program_id,
    )?;
    deactivate_planet_owner_index(
        &ctx.remaining_accounts[1],
        ctx.accounts.ownership_registry.owner_index,
        seller,
        planet_key,
        ctx.program_id,
    )?;

    ctx.accounts.ownership_registry.authority = new_authority;
    ctx.accounts.ownership_registry.owner_index = ctx.remaining_accounts[0].key();

    planet.authority = new_authority;
    planet.player = ctx.accounts.new_player_profile.key();
    coords.authority = new_authority;
    ctx.accounts.new_player_profile.planet_count = ctx
        .accounts
        .new_player_profile
        .planet_count
        .checked_add(1)
        .ok_or(GameStateError::PlanetCountOverflow)?;

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

pub fn rotate_game_config_admin(
    ctx: Context<RotateGameConfigAdmin>,
    new_admin: Pubkey,
) -> Result<()> {
    require_keys_eq!(new_admin, ctx.accounts.new_admin.key(), GameStateError::InvalidArgs);
    require!(new_admin != Pubkey::default(), GameStateError::InvalidArgs);
    require!(new_admin != ctx.accounts.admin.key(), GameStateError::InvalidArgs);
    ctx.accounts.game_config.admin = new_admin;
    Ok(())
}

/// Devnet faucet: mint 10,000 ANTIMATTER to a wallet once every 24 hours.
pub fn claim_antimatter_faucet(ctx: Context<ClaimAntimatterFaucet>) -> Result<()> {
    require!(
        ctx.accounts.antimatter_mint.decimals == ANTIMATTER_DECIMALS,
        GameStateError::InvalidAntimatterMintDecimals
    );
    match ctx.accounts.antimatter_mint.mint_authority {
        COption::Some(authority) => require_keys_eq!(
            authority,
            ctx.accounts.faucet_authority.key(),
            GameStateError::Unauthorized
        ),
        COption::None => return err!(GameStateError::Unauthorized),
    }

    let now = chain_now()?;
    if ctx.accounts.faucet_claim.authority == Pubkey::default() {
        ctx.accounts.faucet_claim.authority = ctx.accounts.recipient.key();
        ctx.accounts.faucet_claim.bump = ctx.bumps.faucet_claim;
    } else {
        require_keys_eq!(
            ctx.accounts.faucet_claim.authority,
            ctx.accounts.recipient.key(),
            GameStateError::Unauthorized
        );
        require!(
            now.saturating_sub(ctx.accounts.faucet_claim.last_claim_ts)
                >= ANTIMATTER_FAUCET_COOLDOWN_SECONDS,
            GameStateError::FaucetCooldownActive
        );
    }

    token::mint_to(
        CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            MintTo {
                mint: ctx.accounts.antimatter_mint.to_account_info(),
                to: ctx.accounts.recipient_antimatter_account.to_account_info(),
                authority: ctx.accounts.faucet_authority.to_account_info(),
            },
        ),
        ANTIMATTER_FAUCET_AMOUNT,
    )?;
    ctx.accounts.faucet_claim.last_claim_ts = now;
    Ok(())
}

/// Wallet-signed: burn 1 ANTIMATTER per second remaining to finish a building queue instantly.
pub fn accelerate_build_with_antimatter(ctx: Context<UseAntimatter>) -> Result<()> {
    require_keys_eq!(
        ctx.accounts.game_config.antimatter_mint,
        ctx.accounts.antimatter_mint.key(),
        GameStateError::InvalidAntimatterMint
    );
    let amount = accelerate_build_with_antimatter_inner(
        &mut ctx.accounts.planet_state,
        &ctx.accounts.antimatter_mint,
        &ctx.accounts.user_antimatter_account,
        &ctx.accounts.authority,
        &ctx.accounts.token_program,
    )?;
    let now = chain_now()?;
    increment_quest_progress(
        ctx.remaining_accounts.first(),
        ctx.remaining_accounts.get(1),
        ctx.accounts.authority.key(),
        ctx.program_id,
        now,
        QuestProgressMetric::AntimatterSpent,
        amount,
        Some(ctx.accounts.planet_state.key()),
    )
}

/// Wallet-signed: burn 1 ANTIMATTER per second remaining to finish research instantly.
pub fn accelerate_research_with_antimatter(ctx: Context<UseAntimatter>) -> Result<()> {
    require_keys_eq!(
        ctx.accounts.game_config.antimatter_mint,
        ctx.accounts.antimatter_mint.key(),
        GameStateError::InvalidAntimatterMint
    );
    let amount = accelerate_research_with_antimatter_inner(
        &mut ctx.accounts.planet_state,
        &ctx.accounts.antimatter_mint,
        &ctx.accounts.user_antimatter_account,
        &ctx.accounts.authority,
        &ctx.accounts.token_program,
    )?;
    let now = chain_now()?;
    increment_quest_progress(
        ctx.remaining_accounts.first(),
        ctx.remaining_accounts.get(1),
        ctx.accounts.authority.key(),
        ctx.program_id,
        now,
        QuestProgressMetric::AntimatterSpent,
        amount,
        Some(ctx.accounts.planet_state.key()),
    )
}

/// Wallet-signed: burn 1 ANTIMATTER per second remaining to finish ship production instantly.
pub fn accelerate_ship_build_with_antimatter(ctx: Context<UseAntimatter>) -> Result<()> {
    require_keys_eq!(
        ctx.accounts.game_config.antimatter_mint,
        ctx.accounts.antimatter_mint.key(),
        GameStateError::InvalidAntimatterMint
    );
    let amount = accelerate_ship_build_with_antimatter_inner(
        &mut ctx.accounts.planet_state,
        &ctx.accounts.antimatter_mint,
        &ctx.accounts.user_antimatter_account,
        &ctx.accounts.authority,
        &ctx.accounts.token_program,
    )?;
    let now = chain_now()?;
    increment_quest_progress(
        ctx.remaining_accounts.first(),
        ctx.remaining_accounts.get(1),
        ctx.accounts.authority.key(),
        ctx.program_id,
        now,
        QuestProgressMetric::AntimatterSpent,
        amount,
        Some(ctx.accounts.planet_state.key()),
    )
}

/// Wallet-signed: burn 1 ANTIMATTER per second remaining to finish defense production instantly.
pub fn accelerate_defense_build_with_antimatter(ctx: Context<UseAntimatter>) -> Result<()> {
    require_keys_eq!(
        ctx.accounts.game_config.antimatter_mint,
        ctx.accounts.antimatter_mint.key(),
        GameStateError::InvalidAntimatterMint
    );
    let amount = accelerate_defense_build_with_antimatter_inner(
        &mut ctx.accounts.planet_state,
        &ctx.accounts.antimatter_mint,
        &ctx.accounts.user_antimatter_account,
        &ctx.accounts.authority,
        &ctx.accounts.token_program,
    )?;
    let now = chain_now()?;
    increment_quest_progress(
        ctx.remaining_accounts.first(),
        ctx.remaining_accounts.get(1),
        ctx.accounts.authority.key(),
        ctx.program_id,
        now,
        QuestProgressMetric::AntimatterSpent,
        amount,
        Some(ctx.accounts.planet_state.key()),
    )
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
    let amount = accelerate_mission_with_antimatter_inner(
        &mut ctx.accounts.planet_state,
        &ctx.accounts.antimatter_mint,
        &ctx.accounts.user_antimatter_account,
        &ctx.accounts.authority,
        &ctx.accounts.token_program,
        slot,
        leg,
    )?;
    let now = chain_now()?;
    increment_quest_progress(
        ctx.remaining_accounts.first(),
        ctx.remaining_accounts.get(1),
        ctx.accounts.authority.key(),
        ctx.program_id,
        now,
        QuestProgressMetric::AntimatterSpent,
        amount,
        Some(ctx.accounts.planet_state.key()),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn program_account_owner_guard_rejects_foreign_owner() {
        let key = Pubkey::new_unique();
        let foreign_owner = Pubkey::new_unique();
        let mut lamports = 1;
        let mut data = vec![0u8; 16];
        let account = AccountInfo::new(
            &key,
            false,
            true,
            &mut lamports,
            &mut data,
            &foreign_owner,
            false,
            0,
        );

        assert!(ensure_program_account_owner(&account).is_err());
    }

    #[test]
    fn program_account_owner_guard_accepts_game_state_owner() {
        let key = Pubkey::new_unique();
        let mut lamports = 1;
        let mut data = vec![0u8; 16];
        let account = AccountInfo::new(
            &key,
            false,
            true,
            &mut lamports,
            &mut data,
            &crate::ID,
            false,
            0,
        );

        assert!(ensure_program_account_owner(&account).is_ok());
    }

    fn empty_alliance_treasury() -> AllianceTreasuryState {
        AllianceTreasuryState {
            alliance: Pubkey::new_unique(),
            metal: 0,
            crystal: 0,
            deuterium: 0,
            antimatter: 0,
            logistics_hub: 0,
            research_grid: 0,
            defense_coordination: 0,
            trade_network: 0,
            total_metal_deposited: 0,
            total_crystal_deposited: 0,
            total_deuterium_deposited: 0,
            total_antimatter_deposited: 0,
            bump: 0,
        }
    }

    fn inactive_membership(authority: Pubkey, alliance: Pubkey) -> AllianceMembership {
        AllianceMembership {
            authority,
            alliance,
            role: 0,
            joined_at: 10,
            daily_epoch: 20,
            weekly_epoch: 3,
            monthly_epoch: 1,
            daily_claimed_mask: 0b101,
            weekly_claimed_mask: 0b010,
            monthly_claimed_mask: 0b001,
            bump: 7,
        }
    }

    #[test]
    fn same_alliance_reactivation_preserves_current_claims() {
        let authority = Pubkey::new_unique();
        let alliance = Pubkey::new_unique();
        let mut membership = inactive_membership(authority, alliance);
        let now = 20 * 86_400;
        membership.daily_epoch = daily_epoch(now);
        membership.weekly_epoch = weekly_epoch(now);
        membership.monthly_epoch = monthly_epoch(now);

        activate_alliance_membership_state(&mut membership, authority, alliance, 1, now, 7)
            .unwrap();

        assert_eq!(membership.role, 1);
        assert_eq!(membership.daily_claimed_mask, 0b101);
        assert_eq!(membership.weekly_claimed_mask, 0b010);
        assert_eq!(membership.monthly_claimed_mask, 0b001);
    }

    #[test]
    fn different_alliance_activation_resets_claims() {
        let authority = Pubkey::new_unique();
        let mut membership = inactive_membership(authority, Pubkey::new_unique());
        let new_alliance = Pubkey::new_unique();
        let now = 20 * 86_400;

        activate_alliance_membership_state(
            &mut membership,
            authority,
            new_alliance,
            1,
            now,
            7,
        )
        .unwrap();

        assert_eq!(membership.alliance, new_alliance);
        assert_eq!(membership.role, 1);
        assert_eq!(membership.daily_claimed_mask, 0);
        assert_eq!(membership.weekly_claimed_mask, 0);
        assert_eq!(membership.monthly_claimed_mask, 0);
    }

    #[test]
    fn inactive_alliance_membership_cannot_decrement_member_count_again() {
        let alliance_key = Pubkey::new_unique();
        let mut alliance = AllianceState {
            founder: Pubkey::new_unique(),
            name: [0; MAX_ALLIANCE_NAME_LEN],
            level: 1,
            xp: 0,
            member_count: 2,
            max_members: BASE_ALLIANCE_MAX_MEMBERS,
            total_missions_completed: 0,
            created_at: 0,
            bump: 0,
        };
        let mut membership = inactive_membership(Pubkey::new_unique(), alliance_key);

        assert!(deactivate_alliance_membership(&mut alliance, &mut membership).is_err());
        assert_eq!(alliance.member_count, 2);
        assert_eq!(membership.role, 0);
    }

    #[test]
    fn active_alliance_membership_decrements_member_count_once() {
        let alliance_key = Pubkey::new_unique();
        let mut alliance = AllianceState {
            founder: Pubkey::new_unique(),
            name: [0; MAX_ALLIANCE_NAME_LEN],
            level: 1,
            xp: 0,
            member_count: 2,
            max_members: BASE_ALLIANCE_MAX_MEMBERS,
            total_missions_completed: 0,
            created_at: 0,
            bump: 0,
        };
        let mut membership = inactive_membership(Pubkey::new_unique(), alliance_key);
        membership.role = 1;

        deactivate_alliance_membership(&mut alliance, &mut membership).unwrap();
        assert_eq!(alliance.member_count, 1);
        assert_eq!(membership.role, 0);
        assert!(deactivate_alliance_membership(&mut alliance, &mut membership).is_err());
        assert_eq!(alliance.member_count, 1);
    }

    #[test]
    fn alliance_treasury_credit_is_atomic_on_overflow() {
        let mut treasury = empty_alliance_treasury();
        treasury.metal = u64::MAX;

        assert!(credit_alliance_treasury(&mut treasury, 1, 2, 3, 4).is_err());
        assert_eq!(treasury.metal, u64::MAX);
        assert_eq!(treasury.crystal, 0);
        assert_eq!(treasury.total_crystal_deposited, 0);

        let mut valid = empty_alliance_treasury();
        credit_alliance_treasury(&mut valid, 1, 2, 3, 4).unwrap();
        assert_eq!(valid.metal, 1);
        assert_eq!(valid.crystal, 2);
        assert_eq!(valid.deuterium, 3);
        assert_eq!(valid.antimatter, 4);
        assert_eq!(valid.total_antimatter_deposited, 4);
    }

    #[test]
    fn maximum_alliance_discounts_preserve_resource_sinks() {
        let mut treasury = empty_alliance_treasury();
        treasury.trade_network = ALLIANCE_BUILDING_MAX_LEVEL;
        treasury.defense_coordination = ALLIANCE_BUILDING_MAX_LEVEL;

        assert_eq!(alliance_trade_cost_discount_bps(&treasury), 2_500);
        assert_eq!(alliance_defense_deuterium_discount_bps(&treasury), 3_750);
        assert_eq!(
            apply_alliance_upgrade_discounts(100, 100, 100, 100, &treasury),
            (75, 75, 63, 75)
        );
    }

    #[test]
    fn alliance_deposit_variable_xp_is_capped() {
        let mission = AllianceDepositMissionEntry {
            metal: 1_000,
            crystal: 0,
            deuterium: 0,
            antimatter: 0,
            xp: 80,
        };

        assert_eq!(alliance_deposit_xp(&mission, 1_000, 0, 0, 0), 81);
        assert_eq!(
            alliance_deposit_xp(&mission, u64::MAX, u64::MAX, u64::MAX, u64::MAX),
            880
        );
    }

    #[test]
    fn alliance_level_refresh_handles_maximum_xp_in_constant_iterations() {
        let mut alliance = AllianceState {
            founder: Pubkey::new_unique(),
            name: [0; MAX_ALLIANCE_NAME_LEN],
            level: 1,
            xp: u64::MAX,
            member_count: 1,
            max_members: BASE_ALLIANCE_MAX_MEMBERS,
            total_missions_completed: 0,
            created_at: 0,
            bump: 0,
        };

        refresh_alliance_level(&mut alliance);

        assert_eq!(alliance.level, u16::MAX);
        assert_eq!(alliance.max_members, u16::MAX);
    }

    #[test]
    fn direct_planet_transfer_rejects_same_authority() {
        let authority = Pubkey::new_unique();
        assert!(require_distinct_planet_transfer_authorities(authority, authority).is_err());
        assert!(require_distinct_planet_transfer_authorities(authority, Pubkey::new_unique()).is_ok());
    }

    #[test]
    fn owner_index_sync_rejects_unallocated_slots() {
        assert!(require_allocated_owner_index_slot(0, 1).is_ok());
        assert!(require_allocated_owner_index_slot(1, 1).is_err());
        assert!(require_allocated_owner_index_slot(u32::MAX, 1).is_err());
    }

    #[test]
    fn raw_planet_paths_require_size_and_discriminator() {
        let minimum_len = PLANET_DEFENSE_BUILD_FINISH_TS_OFFSET + 8;
        let mut valid = vec![0u8; minimum_len];
        valid[..8].copy_from_slice(
            &<PlanetState as anchor_lang::Discriminator>::DISCRIMINATOR,
        );

        assert!(require_planet_state_data(&valid).is_ok());
        assert!(require_planet_state_data(&valid[..minimum_len - 1]).is_err());

        valid[0] ^= 0xff;
        assert!(require_planet_state_data(&valid).is_err());
    }

    #[test]
    fn rotating_quests_expose_twelve_claimable_slots_per_period() {
        for period in [1u8, 2, 3] {
            for quest_id in 0u8..12 {
                let claim_id = if period == 1 { quest_id + 1 } else { quest_id };
                rotating_quest(period, claim_id, 0)
                    .unwrap_or_else(|_| panic!("missing period {period} quest slot {claim_id}"));
            }
        }
    }

    #[test]
    fn rotating_quests_do_not_repeat_requirements_in_same_period() {
        for epoch in 0i64..370 {
            for period in [1u8, 2, 3] {
                let mut signatures = [0u64; 12];
                for quest_id in 0u8..12 {
                    let claim_id = if period == 1 { quest_id + 1 } else { quest_id };
                    let quest = rotating_quest(period, claim_id, epoch).unwrap_or_else(|_| {
                        panic!("missing period {period} epoch {epoch} quest slot {claim_id}")
                    });
                    let signature = recurring_requirement_signature(period, quest.req);
                    assert!(
                        !signatures[..quest_id as usize].contains(&signature),
                        "duplicate period {period} epoch {epoch} quest slot {claim_id}"
                    );
                    signatures[quest_id as usize] = signature;
                }
            }
        }
    }

    #[test]
    fn quest_reward_target_lock_only_sets_periods_with_matching_active_metric() {
        let authority = Pubkey::new_unique();
        let planet = Pubkey::new_unique();
        let metrics = [
            QuestProgressMetric::StorePacksBought,
            QuestProgressMetric::AntimatterSpent,
            QuestProgressMetric::PlanetsColonized,
            QuestProgressMetric::AttacksResolved,
            QuestProgressMetric::TransportsResolved,
            QuestProgressMetric::SpyMissionsResolved,
        ];

        for day in [0i64, 1, 6, 7, 29, 30, 91, 180, 365] {
            let now = day * SECONDS_PER_DAY;
            for metric in metrics {
                let mut targets = empty_quest_reward_targets(authority, now);
                lock_matching_quest_reward_targets(&mut targets, now, metric, planet)
                    .unwrap_or_else(|_| panic!("failed to lock metric {metric:?} on day {day}"));

                for (period, epoch) in [
                    (1u8, daily_epoch(now)),
                    (2u8, weekly_epoch(now)),
                    (3u8, monthly_epoch(now)),
                ] {
                    let expected = period_has_metric_quest(period, epoch, metric).unwrap();
                    let actual = *quest_target_slot_mut(&mut targets, period, metric).unwrap();
                    assert_eq!(
                        actual == planet,
                        expected,
                        "period {period} day {day} metric {metric:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn tutorial_quest_target_slot_map_is_stable() {
        let authority = Pubkey::new_unique();
        let mut targets = empty_tutorial_quest_reward_targets(authority, 0);
        let planet = Pubkey::new_unique();

        let slot_0 = tutorial_quest_target_slot_mut(&mut targets, 0).unwrap();
        assert_eq!(*slot_0, Pubkey::default());

        *slot_0 = planet;
        let slot_0_again = tutorial_quest_target_slot_mut(&mut targets, 0).unwrap();
        assert_eq!(*slot_0_again, planet);

        let slot_20 = tutorial_quest_target_slot_mut(&mut targets, 20).unwrap();
        assert_eq!(*slot_20, Pubkey::default());

        let invalid = tutorial_quest_target_slot_mut(&mut targets, 21);
        assert!(invalid.is_err());
    }
}
