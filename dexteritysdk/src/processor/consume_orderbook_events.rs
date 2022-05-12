use anchor_lang::{InstructionData, ToAccountMetas};
use solana_program::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
};
use solana_program_test::ProgramTestContext;
use solana_sdk::signature::{Keypair, Signer};

use dex::{accounts, instruction};

use crate::{common::utils::*, sdk_client::SDKClient};

pub fn consume_orderbook_events_ixs(
    aaob_program: Pubkey,
    market_product_group: Pubkey,
    product: Pubkey,
    market_signer: Pubkey,
    orderbook: Pubkey,
    event_queue: Pubkey,
    reward_target: &Keypair,
    fee_model_program: Pubkey,
    fee_model_configuration_acct: Pubkey,
    risk_model_configuration_acct: Pubkey,
    fee_output_register: Pubkey,
    risk_engine_program: Pubkey,
    risk_output_register: Pubkey,
    user_accounts: &[Pubkey],
    max_iterations: u64,
) -> Vec<Instruction> {
    let (risk_and_fee_signer, _) =
        Pubkey::find_program_address(&[market_product_group.as_ref()], &dex::ID);
    let params = dex::ConsumeOrderbookEventsParams { max_iterations };
    let mut account_metas = accounts::ConsumeOrderbookEvents {
        aaob_program,
        market_product_group,
        product,
        market_signer,
        orderbook,
        event_queue,
        reward_target: reward_target.pubkey(),
        fee_model_program,
        fee_model_configuration_acct,
        fee_output_register,
        risk_and_fee_signer: risk_and_fee_signer,
    }
    .to_account_metas(None);
    for key in user_accounts {
        account_metas.push(AccountMeta::new(*key, false));
    }
    vec![Instruction {
        program_id: dex::ID,
        data: instruction::ConsumeOrderbookEvents { params }.data(),
        accounts: account_metas,
    }]
}
