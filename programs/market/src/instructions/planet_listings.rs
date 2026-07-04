use anchor_lang::prelude::*;
use anchor_lang::solana_program::instruction::{AccountMeta, Instruction};
use anchor_spl::token::{self, Mint, Token, TokenAccount, Transfer};

use crate::constants::{
    ANTIMATTER_SCALE, GAME_STATE_PROGRAM_ID, MARKET_FEE_BPS, MAX_OFFERS_PER_WALLET,
    PLANET_LISTING_ACCOUNT_SPACE, TRANSFER_PLANET_FROM_MARKET_DISCRIMINATOR,
};
use crate::error::MarketError;
use crate::state::{MarketConfig, PlanetListing, SellerCounter};
use crate::utils::require_protocol_antimatter_treasury;

const PLANET_STATE_AUTHORITY_OFFSET: usize = 8;
const PLANET_STATE_INDEX_OFFSET: usize = 72;
const PLANET_COORDS_GALAXY_OFFSET: usize = 8;
const PLANET_COORDS_SYSTEM_OFFSET: usize = 10;
const PLANET_COORDS_POSITION_OFFSET: usize = 12;
const PLANET_COORDS_PLANET_OFFSET: usize = 13;
const PLANET_COORDS_AUTHORITY_OFFSET: usize = 45;

fn read_pubkey_at(data: &[u8], offset: usize) -> Result<Pubkey> {
    require!(data.len() >= offset + 32, MarketError::InvalidSellerPlanet);
    let mut bytes = [0u8; 32];
    bytes.copy_from_slice(&data[offset..offset + 32]);
    Ok(Pubkey::new_from_array(bytes))
}

fn validate_planet_authority(planet: &AccountInfo, expected_authority: Pubkey) -> Result<()> {
    let data = planet.try_borrow_data()?;
    let authority = read_pubkey_at(&data, PLANET_STATE_AUTHORITY_OFFSET)?;
    require_keys_eq!(authority, expected_authority, MarketError::InvalidSellerPlanet);
    require!(
        data.len() >= PLANET_STATE_INDEX_OFFSET + 4,
        MarketError::InvalidSellerPlanet
    );
    let mut index_bytes = [0u8; 4];
    index_bytes.copy_from_slice(&data[PLANET_STATE_INDEX_OFFSET..PLANET_STATE_INDEX_OFFSET + 4]);
    let planet_index = u32::from_le_bytes(index_bytes);
    require!(planet_index > 0, MarketError::HomeworldNotSellable);
    Ok(())
}

fn validate_planet_coords(
    planet_coords: &AccountInfo,
    expected_planet: Pubkey,
    expected_authority: Pubkey,
) -> Result<()> {
    require_keys_eq!(
        *planet_coords.owner,
        GAME_STATE_PROGRAM_ID,
        MarketError::InvalidSellerPlanet
    );
    let data = planet_coords.try_borrow_data()?;
    require!(
        data.len() >= PLANET_COORDS_AUTHORITY_OFFSET + 32,
        MarketError::InvalidSellerPlanet
    );

    let planet = read_pubkey_at(&data, PLANET_COORDS_PLANET_OFFSET)?;
    let authority = read_pubkey_at(&data, PLANET_COORDS_AUTHORITY_OFFSET)?;
    require_keys_eq!(planet, expected_planet, MarketError::InvalidSellerPlanet);
    require_keys_eq!(
        authority,
        expected_authority,
        MarketError::InvalidSellerPlanet
    );

    let galaxy = u16::from_le_bytes([
        data[PLANET_COORDS_GALAXY_OFFSET],
        data[PLANET_COORDS_GALAXY_OFFSET + 1],
    ]);
    let system = u16::from_le_bytes([
        data[PLANET_COORDS_SYSTEM_OFFSET],
        data[PLANET_COORDS_SYSTEM_OFFSET + 1],
    ]);
    let position = data[PLANET_COORDS_POSITION_OFFSET];
    let (expected_coords_pda, _) = Pubkey::find_program_address(
        &[
            b"planet_coords",
            &galaxy.to_le_bytes(),
            &system.to_le_bytes(),
            &[position],
        ],
        &GAME_STATE_PROGRAM_ID,
    );
    require_keys_eq!(
        planet_coords.key(),
        expected_coords_pda,
        MarketError::InvalidSellerPlanet
    );
    Ok(())
}

