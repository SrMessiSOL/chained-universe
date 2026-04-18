use anchor_lang::prelude::*;
use anchor_lang::solana_program::instruction::{AccountMeta, Instruction};

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
