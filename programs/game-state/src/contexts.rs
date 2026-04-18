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
    pub player_profile: Account<'info, PlayerProfile>,
    #[account(
        init,
        payer = authority,
        space = AUTHORIZED_VAULT_SPACE,
        seeds = [b"authorized_vault", authority.key().as_ref()],
        bump
    )]
    pub authorized_vault: Account<'info, AuthorizedVault>,
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
        init,
        payer = vault_signer,
        space = PLANET_STATE_SPACE,
        seeds = [b"planet_state", authority.key().as_ref(), &player_profile.planet_count.to_le_bytes()],
        bump
    )]
    pub planet_state: Account<'info, PlanetState>,
    /// CHECK: verified and initialized manually inside `create_planet_state`.
    #[account(mut)]
    pub planet_coords: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
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
    #[account(
        seeds = [b"authorized_vault", planet_state.authority.as_ref()],
        bump = authorized_vault.bump,
    )]
    pub authorized_vault: Account<'info, AuthorizedVault>,
    #[account(mut)]
    pub planet_state: Account<'info, PlanetState>,
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
        seeds = [b"planet_state", authority.key().as_ref(), &source_planet.planet_index.to_le_bytes()],
        bump = source_planet.bump,
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
        seeds = [b"player_profile", new_authority.key().as_ref()],
        bump = new_player_profile.bump,
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
}
