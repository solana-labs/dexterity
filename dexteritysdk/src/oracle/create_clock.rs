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

pub async fn create_clock_account(
    client: &SDKClient,
    dummy_oracle_program_id: Pubkey,
    authority: &KeypairD,
    system_program_id: Pubkey,
) -> std::result::Result<Pubkey, SDKError> {
    let seeds: &[&[u8]] = &[b"clock"];
    let (clock_metadata_key, _) = Pubkey::find_program_address(&seeds, &dummy_oracle_program_id);
    let initialize_clock_ix = dummy_oracle::processor::initialize_clock_ix(
        dummy_oracle_program_id,
        clock_metadata_key,
        authority.pubkey(),
        system_program_id,
        dummy_oracle::processor::initialize_clock::Params {},
    );
    client
        .sign_send_instructions(vec![initialize_clock_ix], vec![authority])
        .await?;
    Ok(clock_metadata_key)
}
