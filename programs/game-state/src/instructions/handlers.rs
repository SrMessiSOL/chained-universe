use anchor_lang::prelude::*;

use crate::contexts::*;
use crate::constants::*;
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
    require!(backup_ciphertext.len() <= 512, GameStateError::BackupTooLarge);

    let authority = ctx.accounts.authority.key();

    ctx.accounts.player_profile.set_inner(PlayerProfile {
        authority,
        planet_count: 0,
        bump: ctx.bumps.player_profile,
    });

    let now = Clock::get()?.unix_timestamp;
    require!(expires_at == 0 || expires_at > now, GameStateError::InvalidArgs);

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
    require!(backup_ciphertext.len() <= 512, GameStateError::BackupTooLarge);

    let now = Clock::get()?.unix_timestamp;
    require!(expires_at == 0 || expires_at > now, GameStateError::InvalidArgs);

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
    require!(expires_at == 0 || expires_at > now, GameStateError::InvalidArgs);
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
        name: if params.name.is_empty() { "Homeworld".to_string() } else { params.name },
        galaxy: if params.galaxy == 0 { ((auth_bytes[0] as u16) % 9) + 1 } else { params.galaxy.clamp(1, 9) },
        system: if params.system == 0 { (u16::from_le_bytes([auth_bytes[1], auth_bytes[2]]) % 499) + 1 } else { params.system.clamp(1, 499) },
        position,
        diameter: 8_000u32 + (u16::from_le_bytes([auth_bytes[4], auth_bytes[5]]) as u32 % 10_000),
        temperature: (base_temp + ((auth_bytes[6] as i16) % 40 - 20)).clamp(-60, 120),
        max_fields: 163u16 + (auth_bytes[7] as u16 % 40),
        used_fields: 3,
        metal_mine: 1, crystal_mine: 1, deuterium_synthesizer: 1, solar_plant: 1,
        fusion_reactor: 0, robotics_factory: 0, nanite_factory: 0, shipyard: 0,
        metal_storage: 0, crystal_storage: 0, deuterium_tank: 0, research_lab: 0, missile_silo: 0,
        energy_tech: 0, combustion_drive: 0, impulse_drive: 0, hyperspace_drive: 0,
        computer_tech: 0, astrophysics: 0, igr_network: 0,
        research_queue_item: 255, research_queue_target: 0, research_finish_ts: 0,
        build_queue_item: 255, build_queue_target: 0, build_finish_ts: 0,
        ship_build_item: 255, ship_build_qty: 0, ship_build_finish_ts: 0,
        metal: 1_000_000, crystal: 1_000_000, deuterium: 1_000_000,
        metal_hour: 33, crystal_hour: 22, deuterium_hour: 14,
        energy_production: 22, energy_consumption: 42,
        metal_cap: 1_000_000, crystal_cap: 1_000_000, deuterium_cap: 1_000_000,
        last_update_ts: params.now,
        small_cargo: 0, large_cargo: 0, light_fighter: 0, heavy_fighter: 0,
        cruiser: 0, battleship: 0, battlecruiser: 0, bomber: 0, destroyer: 0,
        deathstar: 0, recycler: 0, espionage_probe: 0, colony_ship: 0, solar_satellite: 0,
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

