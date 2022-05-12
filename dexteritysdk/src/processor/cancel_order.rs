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

pub fn cancel_order_ixs(
    aaob_program_id: Pubkey,
    user: Pubkey,
    trader_risk_group: Pubkey,
    market_product_group: Pubkey,
    product: Pubkey,
    market_signer: Pubkey,
    orderbook: Pubkey,
    event_queue: Pubkey,
    bids: Pubkey,
    asks: Pubkey,
    risk_engine_program: Pubkey,
    risk_engine_accounts: Vec<Pubkey>,
    order_id: u128,
    risk_output_register: Pubkey,
    trader_risk_state_acct: Pubkey,
    risk_model_configuration_acct: Pubkey,
) -> Vec<Instruction> {
    let (risk_signer, _) = Pubkey::find_program_address(&[market_product_group.as_ref()], &dex::ID);
    let mut account_metas = accounts::CancelOrder {
        aaob_program: aaob_program_id,
        user,
        trader_risk_group,
        market_product_group,
        product,
        market_signer,
        orderbook,
        event_queue,
        bids,
        asks,
        system_program: system_program::id(),
        risk_engine_program,
        risk_output_register,
        trader_risk_state_acct,
        risk_model_configuration_acct: risk_model_configuration_acct,
        risk_signer,
    }
    .to_account_metas(None);
    for key in risk_engine_accounts.into_iter() {
        account_metas.push(AccountMeta::new_readonly(key, false));
    }
    vec![Instruction {
        program_id: dex::ID,
        data: instruction::CancelOrder {
            params: dex::CancelOrderParams { order_id },
        }
        .data(),
        accounts: account_metas,
    }]
}
