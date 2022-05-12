use anchor_lang::{InstructionData, ToAccountMetas};
use dex::{accounts, state::constants::NAME_LEN, utils::numeric::Fractional};
use solana_program::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
};
use solana_program_test::ProgramTestContext;
use solana_sdk::signature::{Keypair, Signer};

use crate::{common::utils::*, sdk_client::SDKClient};

pub fn initialize_combo_ixs(
    authority: Pubkey,
    market_product_group: Pubkey,
    orderbook: Pubkey,
    name: [u8; NAME_LEN],
    products: &[Pubkey],
    tick_size: Fractional,
    price_offset: Fractional,
    base_decimals: u64,
    ratios: Vec<i8>,
) -> Vec<Instruction> {
    let params = dex::InitializeComboParams {
        name,
        tick_size,
        price_offset,
        base_decimals,
        ratios,
    };
    let mut account_metas = accounts::InitializeCombo {
        authority,
        market_product_group,
        orderbook,
    }
    .to_account_metas(None);
    for key in products {
        account_metas.push(AccountMeta::new_readonly(*key, false));
    }
    vec![Instruction {
        program_id: dex::ID,
        data: dex::instruction::InitializeCombo { params }.data(),
        accounts: account_metas,
    }]
}
