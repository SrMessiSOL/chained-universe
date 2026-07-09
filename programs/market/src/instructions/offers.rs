use anchor_lang::prelude::*;
use anchor_lang::solana_program::instruction::AccountMeta;
use anchor_spl::token::{self, Mint, Token, TokenAccount, Transfer};

use crate::constants::{
    ANTIMATTER_SCALE, GAME_STATE_PROGRAM_ID, LOCK_RESOURCES_FOR_MARKET_DISCRIMINATOR,
    MARKET_FEE_BPS, MAX_OFFERS_PER_WALLET, MIN_RESOURCE_AMOUNT, OFFER_ACCOUNT_SPACE,
    PLANET_MARKET_OBLIGATION_ACCOUNT_SPACE, RELEASE_RESOURCES_FROM_MARKET_DISCRIMINATOR,
    TRANSFER_RESOURCES_FROM_MARKET_DISCRIMINATOR,
};
use crate::error::MarketError;
use crate::state::{
    MarketConfig, MarketOffer, PlanetListingIndex, PlanetMarketObligation, SellerCounter,
};
use crate::types::ResourceType;
use crate::utils::{build_market_resource_ix, require_protocol_antimatter_treasury};

fn decrement_market_obligation(
    obligation: &AccountInfo,
    expected_planet: Pubkey,
) -> Result<()> {
    if *obligation.owner != crate::ID {
        return Ok(());
    }

    let mut data = obligation.try_borrow_mut_data()?;
    let mut data_ref: &[u8] = &data;
    let mut obligation_state = PlanetMarketObligation::try_deserialize(&mut data_ref)?;
    require_keys_eq!(
        obligation_state.planet,
        expected_planet,
        MarketError::InvalidSellerPlanet
    );
    obligation_state.active_resource_offers =
        obligation_state.active_resource_offers.saturating_sub(1);
    let mut output: &mut [u8] = &mut data;
    obligation_state.try_serialize(&mut output)?;
    Ok(())
}

fn require_planet_not_listed(listing_index: &AccountInfo, expected_planet: Pubkey) -> Result<()> {
    if *listing_index.owner != crate::ID {
        return Ok(());
    }

    let data = listing_index.try_borrow_data()?;
    let mut data_ref: &[u8] = &data;
    let index = PlanetListingIndex::try_deserialize(&mut data_ref)?;
    require_keys_eq!(
        index.planet,
        expected_planet,
        MarketError::InvalidSellerPlanet
    );
    require!(!index.active, MarketError::PlanetAlreadyListed);
    Ok(())
}

#[derive(Accounts)]
pub struct CreateOffer<'info> {
    #[account(mut)]
    pub seller: Signer<'info>,
    #[account(mut, seeds = [b"market_config"], bump = market_config.bump)]
    pub market_config: Account<'info, MarketConfig>,
    #[account(init_if_needed, payer = seller, space = 8 + SellerCounter::INIT_SPACE, seeds = [b"seller_counter", seller.key().as_ref()], bump)]
    pub seller_counter: Account<'info, SellerCounter>,
    #[account(init, payer = seller, space = OFFER_ACCOUNT_SPACE, seeds = [b"market_offer", seller.key().as_ref(), &seller_counter.next_offer_id.to_le_bytes()], bump)]
    pub offer: Account<'info, MarketOffer>,
    #[account(init_if_needed, payer = seller, space = PLANET_MARKET_OBLIGATION_ACCOUNT_SPACE, seeds = [b"planet_market_obligation", seller_planet.key().as_ref()], bump)]
    pub market_obligation: Account<'info, PlanetMarketObligation>,
    /// CHECK: optional per-planet listing index. Missing accounts mean no tracked listing.
    #[account(seeds = [b"planet_listing_index", seller_planet.key().as_ref()], bump)]
    pub listing_index: UncheckedAccount<'info>,
    /// CHECK: constrained to the configured game-state program id.
    #[account(address = GAME_STATE_PROGRAM_ID)]
    pub game_program: UncheckedAccount<'info>,
    /// CHECK: game-state planet account is constrained by owner and used by game-state CPI.
    #[account(mut, owner = GAME_STATE_PROGRAM_ID)]
    pub seller_planet: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct CancelOffer<'info> {
    #[account(mut)]
    pub seller: Signer<'info>,
    #[account(mut, seeds = [b"market_offer", seller.key().as_ref(), &offer.offer_id.to_le_bytes()], bump = offer.bump, has_one = seller @ MarketError::Unauthorized, close = seller)]
    pub offer: Account<'info, MarketOffer>,
    #[account(mut, seeds = [b"seller_counter", seller.key().as_ref()], bump = seller_counter.bump)]
    pub seller_counter: Account<'info, SellerCounter>,
    /// CHECK: constrained to the configured game-state program id.
    #[account(address = GAME_STATE_PROGRAM_ID)]
    pub game_program: UncheckedAccount<'info>,
    /// CHECK: game-state planet account is constrained by address, owner, and used by game-state CPI.
    #[account(mut, address = offer.seller_planet @ MarketError::InvalidSellerPlanet, owner = GAME_STATE_PROGRAM_ID)]
    pub seller_planet: UncheckedAccount<'info>,
    /// CHECK: optional backward-compatible per-planet market obligation counter.
    #[account(mut, seeds = [b"planet_market_obligation", offer.seller_planet.as_ref()], bump)]
    pub market_obligation: UncheckedAccount<'info>,
    /// CHECK: PDA authority is constrained by its fixed market_authority seeds.
    #[account(seeds = [b"market_authority"], bump)]
    pub market_authority: UncheckedAccount<'info>,
}

