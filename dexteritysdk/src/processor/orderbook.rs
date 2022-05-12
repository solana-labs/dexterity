use agnostic_orderbook::{
    instruction::create_market,
    state::{EVENT_QUEUE_HEADER_LEN, REGISTER_SIZE},
};
use bonfida_utils::InstructionsAccount;
use solana_program::{
    instruction::Instruction, pubkey::Pubkey, system_instruction::create_account,
};
use solana_program_test::ProgramTestContext;
use solana_sdk::signature::{Keypair, Signer};

use crate::{common::utils::*, sdk_client::SDKClient, KeypairD};

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
) -> Vec<Instruction> {
    let size = 192_u64;
    let lamports = client.rent_exempt(size as usize).max(1);
    let create_market_account_ix = create_account(
        &client.payer.pubkey(),
        &market_account.pubkey(),
        lamports,
        size,
        &aaob_program_id,
    );
    let lamports = client.rent_exempt(eq_size as usize).max(1);
    // Create event queue account
    let create_event_queue_account_ix = create_account(
        &client.payer.pubkey(),
        &event_queue_account.pubkey(),
        lamports,
        eq_size,
        &aaob_program_id,
    );
    // Create bids account
    let lamports = client.rent_exempt(bids_size as usize).max(1);
    let create_bids_account_ix = create_account(
        &client.payer.pubkey(),
        &bids_account.pubkey(),
        lamports,
        bids_size,
        &aaob_program_id,
    );
    // Create asks account
    let lamports = client.rent_exempt(asks_size as usize).max(1);
    let create_asks_account_ix = create_account(
        &client.payer.pubkey(),
        &asks_account.pubkey(),
        lamports,
        asks_size,
        &aaob_program_id,
    );
    // Create Market
    let create_market_ix = create_market::Accounts {
        market: &market_account.pubkey(),
        event_queue: &event_queue_account.pubkey(),
        bids: &bids_account.pubkey(),
        asks: &asks_account.pubkey(),
    }
    .get_instruction(
        aaob_program_id,
        agnostic_orderbook::instruction::AgnosticOrderbookInstruction::CreateMarket as u8,
        create_market::Params {
            caller_authority: caller_authority.to_bytes(),
            callback_info_len: 40,
            callback_id_len: 32,
            min_base_order_size,
            tick_size: 1,
            cranker_reward,
        },
    );
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
) -> std::result::Result<(Pubkey, Pubkey, Pubkey, Pubkey), SDKError> {
    // Create market state account
    let market_account = KeypairD::new();
    let event_queue_account = KeypairD::new();
    let bids_account = KeypairD::new();
    let asks_account = KeypairD::new();

    let event_size =
        agnostic_orderbook::state::Event::compute_slot_size(std::mem::size_of::<Pubkey>() + 8);
    let eq_size = 1000 * event_size + EVENT_QUEUE_HEADER_LEN + REGISTER_SIZE;

    let ixs = create_orderbook_ixs(
        client,
        aaob_program_id,
        caller_authority,
        &market_account,
        &event_queue_account,
        &bids_account,
        &asks_account,
        eq_size as u64,
        65_536_u64,
        65_536_u64,
        1,
        1000,
    );
    client
        .sign_send_instructions(
            ixs,
            vec![
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
) -> std::result::Result<(Pubkey, Pubkey, Pubkey, Pubkey), SDKError> {
    // Create market state account
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
    );
    client
        .sign_send_instructions(
            ixs,
            vec![
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
