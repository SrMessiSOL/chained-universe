use anchor_lang::prelude::*;
use anchor_lang::solana_program::instruction::{AccountMeta, Instruction};
use anchor_spl::token::{self, Mint, Token, TokenAccount, Transfer};

use crate::constants::{
    ANTIMATTER_SCALE, GAME_STATE_PROGRAM_ID, MARKET_FEE_BPS, MAX_OFFERS_PER_WALLET,
    PLANET_LISTING_ACCOUNT_SPACE, PLANET_LISTING_INDEX_ACCOUNT_SPACE,
    TRANSFER_PLANET_FROM_MARKET_DISCRIMINATOR,
};
use crate::error::MarketError;
use crate::state::{
    MarketConfig, PlanetListing, PlanetListingIndex, PlanetMarketObligation, SellerCounter,
};
use crate::utils::require_protocol_antimatter_treasury;

const PLANET_STATE_AUTHORITY_OFFSET: usize = 8;
const PLANET_STATE_INDEX_OFFSET: usize = 72;
const PLANET_STATE_ACTIVE_MISSIONS_OFFSET: usize = 414;
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
    require_keys_eq!(
        authority,
        expected_authority,
        MarketError::InvalidSellerPlanet
    );
    require!(
        data.len() >= PLANET_STATE_INDEX_OFFSET + 4,
        MarketError::InvalidSellerPlanet
    );
    let mut index_bytes = [0u8; 4];
    index_bytes.copy_from_slice(&data[PLANET_STATE_INDEX_OFFSET..PLANET_STATE_INDEX_OFFSET + 4]);
    let planet_index = u32::from_le_bytes(index_bytes);
    require!(planet_index > 0, MarketError::HomeworldNotSellable);
    require!(
        data.len() > PLANET_STATE_ACTIVE_MISSIONS_OFFSET,
        MarketError::InvalidSellerPlanet
    );
    require!(
        data[PLANET_STATE_ACTIVE_MISSIONS_OFFSET] == 0,
        MarketError::PlanetHasActiveMissions
    );
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

fn deactivate_listing_index(
    listing_index: &AccountInfo,
    expected_listing: Pubkey,
    expected_planet: Pubkey,
) -> Result<()> {
    if *listing_index.owner != crate::ID {
        return Ok(());
    }

    let mut data = listing_index.try_borrow_mut_data()?;
    let mut data_ref: &[u8] = &data;
    let mut index = PlanetListingIndex::try_deserialize(&mut data_ref)?;
    require_keys_eq!(
        index.listing,
        expected_listing,
        MarketError::InvalidSellerPlanet
    );
    require_keys_eq!(
        index.planet,
        expected_planet,
        MarketError::InvalidSellerPlanet
    );
    index.active = false;
    let mut output: &mut [u8] = &mut data;
    index.try_serialize(&mut output)?;
    Ok(())
}