#[derive(Accounts)]
pub struct CreatePlanetListing<'info> {
    #[account(mut)]
    pub seller: Signer<'info>,
    #[account(mut, seeds = [b"market_config"], bump = market_config.bump)]
    pub market_config: Account<'info, MarketConfig>,
    #[account(init_if_needed, payer = seller, space = 8 + SellerCounter::INIT_SPACE, seeds = [b"seller_counter", seller.key().as_ref()], bump)]
    pub seller_counter: Account<'info, SellerCounter>,
    #[account(init, payer = seller, space = PLANET_LISTING_ACCOUNT_SPACE, seeds = [b"planet_listing", seller.key().as_ref(), &seller_counter.next_offer_id.to_le_bytes()], bump)]
    pub listing: Account<'info, PlanetListing>,
    /// CHECK: game-state planet account is constrained by owner and validated by raw state fields.
    #[account(mut, owner = GAME_STATE_PROGRAM_ID)]
    pub planet: UncheckedAccount<'info>,
    /// CHECK: game-state coords account is constrained by owner and validated against the planet PDA.
    #[account(mut, owner = GAME_STATE_PROGRAM_ID)]
    pub planet_coords: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct CancelPlanetListing<'info> {
    #[account(mut)]
    pub seller: Signer<'info>,
    #[account(mut, seeds = [b"planet_listing", seller.key().as_ref(), &listing.listing_id.to_le_bytes()], bump = listing.bump, has_one = seller @ MarketError::Unauthorized, close = seller)]
    pub listing: Account<'info, PlanetListing>,
    #[account(mut, seeds = [b"seller_counter", seller.key().as_ref()], bump = seller_counter.bump)]
    pub seller_counter: Account<'info, SellerCounter>,
}

#[derive(Accounts)]
pub struct BuyPlanetListing<'info> {
    #[account(mut)]
    pub buyer: Signer<'info>,

    #[account(mut, address = listing.seller @ MarketError::InvalidSeller)]
    pub seller: SystemAccount<'info>,

    #[account(mut, seeds = [b"market_config"], bump = market_config.bump)]
    pub market_config: Account<'info, MarketConfig>,

    #[account(mut, seeds = [b"planet_listing", listing.seller.as_ref(), &listing.listing_id.to_le_bytes()], bump = listing.bump, close = seller)]
    pub listing: Account<'info, PlanetListing>,

    #[account(mut, seeds = [b"seller_counter", listing.seller.as_ref()], bump = seller_counter.bump)]
    pub seller_counter: Account<'info, SellerCounter>,

    #[account(mut, address = market_config.antimatter_mint)]
    pub antimatter_mint: Account<'info, Mint>,

    #[account(mut, token::mint = antimatter_mint, token::authority = buyer)]
    pub buyer_antimatter_account: Account<'info, TokenAccount>,

    #[account(mut, token::mint = antimatter_mint, token::authority = seller)]
    pub seller_antimatter_account: Account<'info, TokenAccount>,

    #[account(mut, seeds = [b"market_escrow"], bump, token::mint = antimatter_mint, token::authority = market_escrow_authority)]
    pub market_escrow: Account<'info, TokenAccount>,

    #[account(
        mut,
        token::mint = antimatter_mint,
        token::authority = market_config.admin
    )]
    pub treasury_antimatter_account: Account<'info, TokenAccount>,

    /// CHECK: PDA authority is constrained by its fixed market_authority seeds.
    #[account(seeds = [b"market_authority"], bump)]
    pub market_escrow_authority: UncheckedAccount<'info>,

    pub token_program: Program<'info, Token>,

    /// CHECK: constrained to the configured game-state program id.
    #[account(address = GAME_STATE_PROGRAM_ID)]
    pub game_program: UncheckedAccount<'info>,

    /// CHECK: game-state planet account is constrained by address, owner, and validated by raw state fields.
    #[account(mut, address = listing.planet @ MarketError::InvalidSellerPlanet, owner = GAME_STATE_PROGRAM_ID)]
    pub planet: UncheckedAccount<'info>,

    /// CHECK: game-state coords account is constrained by address, owner, and validated against the planet PDA.
    #[account(mut, address = listing.planet_coords @ MarketError::InvalidSellerPlanet, owner = GAME_STATE_PROGRAM_ID)]
    pub planet_coords: UncheckedAccount<'info>,

    /// CHECK: buyer profile is validated by the game-state CPI.
    pub buyer_profile: UncheckedAccount<'info>,
}

pub fn create_planet_listing(ctx: Context<CreatePlanetListing>, price_antimatter: u64) -> Result<()> {
    require!(price_antimatter >= ANTIMATTER_SCALE, MarketError::PriceTooLow);
    require!(
        ctx.accounts.seller_counter.active_offers < MAX_OFFERS_PER_WALLET,
        MarketError::TooManyOffers,
    );
    validate_planet_authority(
        &ctx.accounts.planet.to_account_info(),
        ctx.accounts.seller.key(),
    )?;
    validate_planet_coords(
        &ctx.accounts.planet_coords.to_account_info(),
        ctx.accounts.planet.key(),
        ctx.accounts.seller.key(),
    )?;

    let counter = &mut ctx.accounts.seller_counter;
    let listing_id = counter.next_offer_id;

    if counter.seller == Pubkey::default() {
        counter.seller = ctx.accounts.seller.key();
        counter.bump = ctx.bumps.seller_counter;
    }

    counter.next_offer_id = counter.next_offer_id.saturating_add(1);
    counter.active_offers = counter.active_offers.saturating_add(1);

    ctx.accounts.listing.set_inner(PlanetListing {
        seller: ctx.accounts.seller.key(),
        planet: ctx.accounts.planet.key(),
        planet_coords: ctx.accounts.planet_coords.key(),
        price_antimatter,
        created_at: Clock::get()?.unix_timestamp,
        listing_id,
        filled: false,
        bump: ctx.bumps.listing,
    });
    ctx.accounts.market_config.total_offers =
        ctx.accounts.market_config.total_offers.saturating_add(1);

    msg!(
        "Planet listed: seller={} planet={} price={}",
        ctx.accounts.seller.key(),
        ctx.accounts.planet.key(),
        price_antimatter
    );
    Ok(())
}