#[derive(Accounts)]
pub struct AcceptOffer<'info> {
    #[account(mut)]
    pub buyer: Signer<'info>,

    #[account(mut, address = offer.seller @ MarketError::InvalidSeller)]
    pub seller: SystemAccount<'info>,

    #[account(mut, seeds = [b"market_config"], bump = market_config.bump)]
    pub market_config: Account<'info, MarketConfig>,

    #[account(
        mut,
        seeds = [b"market_offer", offer.seller.as_ref(), &offer.offer_id.to_le_bytes()],
        bump = offer.bump,
        close = seller,
    )]
    pub offer: Account<'info, MarketOffer>,

    #[account(mut, seeds = [b"seller_counter", offer.seller.as_ref()], bump = seller_counter.bump)]
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
    pub system_program: Program<'info, System>,

    /// CHECK: constrained to the configured game-state program id.
    #[account(address = GAME_STATE_PROGRAM_ID)]
    pub game_program: UncheckedAccount<'info>,

    /// CHECK: game-state planet account is constrained by address, owner, and used by game-state CPI.
    #[account(mut, address = offer.seller_planet @ MarketError::InvalidSellerPlanet, owner = GAME_STATE_PROGRAM_ID)]
    pub seller_planet: UncheckedAccount<'info>,
    /// CHECK: optional backward-compatible per-planet market obligation counter.
    #[account(mut, seeds = [b"planet_market_obligation", offer.seller_planet.as_ref()], bump)]
    pub market_obligation: UncheckedAccount<'info>,

    /// CHECK: game-state planet account is constrained by owner and used by game-state CPI.
    #[account(mut, owner = GAME_STATE_PROGRAM_ID)]
    pub buyer_planet: UncheckedAccount<'info>,
}

pub fn create_offer(
    ctx: Context<CreateOffer>,
    resource_type: ResourceType,
    resource_amount: u64,
    price_antimatter: u64,
) -> Result<()> {
    require!(resource_amount >= MIN_RESOURCE_AMOUNT, MarketError::AmountTooSmall);
    require!(price_antimatter >= ANTIMATTER_SCALE, MarketError::PriceTooLow);
    require!(
        ctx.accounts.seller_counter.active_offers < MAX_OFFERS_PER_WALLET,
        MarketError::TooManyOffers,
    );
    require_planet_not_listed(
        &ctx.accounts.listing_index.to_account_info(),
        ctx.accounts.seller_planet.key(),
    )?;

    let counter = &mut ctx.accounts.seller_counter;
    let offer_id = counter.next_offer_id;

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

    let now = Clock::get()?.unix_timestamp;

    let lock_ix = build_market_resource_ix(
        LOCK_RESOURCES_FOR_MARKET_DISCRIMINATOR,
        ctx.accounts.game_program.key(),
        vec![
            AccountMeta::new(ctx.accounts.seller.key(), true),
            AccountMeta::new(ctx.accounts.seller_planet.key(), false),
        ],
        resource_type,
        resource_amount,
    );

    anchor_lang::solana_program::program::invoke(
        &lock_ix,
        &[
            ctx.accounts.seller.to_account_info(),
            ctx.accounts.seller_planet.to_account_info(),
            ctx.accounts.game_program.to_account_info(),
        ],
    )?;

    ctx.accounts.offer.set_inner(MarketOffer {
        seller: ctx.accounts.seller.key(),
        seller_planet: ctx.accounts.seller_planet.key(),
        resource_type,
        resource_amount,
        price_antimatter,
        created_at: now,
        offer_id,
        filled: false,
        bump: ctx.bumps.offer,
    });
    if ctx.accounts.market_obligation.planet == Pubkey::default() {
        ctx.accounts.market_obligation.planet = ctx.accounts.seller_planet.key();
        ctx.accounts.market_obligation.bump = ctx.bumps.market_obligation;
    } else {
        require_keys_eq!(
            ctx.accounts.market_obligation.planet,
            ctx.accounts.seller_planet.key(),
            MarketError::InvalidSellerPlanet
        );
    }
    ctx.accounts.market_obligation.active_resource_offers = ctx
        .accounts
        .market_obligation
        .active_resource_offers
        .saturating_add(1);

    ctx.accounts.market_config.total_offers =
        ctx.accounts.market_config.total_offers.saturating_add(1);

    msg!(
        "Offer created: seller={} type={} amount={} price={}",
        ctx.accounts.seller.key(),
        resource_type.as_str(),
        resource_amount,
        price_antimatter,
    );

    Ok(())
}

