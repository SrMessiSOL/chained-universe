use anchor_lang::prelude::*;
use anchor_spl::token::{self, Burn, Mint, Token, TokenAccount, Transfer};

declare_id!("E6ubJUSv2eqJE93HHma7WAiMrikkUxkBmEkqELvVb8j3");

// ─── Constants ────────────────────────────────────────────────────────────────
pub const ANTIMATTER_DECIMALS: u8 = 6;
pub const ANTIMATTER_SCALE: u64 = 1_000_000;
pub const MAX_OFFERS_PER_WALLET: u32 = 20;
pub const MIN_RESOURCE_AMOUNT: u64 = 1_000;
pub const MARKET_FEE_BPS: u64 = 25;
pub const OFFER_ACCOUNT_SPACE: usize = 8 + MarketOffer::INIT_SPACE;
pub const MARKET_CONFIG_SPACE: usize = 8 + MarketConfig::INIT_SPACE;
pub const LOCK_RESOURCES_FOR_MARKET_DISCRIMINATOR: [u8; 8] =
    [0x77, 0x52, 0x53, 0xd9, 0x39, 0x6e, 0xc9, 0x8b];
pub const RELEASE_RESOURCES_FROM_MARKET_DISCRIMINATOR: [u8; 8] =
    [0xd7, 0x8f, 0xe2, 0xee, 0x0c, 0x56, 0x12, 0x7c];
pub const TRANSFER_RESOURCES_FROM_MARKET_DISCRIMINATOR: [u8; 8] =
    [0xe2, 0xea, 0x85, 0x31, 0xe4, 0x20, 0x2a, 0x0c];

pub const GAME_STATE_PROGRAM_ID: Pubkey = Pubkey::new_from_array([
    103, 148, 27, 1, 200, 217, 76, 87, 92, 42, 194, 80, 114, 230, 121, 192,
    54, 239, 209, 103, 217, 18, 202, 213, 138, 22, 161, 194, 40, 24, 140, 181,
]);


// ─── Resource type enum ───────────────────────────────────────────────────────
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, InitSpace)]
pub enum ResourceType {
    Metal = 0,
    Crystal = 1,
    Deuterium = 2,
}

impl ResourceType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ResourceType::Metal => "metal",
            ResourceType::Crystal => "crystal",
            ResourceType::Deuterium => "deuterium",
        }
    }
}

// ─── Accounts ─────────────────────────────────────────────────────────────────
#[account]
#[derive(InitSpace)]
pub struct MarketConfig {
    pub admin: Pubkey,
    pub antimatter_mint: Pubkey,
    pub total_volume: u128,
    pub total_offers: u64,
    pub bump: u8,
}

#[account]
#[derive(InitSpace)]
pub struct MarketOffer {
    pub seller: Pubkey,
    pub seller_planet: Pubkey,
    pub resource_type: ResourceType,
    pub resource_amount: u64,
    pub price_antimatter: u64,
    pub created_at: i64,
    pub offer_id: u32,
    pub filled: bool,
    pub bump: u8,
}

#[account]
#[derive(InitSpace)]
pub struct SellerCounter {
    pub seller: Pubkey,
    pub next_offer_id: u32,
    pub active_offers: u32,
    pub bump: u8,
}

// ─── Instruction Contexts ─────────────────────────────────────────────────────
#[derive(Accounts)]
pub struct InitializeMarket<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    #[account(init, payer = admin, space = MARKET_CONFIG_SPACE, seeds = [b"market_config"], bump)]
    pub market_config: Account<'info, MarketConfig>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct InitializeEscrow<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    #[account(seeds = [b"market_config"], bump = market_config.bump, has_one = admin @ MarketError::Unauthorized)]
    pub market_config: Account<'info, MarketConfig>,
    #[account(address = market_config.antimatter_mint @ MarketError::InvalidMint)]
    pub antimatter_mint: Account<'info, Mint>,
    #[account(init, payer = admin, seeds = [b"market_escrow"], bump, token::mint = antimatter_mint, token::authority = market_escrow_authority)]
    pub market_escrow: Account<'info, TokenAccount>,
    #[account(seeds = [b"market_authority"], bump)]
    pub market_escrow_authority: UncheckedAccount<'info>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct UpdateMarketConfig<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    #[account(mut, seeds = [b"market_config"], bump = market_config.bump, has_one = admin @ MarketError::Unauthorized)]
    pub market_config: Account<'info, MarketConfig>,
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
    #[account(address = GAME_STATE_PROGRAM_ID)]
    pub game_program: UncheckedAccount<'info>,
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
    #[account(address = GAME_STATE_PROGRAM_ID)]
    pub game_program: UncheckedAccount<'info>,
    #[account(mut, address = offer.seller_planet @ MarketError::InvalidSellerPlanet, owner = GAME_STATE_PROGRAM_ID)]
    pub seller_planet: UncheckedAccount<'info>,
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

    #[account(seeds = [b"market_authority"], bump)]
    pub market_escrow_authority: UncheckedAccount<'info>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,

    #[account(address = GAME_STATE_PROGRAM_ID)]
    pub game_program: UncheckedAccount<'info>,

    #[account(mut, address = offer.seller_planet @ MarketError::InvalidSellerPlanet, owner = GAME_STATE_PROGRAM_ID)]
    pub seller_planet: UncheckedAccount<'info>,

    #[account(mut, owner = GAME_STATE_PROGRAM_ID)]
    pub buyer_planet: UncheckedAccount<'info>,
}


