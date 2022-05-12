use crate::{common::utils::*, sdk_client::SDKClient};
use anchor_lang::{InstructionData, ToAccountMetas};
use dex::{accounts, state::constants::NAME_LEN, utils::numeric::Fractional};
use solana_program::{instruction::Instruction, pubkey::Pubkey};
use solana_program_test::ProgramTestContext;
use solana_sdk::signature::{Keypair, Signer};

pub fn initialize_market_product_ixs(
    authority: Pubkey,
    market_product_group: Pubkey,
    product: Pubkey,
    orderbook: Pubkey,
    name: [u8; NAME_LEN],
    tick_size: Fractional,
    base_decimals: u64,
    price_offset: Fractional,
) -> Vec<Instruction> {
    let params = dex::InitializeMarketProductParams {
        name,
        tick_size,
        base_decimals,
        price_offset,
    };
    let account_metas = accounts::InitializeMarketProduct {
        authority,
        market_product_group,
        product,
        orderbook,
    }
    .to_account_metas(None);
    vec![Instruction {
        program_id: dex::ID,
        data: dex::instruction::InitializeMarketProduct { params }.data(),
        accounts: account_metas,
    }]
}