/// Vault-signed: initialize colony.
/// Creates both `planet_state` and `planet_coords` atomically.
/// If `planet_coords` already exists the tx fails — client shows "slot occupied".
pub fn initialize_colony(
    ctx: Context<InitializePlanetVault>,
    params: InitializeColonyParams,
) -> Result<()> {
    require_active_vault(
        ctx.accounts.vault_signer.key(),
        &ctx.accounts.authorized_vault,
        ctx.accounts.player_profile.authority,
    )?;

    let authority = ctx.accounts.player_profile.authority;

    let planet_params = InitializePlanetParams {
        name: if params.name.is_empty() { "Colony".to_string() } else { params.name },
        galaxy: params.galaxy,
        system: params.system,
        position: params.position,
        diameter: 8_000u32 + ((params.galaxy as u32 * 997 + params.system as u32 * 37 + params.position as u32 * 101) % 10_000),
        temperature: (120i16 - (params.position as i16 * 12)).clamp(-60, 120),
        max_fields: 163u16 + ((params.galaxy + params.system + params.position as u16) % 40),
        used_fields: 3,
        metal_mine: 1, crystal_mine: 1, deuterium_synthesizer: 1, solar_plant: 1,
        fusion_reactor: 0, robotics_factory: 0, nanite_factory: 0, shipyard: 0,
        metal_storage: 0, crystal_storage: 0, deuterium_tank: 0, research_lab: 0, missile_silo: 0,
        energy_tech: 0, combustion_drive: 0, impulse_drive: 0, hyperspace_drive: 0,
        computer_tech: 0, astrophysics: 0, igr_network: 0,
        research_queue_item: 255, research_queue_target: 0, research_finish_ts: 0,
        build_queue_item: 255, build_queue_target: 0, build_finish_ts: 0,
        ship_build_item: 255, ship_build_qty: 0, ship_build_finish_ts: 0,
        metal: params.cargo_metal, crystal: params.cargo_crystal, deuterium: params.cargo_deuterium,
        metal_hour: 33, crystal_hour: 22, deuterium_hour: 14,
        energy_production: 22, energy_consumption: 42,
        metal_cap: 1_000_000, crystal_cap: 1_000_000, deuterium_cap: 1_000_000,
        last_update_ts: params.now,
        small_cargo: params.small_cargo, large_cargo: params.large_cargo,
        light_fighter: params.light_fighter, heavy_fighter: params.heavy_fighter,
        cruiser: params.cruiser, battleship: params.battleship, battlecruiser: params.battlecruiser,
        bomber: params.bomber, destroyer: params.destroyer, deathstar: params.deathstar,
        recycler: params.recycler, espionage_probe: params.espionage_probe,
        colony_ship: 0, solar_satellite: params.solar_satellite,
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

pub fn produce(ctx: Context<MutatePlanetState>, now: i64) -> Result<()> {
    produce_planet(&mut ctx.accounts.planet_state, now)
}

pub fn produce_vault(ctx: Context<MutatePlanetStateVault>, now: i64) -> Result<()> {
    require_active_vault(ctx.accounts.vault_signer.key(), &ctx.accounts.authorized_vault, ctx.accounts.planet_state.authority)?;
    produce_planet(&mut ctx.accounts.planet_state, now)
}

pub fn start_build(ctx: Context<MutatePlanetState>, building_idx: u8, now: i64) -> Result<()> {
    start_build_planet(&mut ctx.accounts.planet_state, building_idx, now)
}

pub fn start_build_vault(ctx: Context<MutatePlanetStateVault>, building_idx: u8, now: i64) -> Result<()> {
    require_active_vault(ctx.accounts.vault_signer.key(), &ctx.accounts.authorized_vault, ctx.accounts.planet_state.authority)?;
    start_build_planet(&mut ctx.accounts.planet_state, building_idx, now)
}

pub fn finish_build(ctx: Context<MutatePlanetState>, now: i64) -> Result<()> {
    finish_build_planet(&mut ctx.accounts.planet_state, now)
}

pub fn finish_build_vault(ctx: Context<MutatePlanetStateVault>, now: i64) -> Result<()> {
    require_active_vault(ctx.accounts.vault_signer.key(), &ctx.accounts.authorized_vault, ctx.accounts.planet_state.authority)?;
    finish_build_planet(&mut ctx.accounts.planet_state, now)
}

pub fn start_research(ctx: Context<MutatePlanetState>, tech_idx: u8, now: i64) -> Result<()> {
    start_research_planet(&mut ctx.accounts.planet_state, tech_idx, now)
}

pub fn start_research_vault(ctx: Context<MutatePlanetStateVault>, tech_idx: u8, now: i64) -> Result<()> {
    require_active_vault(ctx.accounts.vault_signer.key(), &ctx.accounts.authorized_vault, ctx.accounts.planet_state.authority)?;
    start_research_planet(&mut ctx.accounts.planet_state, tech_idx, now)
}

pub fn finish_research(ctx: Context<MutatePlanetState>, now: i64) -> Result<()> {
    finish_research_planet(&mut ctx.accounts.planet_state, now)
}

pub fn finish_research_vault(ctx: Context<MutatePlanetStateVault>, now: i64) -> Result<()> {
    require_active_vault(ctx.accounts.vault_signer.key(), &ctx.accounts.authorized_vault, ctx.accounts.planet_state.authority)?;
    finish_research_planet(&mut ctx.accounts.planet_state, now)
}

pub fn build_ship(ctx: Context<MutatePlanetState>, ship_type: u8, quantity: u32, now: i64) -> Result<()> {
    build_ship_planet(&mut ctx.accounts.planet_state, ship_type, quantity, now)
}

pub fn build_ship_vault(ctx: Context<MutatePlanetStateVault>, ship_type: u8, quantity: u32, now: i64) -> Result<()> {
    require_active_vault(ctx.accounts.vault_signer.key(), &ctx.accounts.authorized_vault, ctx.accounts.planet_state.authority)?;
    build_ship_planet(&mut ctx.accounts.planet_state, ship_type, quantity, now)
}

pub fn finish_ship_build(ctx: Context<MutatePlanetState>, now: i64) -> Result<()> {
    finish_ship_build_planet(&mut ctx.accounts.planet_state, now)
}

pub fn finish_ship_build_vault(ctx: Context<MutatePlanetStateVault>, now: i64) -> Result<()> {
    require_active_vault(ctx.accounts.vault_signer.key(), &ctx.accounts.authorized_vault, ctx.accounts.planet_state.authority)?;
    finish_ship_build_planet(&mut ctx.accounts.planet_state, now)
}

pub fn launch_fleet(ctx: Context<MutatePlanetState>, params: LaunchFleetParams) -> Result<()> {
    launch_fleet_planet(&mut ctx.accounts.planet_state, params)
}

pub fn launch_fleet_vault(ctx: Context<MutatePlanetStateVault>, params: LaunchFleetParams) -> Result<()> {
    require_active_vault(ctx.accounts.vault_signer.key(), &ctx.accounts.authorized_vault, ctx.accounts.planet_state.authority)?;
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
    settle_resources(planet, now);

    match resource_type {
        ResourceType::Metal => {
            require!(planet.metal >= amount, GameStateError::InsufficientResources);
            planet.metal = planet.metal.saturating_sub(amount);
        }
        ResourceType::Crystal => {
            require!(planet.crystal >= amount, GameStateError::InsufficientResources);
            planet.crystal = planet.crystal.saturating_sub(amount);
        }
        ResourceType::Deuterium => {
            require!(planet.deuterium >= amount, GameStateError::InsufficientResources);
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
    settle_resources(seller, now);

    match resource_type {
        ResourceType::Metal => seller.metal = seller.metal.saturating_add(amount),
        ResourceType::Crystal => seller.crystal = seller.crystal.saturating_add(amount),
        ResourceType::Deuterium => seller.deuterium = seller.deuterium.saturating_add(amount),
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
    settle_resources(buyer, now);
    require_keys_eq!(buyer.authority, ctx.accounts.buyer.key(), GameStateError::Unauthorized);

    match resource_type {
        ResourceType::Metal => buyer.metal = buyer.metal.saturating_add(amount),
        ResourceType::Crystal => buyer.crystal = buyer.crystal.saturating_add(amount),
        ResourceType::Deuterium => buyer.deuterium = buyer.deuterium.saturating_add(amount),
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

pub fn resolve_transport(ctx: Context<ResolveTransport>, slot: u8, now: i64) -> Result<()> {
        msg!("resolve_transport: entered");
        msg!("resolve_transport: slot={}", slot);
    resolve_transport_planets(&mut ctx.accounts.source_planet, &mut ctx.accounts.destination_planet, slot as usize, now)
}

pub fn resolve_transport_vault(ctx: Context<ResolveTransportVault>, slot: u8, now: i64) -> Result<()> {
msg!("resolve_transport_vault: entered");
msg!("resolve_transport_vault: slot={}", slot);
require_active_vault(
    ctx.accounts.vault_signer.key(),
    &ctx.accounts.authorized_vault,
    ctx.accounts.source_planet.authority
)?;
msg!("resolve_transport_vault: vault ok");
resolve_transport_planets(&mut ctx.accounts.source_planet, &mut ctx.accounts.destination_planet, slot as usize, now)
    }

pub fn resolve_transport_empty(ctx: Context<MutatePlanetState>, slot: u8, now: i64) -> Result<()> {
    resolve_transport_empty_slot(&mut ctx.accounts.planet_state, slot as usize, now)
}

pub fn resolve_transport_empty_vault(ctx: Context<MutatePlanetStateVault>, slot: u8, now: i64) -> Result<()> {
    require_active_vault(
        ctx.accounts.vault_signer.key(),
        &ctx.accounts.authorized_vault,
        ctx.accounts.planet_state.authority
    )?;
    resolve_transport_empty_slot(&mut ctx.accounts.planet_state, slot as usize, now)
}

/// Wallet-signed: resolve a colonize mission.
/// The colony planet + coord lock must already exist (created by `initialize_colony`).
pub fn resolve_colonize(ctx: Context<ResolveColonize>, slot: u8, now: i64) -> Result<()> {
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
    resolve_colonize_planet(&mut ctx.accounts.source_planet, slot as usize, now)
}

/// Vault-signed: resolve a colonize mission.
    pub fn resolve_colonize_vault(ctx: Context<ResolveColonizeVault>, slot: u8, now: i64) -> Result<()> {
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
    ctx.accounts.game_config.antimatter_mint = antimatter_mint;
    Ok(())
}

/// Wallet-signed: burn 1 ANTIMATTER per second remaining to finish a building queue instantly.
pub fn accelerate_build_with_antimatter(
    ctx: Context<UseAntimatter>,
) -> Result<()> {
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
pub fn accelerate_research_with_antimatter(
    ctx: Context<UseAntimatter>,
) -> Result<()> {
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
pub fn accelerate_ship_build_with_antimatter(
    ctx: Context<UseAntimatter>,
) -> Result<()> {
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
