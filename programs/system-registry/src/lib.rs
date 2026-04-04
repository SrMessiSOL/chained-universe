use anchor_lang::prelude::*;

declare_id!("BV6JwMdA9gLfG5ut2VBzbmQoJTXUu5umXErBqv4V3PJq");

#[program]
pub mod system_registry {
    use super::*;

    pub fn init_wallet_meta(ctx: Context<InitWalletMeta>) -> Result<()> {
        let meta = &mut ctx.accounts.wallet_meta;
        let wallet = ctx.accounts.wallet.key();

        if meta.wallet == Pubkey::default() {
            meta.wallet = wallet;
            meta.planet_count = 0;
        } else {
            require_keys_eq!(meta.wallet, wallet, RegistryError::WalletMismatch);
        }

        Ok(())
    }

    pub fn register_planet(
        ctx: Context<RegisterPlanet>,
        entity_pda: Pubkey,
        planet_pda: Pubkey,
        galaxy: u16,
        system: u16,
        position: u8,
    ) -> Result<()> {
        let wallet = ctx.accounts.wallet.key();
        let wallet_meta = &mut ctx.accounts.wallet_meta;

        require_keys_eq!(wallet_meta.wallet, wallet, RegistryError::WalletMismatch);
        require!((1..=9).contains(&galaxy), RegistryError::InvalidGalaxy);
        require!((1..=499).contains(&system), RegistryError::InvalidSystem);
        require!((1..=15).contains(&position), RegistryError::InvalidPosition);

        let index = wallet_meta.planet_count;

        let registry = &mut ctx.accounts.registry;
        registry.wallet = wallet;
        registry.planet_index = index;
        registry.entity_pda = entity_pda;
        registry.planet_pda = planet_pda;
        registry.created_at = Clock::get()?.unix_timestamp;

        let coord = &mut ctx.accounts.coord;
        coord.galaxy = galaxy;
        coord.system = system;
        coord.position = position;
        coord.owner_wallet = wallet;
        coord.entity_pda = entity_pda;
        coord.planet_pda = planet_pda;

        wallet_meta.planet_count = wallet_meta
            .planet_count
            .checked_add(1)
            .ok_or(RegistryError::PlanetLimitReached)?;

        msg!("Planet registered at index {} | PDA: {}", index, planet_pda);
        Ok(())
    }
}

#[derive(Accounts)]
pub struct InitWalletMeta<'info> {
    #[account(mut)]
    pub wallet: Signer<'info>,

    #[account(
        init_if_needed,
        payer = wallet,
        space = WalletMeta::SIZE,
        seeds = [b"wallet_meta", wallet.key().as_ref()],
        bump,
    )]
    pub wallet_meta: Account<'info, WalletMeta>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(entity_pda: Pubkey, planet_pda: Pubkey, galaxy: u16, system: u16, position: u8)]
pub struct RegisterPlanet<'info> {
    #[account(mut)]
    pub wallet: Signer<'info>,

    #[account(
        mut,
        seeds = [b"wallet_meta", wallet.key().as_ref()],
        bump,
    )]
    pub wallet_meta: Account<'info, WalletMeta>,

    #[account(
        init,
        payer = wallet,
        space = PlanetRegistryEntry::SIZE,
        seeds = [
            b"registry",
            wallet.key().as_ref(),
            wallet_meta.planet_count.to_le_bytes().as_ref(),
            ],
        bump,
    )]
    pub registry: Account<'info, PlanetRegistryEntry>,

    #[account(
        init,
        payer = wallet,
        space = CoordinateRegistry::SIZE,
        seeds = [b"coord", galaxy.to_le_bytes().as_ref(), system.to_le_bytes().as_ref(), &[position]],
        bump,
    )]
    pub coord: Account<'info, CoordinateRegistry>,

    pub system_program: Program<'info, System>,
}

#[account]
pub struct WalletMeta {
    pub wallet: Pubkey,
    pub planet_count: u32,
}

impl WalletMeta {
    pub const SIZE: usize = 8 + 32 + 4;
}

#[account]
pub struct PlanetRegistryEntry {
    pub wallet: Pubkey,
    pub planet_index: u32,
    pub entity_pda: Pubkey,
    pub planet_pda: Pubkey,
    pub created_at: i64,
}

impl PlanetRegistryEntry {
    pub const SIZE: usize = 8 + 32 + 4 + 32 + 32 + 8;
}


#[account]
pub struct CoordinateRegistry {
    pub galaxy: u16,
    pub system: u16,
    pub position: u8,
    pub owner_wallet: Pubkey,
    pub entity_pda: Pubkey,
    pub planet_pda: Pubkey,
}

impl CoordinateRegistry {
    pub const SIZE: usize = 8 + 2 + 2 + 1 + 32 + 32 + 32;
}

#[error_code]
pub enum RegistryError {
    #[msg("Wallet meta does not belong to the signer")]
    WalletMismatch,
    #[msg("Galaxy must be between 1 and 9")]
    InvalidGalaxy,
    #[msg("System must be between 1 and 499")]
    InvalidSystem,
    #[msg("Position must be between 1 and 15")]
    InvalidPosition,
    #[msg("Planet limit reached for this wallet")]
    PlanetLimitReached,
}
