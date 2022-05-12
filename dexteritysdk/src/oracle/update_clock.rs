use crate::{
    common::utils::{SDKError, SDKResult},
    sdk_client::{ClientSubset, SDKClient},
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

pub async fn update_clock_account(
    client: &SDKClient,
    dummy_oracle_program_id: Pubkey,
    authority: &KeypairD,
    system_program_id: Pubkey,
    slot: u64,
    epoch_start_timestamp: i64,
    epoch: u64,
    leader_schedule_epoch: u64,
    unix_timestamp: i64,
) -> std::result::Result<Pubkey, SDKError> {
    let seeds: Vec<Vec<u8>> = vec![b"clock".to_vec()];
    let (clock_metadata_key, _) = Pubkey::find_program_address(
        seeds
            .iter()
            .map(|v| v.as_slice())
            .collect::<Vec<&[u8]>>()
            .as_ref(),
        &dummy_oracle_program_id,
    );
    let initialize_clock_ix = dummy_oracle::processor::update_clock_ix(
        dummy_oracle_program_id,
        clock_metadata_key,
        authority.pubkey(),
        system_program_id,
        dummy_oracle::processor::update_clock::Params {
            slot: slot,
            epoch_start_timestamp: epoch_start_timestamp,
            epoch: epoch,
            leader_schedule_epoch: leader_schedule_epoch,
            unix_timestamp: unix_timestamp,
        },
    );
    client
        .sign_send_instructions(vec![initialize_clock_ix], vec![authority])
        .await?;
    Ok(clock_metadata_key)
}