pub fn cancel_offer(ctx: Context<CancelOffer>) -> Result<()> {
    require!(!ctx.accounts.offer.filled, MarketError::AlreadyFilled);
    require_keys_eq!(
        ctx.accounts.seller_counter.seller,
        ctx.accounts.seller.key(),
        MarketError::Unauthorized
    );

    let authority_seeds: &[&[&[u8]]] =
        &[&[b"market_authority", &[ctx.bumps.market_authority]]];
    let release_ix = build_market_resource_ix(
        RELEASE_RESOURCES_FROM_MARKET_DISCRIMINATOR,
        ctx.accounts.game_program.key(),
        vec![
            AccountMeta::new(ctx.accounts.seller_planet.key(), false),
            AccountMeta::new_readonly(ctx.accounts.market_authority.key(), true),
        ],
        ctx.accounts.offer.resource_type,
        ctx.accounts.offer.resource_amount,
    );

    anchor_lang::solana_program::program::invoke_signed(
        &release_ix,
        &[
            ctx.accounts.seller_planet.to_account_info(),
            ctx.accounts.market_authority.to_account_info(),
            ctx.accounts.game_program.to_account_info(),
        ],
        authority_seeds,
    )?;

    ctx.accounts.seller_counter.active_offers =
        ctx.accounts.seller_counter.active_offers.saturating_sub(1);
    decrement_market_obligation(
        &ctx.accounts.market_obligation.to_account_info(),
        ctx.accounts.offer.seller_planet,
    )?;

    msg!("Offer cancelled: offer_id={}", ctx.accounts.offer.offer_id);
    Ok(())
}

pub fn accept_offer(ctx: Context<AcceptOffer>) -> Result<()> {
    require!(!ctx.accounts.offer.filled, MarketError::AlreadyFilled);
    require_keys_eq!(
        ctx.accounts.seller_counter.seller,
        ctx.accounts.offer.seller,
        MarketError::Unauthorized
    );
    require_keys_neq!(
        ctx.accounts.buyer.key(),
        ctx.accounts.offer.seller,
        MarketError::Unauthorized
    );

    let price = ctx.accounts.offer.price_antimatter;
    let resource_amount = ctx.accounts.offer.resource_amount;
    let resource_type = ctx.accounts.offer.resource_type;

    {
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

        let transfer_ix = build_market_resource_ix(
            TRANSFER_RESOURCES_FROM_MARKET_DISCRIMINATOR,
            ctx.accounts.game_program.key(),
            vec![
                AccountMeta::new(ctx.accounts.buyer_planet.key(), false),
                AccountMeta::new_readonly(ctx.accounts.market_escrow_authority.key(), true),
                AccountMeta::new(ctx.accounts.buyer.key(), true),
            ],
            resource_type,
            resource_amount,
        );

        anchor_lang::solana_program::program::invoke_signed(
            &transfer_ix,
            &[
                ctx.accounts.buyer_planet.to_account_info(),
                ctx.accounts.market_escrow_authority.to_account_info(),
                ctx.accounts.buyer.to_account_info(),
                ctx.accounts.game_program.to_account_info(),
            ],
            authority_seeds,
        )?;

        ctx.accounts.offer.filled = true;
        ctx.accounts.seller_counter.active_offers =
            ctx.accounts.seller_counter.active_offers.saturating_sub(1);
        decrement_market_obligation(
            &ctx.accounts.market_obligation.to_account_info(),
            ctx.accounts.offer.seller_planet,
        )?;
        ctx.accounts.market_config.total_volume =
            ctx.accounts.market_config.total_volume.saturating_add(price as u128);

        msg!(
            "Offer filled: offer_id={} buyer={} seller={} resource={} amount={} price={}",
            ctx.accounts.offer.offer_id,
            ctx.accounts.buyer.key(),
            ctx.accounts.offer.seller,
            resource_type.as_str(),
            resource_amount,
            price,
        );

        Ok(())
    }
}
