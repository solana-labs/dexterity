use crate::{common::utils::*, sdk_client::SDKClient, KeypairD};
use agnostic_orderbook::state::critbit::Slab;
use agnostic_orderbook::state::market_state::MarketState;
use agnostic_orderbook::state::orderbook::CallbackInfo;
use anchor_lang::prelude::*;
use anchor_lang::InstructionData;
use bonfida_utils::InstructionsAccount;
use solana_program::{
    instruction::Instruction, pubkey::Pubkey, system_instruction::create_account,
};
use solana_program_test::ProgramTestContext;
use solana_sdk::{
    signature::{Keypair, Signer},
    system_instruction::create_nonce_account,
};

pub fn create_orderbook_ixs(
    client: &SDKClient,
    aaob_program_id: Pubkey,
    caller_authority: Pubkey,
    market_account: &Keypair,
    event_queue_account: &Keypair,
    bids_account: &Keypair,
    asks_account: &Keypair,
    eq_size: u64,
    bids_size: u64,
    asks_size: u64,
    min_base_order_size: u64,
    cranker_reward: u64,
    market_product_group: Pubkey,
    authority: Pubkey,
) -> Vec<Instruction> {
    let size = 8 + MarketState::LEN;
    let lamports = client.rent_exempt(size).max(1);
    //TODO: Change
    let create_market_account_ix = create_account(
        &client.payer.pubkey(),
        &market_account.pubkey(),
        lamports,
        size as u64,
        &dex::ID,
    );
    let lamports = client.rent_exempt(eq_size as usize).max(1);
    // Create event queue account
    let create_event_queue_account_ix = create_account(
        &client.payer.pubkey(),
        &event_queue_account.pubkey(),
        lamports,
        eq_size,
        &dex::ID,
    );
    // Create bids account
    let lamports = client.rent_exempt(bids_size as usize).max(1);
    let create_bids_account_ix = create_account(
        &client.payer.pubkey(),
        &bids_account.pubkey(),
        lamports,
        bids_size,
        &dex::ID,
    );
    // Create asks account
    let lamports = client.rent_exempt(asks_size as usize).max(1);
    let create_asks_account_ix = create_account(
        &client.payer.pubkey(),
        &asks_account.pubkey(),
        lamports,
        asks_size,
        &dex::ID,
    );

    let create_market_ix = Instruction {
        program_id: dex::ID,
        data: dex::instruction::CreateMarket {
            params: agnostic_orderbook::instruction::create_market::Params {
                min_base_order_size,
                tick_size: 1,
            },
        }
        .data(),
        accounts: dex::accounts::CreateMarketAccounts {
            authority,
            market: market_account.pubkey(),
            event_queue: event_queue_account.pubkey(),
            bids: bids_account.pubkey(),
            asks: asks_account.pubkey(),
            market_product_group,
        }
        .to_account_metas(Some(true)),
    };

    vec![
        create_market_account_ix,
        create_event_queue_account_ix,
        create_bids_account_ix,
        create_asks_account_ix,
        create_market_ix,
    ]
}

pub async fn create_orderbook(
    client: &SDKClient,
    aaob_program_id: Pubkey,
    caller_authority: Pubkey,
    market_product_group: Pubkey,
    authority: &KeypairD,
) -> std::result::Result<(Pubkey, Pubkey, Pubkey, Pubkey), SDKError> {
    // Create market state account
    let market_account = KeypairD::new();
    let event_queue_account = KeypairD::new();
    let bids_account = KeypairD::new();
    let asks_account = KeypairD::new();

    // TODO: verify the desired event capacity
    let event_size = agnostic_orderbook::state::event_queue::EventQueue::<
        dex::state::callback_info::CallBackInfoDex,
    >::compute_allocation_size(1000);
    let bids_asks_len =
        Slab::<dex::state::callback_info::CallBackInfoDex>::compute_allocation_size(1000);
    let ixs = create_orderbook_ixs(
        client,
        aaob_program_id,
        caller_authority,
        &market_account,
        &event_queue_account,
        &bids_account,
        &asks_account,
        event_size as u64,
        bids_asks_len as u64,
        bids_asks_len as u64,
        1,
        1000,
        market_product_group,
        authority.pubkey(),
    );
    client
        .sign_send_instructions(
            ixs,
            vec![
                authority,
                &market_account,
                &event_queue_account,
                &bids_account,
                &asks_account,
            ],
        )
        .await?;
    Ok((
        market_account.pubkey(),
        bids_account.pubkey(),
        asks_account.pubkey(),
        event_queue_account.pubkey(),
    ))
}

pub async fn create_orderbook_with_params(
    client: &SDKClient,
    aaob_program_id: Pubkey,
    caller_authority: Pubkey,
    eq_size: u64,
    bids_size: u64,
    asks_size: u64,
    min_base_order_size: u64,
    cranker_reward: u64,
    market_product_group: Pubkey,
    authority: &KeypairD,
) -> std::result::Result<(Pubkey, Pubkey, Pubkey, Pubkey), SDKError> {
    let market_account = KeypairD::new();
    let event_queue_account = KeypairD::new();
    let bids_account = KeypairD::new();
    let asks_account = KeypairD::new();
    let ixs = create_orderbook_ixs(
        client,
        aaob_program_id,
        caller_authority,
        &market_account,
        &event_queue_account,
        &bids_account,
        &asks_account,
        eq_size,
        bids_size,
        asks_size,
        min_base_order_size,
        cranker_reward,
        market_product_group,
        authority.pubkey(),
    );
    client
        .sign_send_instructions(
            ixs,
            vec![
                authority,
                &market_account,
                &event_queue_account,
                &bids_account,
                &asks_account,
            ],
        )
        .await?;
    Ok((
        market_account.pubkey(),
        bids_account.pubkey(),
        asks_account.pubkey(),
        event_queue_account.pubkey(),
    ))
}
