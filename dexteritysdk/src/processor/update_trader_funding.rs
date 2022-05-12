use crate::{common::utils::*, sdk_client::SDKClient, KeypairD};
use agnostic_orderbook::state::*;
use anchor_lang::{InstructionData, ToAccountMetas};
use dex::{accounts, instruction, state::enums::OrderType, utils::numeric::Fractional};
use rand::Rng;
use solana_program::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    system_program,
};
use solana_program_test::ProgramTestContext;
use solana_sdk::signature::{Keypair, Signer};

pub fn update_trader_funding_ixs(
    trader_risk_group: Pubkey,
    market_product_group: Pubkey,
) -> Vec<Instruction> {
    let account_metas = accounts::UpdateTraderFunding {
        market_product_group,
        trader_risk_group,
    }
    .to_account_metas(None);

    let mut rng = rand::prelude::thread_rng();
    let out = rng.gen_range(0..255);
    vec![Instruction {
        program_id: dex::ID,
        data: [instruction::UpdateTraderFunding {}.data(), vec![out]].concat(),
        accounts: account_metas,
    }]
}

pub async fn update_trader_funding(
    client: &SDKClient,
    user_trader_risk_group: Pubkey,
    market_product_group: Pubkey,
) -> SDKResult {
    let ixs = update_trader_funding_ixs(user_trader_risk_group, market_product_group);
    client.sign_send_instructions(ixs, vec![]).await
}