fn require_no_market_obligations(
    obligation: &AccountInfo,
    expected_planet: Pubkey,
) -> Result<()> {
    if *obligation.owner != crate::ID {
        return Ok(());
    }

    let data = obligation.try_borrow_data()?;
    let mut data_ref: &[u8] = &data;
    let obligation_state = PlanetMarketObligation::try_deserialize(&mut data_ref)?;
    require_keys_eq!(
        obligation_state.planet,
        expected_planet,
        MarketError::InvalidSellerPlanet
    );
    require!(
        obligation_state.active_resource_offers == 0,
        MarketError::PlanetHasActiveMarketOffers
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
    #[account(init_if_needed, payer = seller, space = PLANET_LISTING_INDEX_ACCOUNT_SPACE, seeds = [b"planet_listing_index", planet.key().as_ref()], bump)]
    pub listing_index: Account<'info, PlanetListingIndex>,
    /// CHECK: optional per-planet market obligation counter. Missing accounts mean no tracked obligations.
    #[account(seeds = [b"planet_market_obligation", planet.key().as_ref()], bump)]
    pub market_obligation: UncheckedAccount<'info>,
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
    /// CHECK: optional backward-compatible per-planet listing index.
    #[account(mut, seeds = [b"planet_listing_index", listing.planet.as_ref()], bump)]
    pub listing_index: UncheckedAccount<'info>,
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

    /// CHECK: optional backward-compatible per-planet listing index.
    #[account(mut, seeds = [b"planet_listing_index", listing.planet.as_ref()], bump)]
    pub listing_index: UncheckedAccount<'info>,

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

    /// CHECK: buyer profile is validated and updated by the game-state CPI.
    #[account(mut)]
    pub buyer_profile: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,
}

pub fn create_planet_listing(
    ctx: Context<CreatePlanetListing>,
    price_antimatter: u64,
) -> Result<()> {
    require!(
        price_antimatter >= ANTIMATTER_SCALE,
        MarketError::PriceTooLow
    );
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
    require!(
        !ctx.accounts.listing_index.active,
        MarketError::PlanetAlreadyListed
    );
    require_no_market_obligations(
        &ctx.accounts.market_obligation.to_account_info(),
        ctx.accounts.planet.key(),
    )?;

    let counter = &mut ctx.accounts.seller_counter;
    let listing_id = counter.next_offer_id;

    if counter.seller == Pubkey::default() {
        counter.seller = ctx.accounts.seller.key();
        counter.bump = ctx.bumps.seller_counter;
    } else {
        require_keys_eq!(
            counter.seller,
            ctx.accounts.seller.key(),
            MarketError::Unauthorized
        );
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
    ctx.accounts.listing_index.set_inner(PlanetListingIndex {
        planet: ctx.accounts.planet.key(),
        listing: ctx.accounts.listing.key(),
        seller: ctx.accounts.seller.key(),
        active: true,
        bump: ctx.bumps.listing_index,
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
    require_keys_eq!(
        ctx.accounts.seller_counter.seller,
        ctx.accounts.seller.key(),
        MarketError::Unauthorized
    );
    ctx.accounts.seller_counter.active_offers =
        ctx.accounts.seller_counter.active_offers.saturating_sub(1);
    deactivate_listing_index(
        &ctx.accounts.listing_index.to_account_info(),
        ctx.accounts.listing.key(),
        ctx.accounts.listing.planet,
    )?;
    msg!(
        "Planet listing cancelled: listing_id={}",
        ctx.accounts.listing.listing_id
    );
    Ok(())
}

pub fn buy_planet_listing<'info>(
    ctx: Context<'_, '_, '_, 'info, BuyPlanetListing<'info>>,
) -> Result<()> {
    require!(!ctx.accounts.listing.filled, MarketError::AlreadyFilled);
    require_keys_eq!(
        ctx.accounts.seller_counter.seller,
        ctx.accounts.listing.seller,
        MarketError::Unauthorized
    );
    require_keys_neq!(
        ctx.accounts.buyer.key(),
        ctx.accounts.listing.seller,
        MarketError::Unauthorized
    );
    if *ctx.accounts.listing_index.to_account_info().owner == crate::ID {
        let listing_index_data = ctx.accounts.listing_index.try_borrow_data()?;
        let mut data_ref: &[u8] = &listing_index_data;
        let listing_index = PlanetListingIndex::try_deserialize(&mut data_ref)?;
        require!(listing_index.active, MarketError::AlreadyFilled);
        require_keys_eq!(
            listing_index.listing,
            ctx.accounts.listing.key(),
            MarketError::InvalidSellerPlanet
        );
        require_keys_eq!(
            listing_index.planet,
            ctx.accounts.listing.planet,
            MarketError::InvalidSellerPlanet
        );
    }
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

    let mut transfer_accounts = vec![
        AccountMeta::new_readonly(ctx.accounts.seller.key(), false),
        AccountMeta::new(ctx.accounts.buyer.key(), true),
        AccountMeta::new(ctx.accounts.buyer_profile.key(), false),
        AccountMeta::new(ctx.accounts.planet.key(), false),
        AccountMeta::new(ctx.accounts.planet_coords.key(), false),
        AccountMeta::new_readonly(ctx.accounts.market_escrow_authority.key(), true),
        AccountMeta::new_readonly(ctx.accounts.system_program.key(), false),
    ];
    transfer_accounts.extend(ctx.remaining_accounts.iter().map(|account| {
        if account.is_writable {
            AccountMeta::new(account.key(), account.is_signer)
        } else {
            AccountMeta::new_readonly(account.key(), account.is_signer)
        }
    }));

    let transfer_ix = Instruction {
        program_id: ctx.accounts.game_program.key(),
        accounts: transfer_accounts,
        data: TRANSFER_PLANET_FROM_MARKET_DISCRIMINATOR.to_vec(),
    };

    let mut transfer_infos: Vec<AccountInfo<'info>> = vec![
        ctx.accounts.seller.to_account_info(),
        ctx.accounts.buyer.to_account_info(),
        ctx.accounts.buyer_profile.to_account_info(),
        ctx.accounts.planet.to_account_info(),
        ctx.accounts.planet_coords.to_account_info(),
        ctx.accounts.market_escrow_authority.to_account_info(),
        ctx.accounts.system_program.to_account_info(),
        ctx.accounts.game_program.to_account_info(),
    ];
    transfer_infos.extend(ctx.remaining_accounts.iter().cloned());

    anchor_lang::solana_program::program::invoke_signed(
        &transfer_ix,
        &transfer_infos,
        authority_seeds,
    )?;

    ctx.accounts.listing.filled = true;
    ctx.accounts.seller_counter.active_offers =
        ctx.accounts.seller_counter.active_offers.saturating_sub(1);
    deactivate_listing_index(
        &ctx.accounts.listing_index.to_account_info(),
        ctx.accounts.listing.key(),
        ctx.accounts.listing.planet,
    )?;
    ctx.accounts.market_config.total_volume = ctx
        .accounts
        .market_config
        .total_volume
        .saturating_add(price as u128);

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