pub fn cancel_planet_listing(ctx: Context<CancelPlanetListing>) -> Result<()> {
    require!(!ctx.accounts.listing.filled, MarketError::AlreadyFilled);
    ctx.accounts.seller_counter.active_offers =
        ctx.accounts.seller_counter.active_offers.saturating_sub(1);
    msg!("Planet listing cancelled: listing_id={}", ctx.accounts.listing.listing_id);
    Ok(())
}

pub fn buy_planet_listing(ctx: Context<BuyPlanetListing>) -> Result<()> {
    require!(!ctx.accounts.listing.filled, MarketError::AlreadyFilled);
    require_keys_neq!(ctx.accounts.buyer.key(), ctx.accounts.listing.seller, MarketError::Unauthorized);
    validate_planet_authority(
        &ctx.accounts.planet.to_account_info(),
        ctx.accounts.listing.seller,
    )?;
    validate_planet_coords(
        &ctx.accounts.planet_coords.to_account_info(),
        ctx.accounts.listing.planet,
        ctx.accounts.listing.seller,
    )?;

    let price = ctx.accounts.listing.price_antimatter;
    let authority_seeds: &[&[&[u8]]] =
        &[&[b"market_authority", &[ctx.bumps.market_escrow_authority]]];
    let fee = if MARKET_FEE_BPS > 0 {
        price.saturating_mul(MARKET_FEE_BPS) / 10_000
    } else {
        0
    };
    let seller_receives = price.saturating_sub(fee);

    require!(
        ctx.accounts.buyer_antimatter_account.amount >= price,
        MarketError::InsufficientAntimatter,
    );

    token::transfer(
        CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.buyer_antimatter_account.to_account_info(),
                to: ctx.accounts.market_escrow.to_account_info(),
                authority: ctx.accounts.buyer.to_account_info(),
            },
        ),
        price,
    )?;

    token::transfer(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.market_escrow.to_account_info(),
                to: ctx.accounts.seller_antimatter_account.to_account_info(),
                authority: ctx.accounts.market_escrow_authority.to_account_info(),
            },
            authority_seeds,
        ),
        seller_receives,
    )?;

    if fee > 0 {
        require_protocol_antimatter_treasury(
            ctx.accounts.treasury_antimatter_account.key(),
            ctx.accounts.market_config.admin,
            ctx.accounts.antimatter_mint.key(),
        )?;
        token::transfer(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.market_escrow.to_account_info(),
                    to: ctx.accounts.treasury_antimatter_account.to_account_info(),
                    authority: ctx.accounts.market_escrow_authority.to_account_info(),
                },
                authority_seeds,
            ),
            fee,
        )?;
    }

    let transfer_ix = Instruction {
        program_id: ctx.accounts.game_program.key(),
        accounts: vec![
            AccountMeta::new_readonly(ctx.accounts.seller.key(), false),
            AccountMeta::new_readonly(ctx.accounts.buyer.key(), false),
            AccountMeta::new_readonly(ctx.accounts.buyer_profile.key(), false),
            AccountMeta::new(ctx.accounts.planet.key(), false),
            AccountMeta::new(ctx.accounts.planet_coords.key(), false),
            AccountMeta::new_readonly(ctx.accounts.market_escrow_authority.key(), true),
        ],
        data: TRANSFER_PLANET_FROM_MARKET_DISCRIMINATOR.to_vec(),
    };

    anchor_lang::solana_program::program::invoke_signed(
        &transfer_ix,
        &[
            ctx.accounts.seller.to_account_info(),
            ctx.accounts.buyer.to_account_info(),
            ctx.accounts.buyer_profile.to_account_info(),
            ctx.accounts.planet.to_account_info(),
            ctx.accounts.planet_coords.to_account_info(),
            ctx.accounts.market_escrow_authority.to_account_info(),
            ctx.accounts.game_program.to_account_info(),
        ],
        authority_seeds,
    )?;

    ctx.accounts.listing.filled = true;
    ctx.accounts.seller_counter.active_offers =
        ctx.accounts.seller_counter.active_offers.saturating_sub(1);
    ctx.accounts.market_config.total_volume =
        ctx.accounts.market_config.total_volume.saturating_add(price as u128);

    msg!(
        "Planet sold: listing_id={} buyer={} seller={} planet={} price={}",
        ctx.accounts.listing.listing_id,
        ctx.accounts.buyer.key(),
        ctx.accounts.listing.seller,
        ctx.accounts.listing.planet,
        price
    );
    Ok(())
}
