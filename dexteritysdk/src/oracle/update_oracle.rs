use crate::{
    common::utils::{SDKError, SDKResult},
    sdk_client::SDKClient,
    state::clone_keypair,
    KeypairD, SDKContext,
};
use solana_program::{
    instruction::Instruction, message::Message, program_error::ProgramError,
    program_option::COption, program_pack::Pack, pubkey::Pubkey, sysvar::rent::Rent,
};
use solana_sdk::{
    account::Account,
    client::{Client, SyncClient},
    signature::{Keypair, Signer},
    system_instruction,
    transaction::Transaction,
    transport::TransportError,
};

pub async fn update_oracle_price_account(
    client: &SDKClient,
    dummy_oracle_program_id: Pubkey,
    authority: &KeypairD,
    system_program_id: Pubkey,
    price: i64,
    decimals: u64,
) -> std::result::Result<Pubkey, SDKError> {
    let seeds: Vec<Vec<u8>> = vec![b"oracle".to_vec()];
    let (oracle_metadata_key, _) = Pubkey::find_program_address(
        seeds
            .iter()
            .map(|v| v.as_slice())
            .collect::<Vec<&[u8]>>()
            .as_ref(),
        &dummy_oracle_program_id,
    );
    let initialize_oracle_ix = dummy_oracle::processor::update_price_ix(
        dummy_oracle_program_id,
        oracle_metadata_key,
        authority.pubkey(),
        system_program_id,
        dummy_oracle::processor::update_price::Params {
            price: price,
            decimals: decimals,
        },
    );
    client
        .sign_send_instructions(vec![initialize_oracle_ix], vec![authority])
        .await?;
    Ok(oracle_metadata_key)
}
