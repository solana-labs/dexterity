use anchor_lang::{InstructionData, ToAccountMetas};
use solana_program::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    system_program,
};
use solana_program_test::ProgramTestContext;
use solana_sdk::signature::{Keypair, Signer};

use dex::{accounts, instruction};

use crate::{common::utils::*, sdk_client::SDKClient};

pub fn clear_expired_orderbook_ixs(
    aaob_program_id: Pubkey,
    market_product_group: Pubkey,
    product: Pubkey,
    market_signer: Pubkey,
    orderbook: Pubkey,
    event_queue: Pubkey,
    bids: Pubkey,
    asks: Pubkey,
    n: Option<u64>,
) -> Vec<Instruction> {
    let num_orders_to_cancel = match n {
        Some(num) => num,
        None => 20,
    } as u8;
    let account_metas = accounts::ClearExpiredOrderbook {
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
        data: instruction::ClearExpiredOrderbook {
            params: dex::ClearExpiredOrderbookParams {
                num_orders_to_cancel,
            },
        }
        .data(),
        accounts: account_metas,
    }]
}
