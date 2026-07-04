use anchor_lang::prelude::*;
use anchor_lang::solana_program::instruction::{AccountMeta, Instruction};
use anchor_spl::associated_token::get_associated_token_address;

use crate::error::MarketError;
use crate::types::ResourceType;

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
