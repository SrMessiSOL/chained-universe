use anchor_lang::prelude::*;

#[error_code]
pub enum MarketError {
    #[msg("Unauthorized.")]
    Unauthorized,
    #[msg("Resource amount is below the minimum.")]
    AmountTooSmall,
    #[msg("Price must be at least 1 ANTIMATTER token.")]
    PriceTooLow,
    #[msg("Too many active offers from this wallet.")]
    TooManyOffers,
    #[msg("This offer has already been filled or cancelled.")]
    AlreadyFilled,
    #[msg("Insufficient ANTIMATTER tokens.")]
    InsufficientAntimatter,
    #[msg("Invalid seller account.")]
    InvalidSeller,
    #[msg("Invalid seller planet account.")]
    InvalidSellerPlanet,
    #[msg("Invalid ANTIMATTER mint.")]
    InvalidMint,
    #[msg("Invalid token account owner.")]
    InvalidTokenAccount,
    #[msg("Seller does not have enough resources.")]
    InsufficientResources,
}
