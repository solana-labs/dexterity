use anchor_lang::{InstructionData, ToAccountMetas};
use solana_program::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
};
use solana_program_test::ProgramTestContext;
use solana_sdk::signature::{Keypair, Signer};

use dex::{accounts, instruction};

use crate::{common::utils::*, sdk_client::SDKClient};

pub fn remove_market_product_ixs(
    authority: Pubkey,
    market_product_group: Pubkey,
    product: Pubkey,
    aaob_program_id: Pubkey,
    orderbook: Pubkey,
    market_signer: Pubkey,
    event_queue: Pubkey,
    bids: Pubkey,
    asks: Pubkey,
) -> Vec<Instruction> {
    let account_metas = accounts::RemoveMarketProduct {
        authority,
        market_product_group,
        product,
        aaob_program: aaob_program_id,
        orderbook,
        market_signer,
        event_queue,
        bids,
        asks,
    }
    .to_account_metas(None);
    vec![Instruction {
        program_id: dex::ID,
        data: instruction::RemoveMarketProduct.data(),
        accounts: account_metas,
    }]
}
