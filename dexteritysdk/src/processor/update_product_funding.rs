use crate::{common::utils::*, sdk_client::SDKClient, KeypairD};
use agnostic_orderbook::state::*;
use anchor_lang::{InstructionData, ToAccountMetas};
use dex::{accounts, instruction, state::enums::OrderType, utils::numeric::Fractional};
use solana_program::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    system_program,
};
use solana_program_test::ProgramTestContext;
use solana_sdk::signature::{Keypair, Signer};

pub fn update_product_funding_ixs(
    market_product_group: Pubkey,
    product: &Keypair,
    amount: Fractional,
    expired: bool,
) -> Vec<Instruction> {
    let params = dex::UpdateProductFundingParams { amount, expired };
    let account_metas = accounts::UpdateProductFunding {
        market_product_group,
        product: product.pubkey(),
    }
    .to_account_metas(None);

    vec![Instruction {
        program_id: dex::ID,
        data: instruction::UpdateProductFunding { params }.data(),
        accounts: account_metas,
    }]
}

pub async fn update_product_funding(
    client: &SDKClient,
    market_product_group: Pubkey,
    product: &KeypairD,
    amount: Fractional,
    expired: bool,
) -> SDKResult {
    let ixs = update_product_funding_ixs(market_product_group, product, amount, expired);
    client.sign_send_instructions(ixs, vec![product]).await
}
