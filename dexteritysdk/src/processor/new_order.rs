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

pub fn new_order_ixs(
    aaob_program: Pubkey,
    user: Pubkey,
    trader_risk_group: Pubkey,
    market_product_group: Pubkey,
    product: Pubkey,
    market_signer: Pubkey,
    orderbook: Pubkey,
    event_queue: Pubkey,
    bids: Pubkey,
    asks: Pubkey,
    fee_model_program: Pubkey,
    fee_model_configuration_acct: Pubkey,
    trader_fee_state_acct: Pubkey,
    fee_output_register: Pubkey,
    risk_engine_program: Pubkey,
    risk_model_configuration_acct: Pubkey,
    risk_engine_accounts: &[Pubkey],
    side: Side,
    max_base_qty: Fractional,
    order_type: OrderType,
    self_trade_behavior: SelfTradeBehavior,
    match_limit: u64,
    limit_price: Fractional,
    risk_output_register: Pubkey,
    trader_risk_state_acct: Pubkey,
) -> Vec<Instruction> {
    let params = dex::NewOrderParams {
        side,
        max_base_qty,
        order_type,
        self_trade_behavior,
        match_limit,
        limit_price,
    };
    let (risk_and_fee_signer, _) =
        Pubkey::find_program_address(&[market_product_group.as_ref()], &dex::ID);
    let mut account_metas = accounts::NewOrder {
        aaob_program,
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
        fee_model_program,
        fee_model_configuration_acct,
        risk_model_configuration_acct,
        trader_fee_state_acct,
        fee_output_register,
        risk_engine_program,
        risk_output_register,
        trader_risk_state_acct,
        risk_and_fee_signer,
    }
    .to_account_metas(Some(true));
    for key in risk_engine_accounts.iter() {
        account_metas.push(AccountMeta::new(*key, false));
    }
    vec![Instruction {
        program_id: dex::ID,
        data: instruction::NewOrder { params }.data(),
        accounts: account_metas,
    }]
}

pub async fn new_order(
    client: &SDKClient,
    dex_program_id: Pubkey,
    aaob_program_id: Pubkey,
    user: &KeypairD,
    user_trader_risk_group: Pubkey,
    market_product_group: Pubkey,
    product_account: Pubkey,
    market_signer: Pubkey,
    orderbook: Pubkey,
    event_queue: Pubkey,
    bids: Pubkey,
    asks: Pubkey,
    fee_model_program_id: Pubkey,
    fee_model_config_acct: Pubkey,
    trader_fee_state: Pubkey,
    fee_output_register: Pubkey,
    risk_engine_program_id: Pubkey,
    risk_model_configuration_acct: Pubkey,
    risk_engine_accounts: &[Pubkey],
    side: Side,
    max_base_qty: Fractional,
    order_type: OrderType,
    self_trade_behavior: SelfTradeBehavior,
    match_limit: u64,
    limit_price: Fractional,
    out_register_risk_info: Pubkey,
    risk_state_account_info: Pubkey,
) -> SDKResult {
    let ixs = new_order_ixs(
        aaob_program_id,
        user.pubkey(),
        user_trader_risk_group,
        market_product_group,
        product_account,
        market_signer,
        orderbook,
        event_queue,
        bids,
        asks,
        fee_model_program_id,
        fee_model_config_acct,
        trader_fee_state,
        fee_output_register,
        risk_engine_program_id,
        risk_model_configuration_acct,
        risk_engine_accounts,
        side,
        max_base_qty,
        order_type,
        self_trade_behavior,
        match_limit,
        limit_price,
        out_register_risk_info,
        risk_state_account_info,
    );
    client.sign_send_instructions(ixs, vec![user]).await
}