// ─── Program ──────────────────────────────────────────────────────────────────
fn build_market_resource_ix(
    discriminator: [u8; 8],
    program_id: Pubkey,
    accounts: Vec<anchor_lang::solana_program::instruction::AccountMeta>,
    resource_type: ResourceType,
    resource_amount: u64,
) -> anchor_lang::solana_program::instruction::Instruction {
    let mut data = Vec::with_capacity(17);
    data.extend_from_slice(&discriminator);
    data.push(resource_type as u8);
    data.extend_from_slice(&resource_amount.to_le_bytes());

    anchor_lang::solana_program::instruction::Instruction {
        program_id,
        accounts,
        data,
    }
}

#[program]
pub mod market {
    use super::*;

    pub fn initialize_market(ctx: Context<InitializeMarket>, antimatter_mint: Pubkey) -> Result<()> {
        ctx.accounts.market_config.set_inner(MarketConfig {
            admin: ctx.accounts.admin.key(),
            antimatter_mint,
            total_volume: 0,
            total_offers: 0,
            bump: ctx.bumps.market_config,
        });
        Ok(())
    }

    pub fn initialize_escrow(_ctx: Context<InitializeEscrow>) -> Result<()> {
        msg!("Market escrow initialized");
        Ok(())
    }

    pub fn update_market_config(ctx: Context<UpdateMarketConfig>, antimatter_mint: Pubkey) -> Result<()> {
        ctx.accounts.market_config.antimatter_mint = antimatter_mint;
        Ok(())
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

        let counter = &mut ctx.accounts.seller_counter;
        let offer_id = counter.next_offer_id;

        if counter.seller == Pubkey::default() {
            counter.seller = ctx.accounts.seller.key();
            counter.bump = ctx.bumps.seller_counter;
        }

        counter.next_offer_id = counter.next_offer_id.saturating_add(1);
        counter.active_offers = counter.active_offers.saturating_add(1);

        let now = Clock::get()?.unix_timestamp;

        let lock_ix = build_market_resource_ix(
            LOCK_RESOURCES_FOR_MARKET_DISCRIMINATOR,
            ctx.accounts.game_program.key(),
            vec![
                anchor_lang::solana_program::instruction::AccountMeta::new(
                    ctx.accounts.seller.key(),
                    true,
                ),
                anchor_lang::solana_program::instruction::AccountMeta::new(
                    ctx.accounts.seller_planet.key(),
                    false,
                ),
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

        let authority_seeds: &[&[&[u8]]] =
            &[&[b"market_authority", &[ctx.bumps.market_authority]]];
        let release_ix = build_market_resource_ix(
            RELEASE_RESOURCES_FROM_MARKET_DISCRIMINATOR,
            ctx.accounts.game_program.key(),
            vec![
                anchor_lang::solana_program::instruction::AccountMeta::new(
                    ctx.accounts.seller_planet.key(),
                    false,
                ),
                anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                    ctx.accounts.market_authority.key(),
                    true,
                ),
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

        msg!("Offer cancelled: offer_id={}", ctx.accounts.offer.offer_id);
        Ok(())
    }

    #[allow(unreachable_code)]
    /// Buyer accepts the listing + transfers resources via CPI
  pub fn accept_offer(ctx: Context<AcceptOffer>) -> Result<()> {
        require!(!ctx.accounts.offer.filled, MarketError::AlreadyFilled);

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
                token::burn(
                    CpiContext::new_with_signer(
                        ctx.accounts.token_program.to_account_info(),
                        Burn {
                            mint: ctx.accounts.antimatter_mint.to_account_info(),
                            from: ctx.accounts.market_escrow.to_account_info(),
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
                    anchor_lang::solana_program::instruction::AccountMeta::new(
                        ctx.accounts.buyer_planet.key(),
                        false,
                    ),
                    anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
                        ctx.accounts.market_escrow_authority.key(),
                        true,
                    ),
                    anchor_lang::solana_program::instruction::AccountMeta::new(
                        ctx.accounts.buyer.key(),
                        true,
                    ),
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

            return Ok(());
        }

        // Antimatter payment validation
        require!(
            ctx.accounts.buyer_antimatter_account.amount >= price,
            MarketError::InsufficientAntimatter,
        );

        let fee = if MARKET_FEE_BPS > 0 {
            price.saturating_mul(MARKET_FEE_BPS) / 10_000
        } else {
            0
        };
        let seller_receives = price.saturating_sub(fee);

        let authority_seeds: &[&[&[u8]]] = &[&[b"market_authority", &[ctx.bumps.market_escrow_authority]]];

        // 1. Buyer → Escrow
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

        // 2. Escrow → Seller
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

        // 3. Burn fee
        if fee > 0 {
            token::burn(
                CpiContext::new_with_signer(
                    ctx.accounts.token_program.to_account_info(),
                    Burn {
                        mint: ctx.accounts.antimatter_mint.to_account_info(),
                        from: ctx.accounts.market_escrow.to_account_info(),
                        authority: ctx.accounts.market_escrow_authority.to_account_info(),
                    },
                    authority_seeds,
                ),
                fee,
            )?;
        }

        // ── 4. CPI to Game Program for resource transfer ─────────────────────
        let cpi_ix = anchor_lang::solana_program::instruction::Instruction {
            program_id: ctx.accounts.game_program.key(),
            accounts: vec![
                anchor_lang::solana_program::instruction::AccountMeta::new(ctx.accounts.seller_planet.key(), false),
                anchor_lang::solana_program::instruction::AccountMeta::new(ctx.accounts.buyer_planet.key(), false),
                anchor_lang::solana_program::instruction::AccountMeta::new(ctx.accounts.buyer.key(), true),
            ],
            data: {
                let mut data = vec![0u8; 17];
                // TODO: Replace with the actual 8-byte discriminator of your game program's transfer_resources_from_market instruction
                // You can get it by running: node -e 'console.log(require("crypto").createHash("sha256").update("global:transfer_resources_from_market").digest("hex").slice(0,16))'
                data[0..8].copy_from_slice(&[0,0,0,0,0,0,0,0]); // ← CHANGE THIS to real discriminator
                data[8] = resource_type as u8;
                data[9..17].copy_from_slice(&resource_amount.to_le_bytes());
                data
            },
        };

        anchor_lang::solana_program::program::invoke(
            &cpi_ix,
            &[
                ctx.accounts.seller_planet.to_account_info(),
                ctx.accounts.buyer_planet.to_account_info(),
                ctx.accounts.buyer.to_account_info(),
                ctx.accounts.game_program.to_account_info(),
            ],
        )?;

        // Mark offer as filled
        ctx.accounts.offer.filled = true;
        ctx.accounts.seller_counter.active_offers = ctx.accounts.seller_counter.active_offers.saturating_sub(1);
        ctx.accounts.market_config.total_volume = ctx.accounts.market_config.total_volume.saturating_add(price as u128);

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

// ─── Errors ───────────────────────────────────────────────────────────────────
#[error_code]
pub enum MarketError {
    #[msg("Unauthorized.")] Unauthorized,
    #[msg("Resource amount is below the minimum.")] AmountTooSmall,
    #[msg("Price must be at least 1 ANTIMATTER token.")] PriceTooLow,
    #[msg("Too many active offers from this wallet.")] TooManyOffers,
    #[msg("This offer has already been filled or cancelled.")] AlreadyFilled,
    #[msg("Insufficient ANTIMATTER tokens.")] InsufficientAntimatter,
    #[msg("Invalid seller account.")] InvalidSeller,
    #[msg("Invalid seller planet account.")] InvalidSellerPlanet,
    #[msg("Invalid ANTIMATTER mint.")] InvalidMint,
    #[msg("Invalid token account owner.")] InvalidTokenAccount,
    #[msg("Seller does not have enough resources.")] InsufficientResources,
}
