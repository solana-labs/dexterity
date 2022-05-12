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

pub struct CreateOracleOptionalArgs {
    price: i64,
    decimals: u64,
}

impl Default for CreateOracleOptionalArgs {
    fn default() -> Self {
        CreateOracleOptionalArgs {
            price: 100,
            decimals: 8,
        }
    }
}

pub async fn create_oracle_price_account(
    client: &SDKClient,
    dummy_oracle_program_id: Pubkey,
    authority: &KeypairD,
    system_program_id: Pubkey,
    optional_args: CreateOracleOptionalArgs,
) -> std::result::Result<Pubkey, SDKError> {
    let seeds: &[&[u8]] = &[b"oracle"];
    let (oracle_metadata_key, _) = Pubkey::find_program_address(seeds, &dummy_oracle_program_id);
    let initialize_oracle_ix = dummy_oracle::processor::initialize_oracle_ix(
        dummy_oracle_program_id,
        oracle_metadata_key,
        authority.pubkey(),
        system_program_id,
        dummy_oracle::processor::initialize_oracle::Params {
            price: optional_args.price,
            decimals: optional_args.decimals,
        },
    );
    client
        .sign_send_instructions(vec![initialize_oracle_ix], vec![authority])
        .await?;
    Ok(oracle_metadata_key)
}
