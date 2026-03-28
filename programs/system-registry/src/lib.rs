use anchor_lang::prelude::*;

declare_id!("N1K6B3oiseLvLrvXELjWPdPAuhPw8MjFo3oepnHd5d3");

#[program]
pub mod system_registry {
    use super::*;

    pub fn init_wallet_meta(ctx: Context<InitWalletMeta>) -> Result<()> {
        let meta = &mut ctx.accounts.wallet_meta;
        meta.wallet = ctx.accounts.wallet.key();
        meta.planet_count = 0;
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
        let index = ctx.accounts.wallet_meta.planet_count;
        require!(index == ctx.accounts.registry.planet_index, RegistryError::InvalidIndex);

        let wallet = ctx.accounts.wallet.key();

        ctx.accounts.registry.wallet = wallet;
        ctx.accounts.registry.planet_index = index;
        ctx.accounts.registry.entity_pda = entity_pda;
        ctx.accounts.registry.planet_pda = planet_pda;
        ctx.accounts.registry.created_at = Clock::get()?.unix_timestamp;

        ctx.accounts.coord.galaxy = galaxy;
        ctx.accounts.coord.system = system;
        ctx.accounts.coord.position = position;
        ctx.accounts.coord.owner_wallet = wallet;
        ctx.accounts.coord.entity_pda = entity_pda;
        ctx.accounts.coord.planet_pda = planet_pda;

        ctx.accounts.wallet_meta.planet_count = ctx.accounts.wallet_meta.planet_count.saturating_add(1);

        Ok(())
    }
}

#[derive(Accounts)]
pub struct InitWalletMeta<'info> {
    #[account(mut)]
    pub wallet: Signer<'info>,
    #[account(
        init,
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
        seeds = [b"registry", wallet.key().as_ref(), &[wallet_meta.planet_count]],
        bump,
    )]
    pub registry: Account<'info, PlanetRegistryEntry>,

    #[account(
        init,
        payer = wallet,
        space = CoordinateRegistry::SIZE,
        seeds = [b"coord".as_ref(), galaxy.to_le_bytes().as_ref(), system.to_le_bytes().as_ref(), [position].as_ref()],
        bump,
    )]
    pub coord: Account<'info, CoordinateRegistry>,

    pub system_program: Program<'info, System>,
}

#[account]
pub struct WalletMeta {
    pub wallet: Pubkey,
    pub planet_count: u8,
}

impl WalletMeta {
    pub const SIZE: usize = 8 + 32 + 1;
}

#[account]
pub struct PlanetRegistryEntry {
    pub wallet: Pubkey,
    pub planet_index: u8,
    pub entity_pda: Pubkey,
    pub planet_pda: Pubkey,
    pub created_at: i64,
}

impl PlanetRegistryEntry {
    pub const SIZE: usize = 8 + 32 + 1 + 32 + 32 + 8;
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
    #[msg("Invalid registry index")]
    InvalidIndex,
}
