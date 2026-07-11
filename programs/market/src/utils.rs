use anchor_lang::prelude::*;
use anchor_lang::solana_program::instruction::{AccountMeta, Instruction};
use anchor_spl::associated_token::get_associated_token_address;

use crate::constants::MARKET_FEE_BPS;
use crate::error::MarketError;
use crate::types::ResourceType;

pub fn market_fee(amount: u64) -> u64 {
    ((amount as u128) * (MARKET_FEE_BPS as u128) / 10_000) as u64
}

pub fn build_market_resource_ix(
    discriminator: [u8; 8],
    program_id: Pubkey,
    accounts: Vec<AccountMeta>,
    resource_type: ResourceType,
    resource_amount: u64,
) -> Instruction {
    let mut data = Vec::with_capacity(17);
    data.extend_from_slice(&discriminator);
    data.push(resource_type as u8);
    data.extend_from_slice(&resource_amount.to_le_bytes());

    Instruction {
        program_id,
        accounts,
        data,
    }
}

pub fn require_protocol_antimatter_treasury(
    treasury: Pubkey,
    admin: Pubkey,
    mint: Pubkey,
) -> Result<()> {
    let expected_treasury = get_associated_token_address(&admin, &mint);
    require_keys_eq!(treasury, expected_treasury, MarketError::InvalidTokenAccount);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn market_fee_is_exact_without_u64_multiplication_overflow() {
        assert_eq!(market_fee(10_000), MARKET_FEE_BPS);
        assert_eq!(
            market_fee(u64::MAX),
            ((u64::MAX as u128) * (MARKET_FEE_BPS as u128) / 10_000) as u64
        );
        assert!(market_fee(u64::MAX) > u64::MAX / 10_000);
    }
}
