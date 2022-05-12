use crate::{common::utils::*, sdk_client::SDKClient};
use anchor_lang::{InstructionData, ToAccountMetas};
use dex::utils::numeric::Fractional;
use instruments::{
    accounts,
    state::enums::{InstrumentType, OracleType},
};
use solana_program::{
    clock::UnixTimestamp,
    instruction::Instruction,
    pubkey::Pubkey,
    system_program,
    sysvar::{clock::Clock, rent::Rent, Sysvar},
};
use solana_program_test::ProgramTestContext;
use solana_sdk::signature::{Keypair, Signer};

pub fn get_derivative_key(
    price_oracle: Pubkey,
    market_product_group: Pubkey,
    instrument_type: InstrumentType,
    strike: Fractional,
    full_funding_period: u64,
    minimum_funding_period: i64,
    initialization_time: i64,
) -> Pubkey {
    let seeds: &[&[u8]] = &[
        b"derivative",
        &price_oracle.to_bytes(),
        &market_product_group.to_bytes(),
        &(instrument_type as u64).to_le_bytes(),
        &strike.m.to_le_bytes(),
        &strike.exp.to_le_bytes(),
        &initialization_time.to_le_bytes(),
        &full_funding_period.to_le_bytes(),
        &minimum_funding_period.to_le_bytes(),
    ];
    let (derivative_metadata_key, _) = Pubkey::find_program_address(seeds, &instruments::ID);
    derivative_metadata_key
}

pub fn initialize_derivative_ixs(
    close_authority: Pubkey,
    price_oracle: Pubkey,
    market_product_group: Pubkey,
    payer: Pubkey,
    clock: Pubkey,
    derivative_metadata: Pubkey,
    instrument_type: InstrumentType,
    strike: impl Into<Fractional>,
    full_funding_period: UnixTimestamp,
    minimum_funding_period: UnixTimestamp,
    initialization_time: UnixTimestamp,
    oracle_type: OracleType,
) -> Vec<Instruction> {
    let params = instruments::InitializeDerivativeParams {
        instrument_type,
        strike: strike.into(),
        full_funding_period,
        minimum_funding_period,
        close_authority,
        initialization_time,
        oracle_type,
    };

    let account_metas = instruments::accounts::InitializeDerivative {
        derivative_metadata: derivative_metadata,
        price_oracle,
        market_product_group,
        payer: payer,
        system_program: system_program::id(),
        clock,
    }
    .to_account_metas(Some(true));

    let instruction = Instruction {
        program_id: instruments::ID,
        data: instruments::instruction::InitializeDerivative { params }.data(),
        accounts: account_metas,
    };
    vec![instruction]
}

pub struct InitializeDerivativeOptionalArgs {
    pub instrument_type: InstrumentType,
    pub initialization_time: i64,
    pub full_funding_period: i64,
    pub minimum_funding_period: i64,
    pub oracle_type: OracleType,
}

impl Default for InitializeDerivativeOptionalArgs {
    fn default() -> Self {
        InitializeDerivativeOptionalArgs {
            instrument_type: InstrumentType::RecurringCall,
            initialization_time: 100,
            full_funding_period: 101,
            minimum_funding_period: 10,
            oracle_type: OracleType::Dummy,
        }
    }
}

impl InitializeDerivativeOptionalArgs {
    pub fn new(
        instrument_type: InstrumentType,
        initialization_time: i64,
        full_funding_period: i64,
        minimum_funding_period: i64,
        oracle_type: OracleType,
    ) -> Self {
        InitializeDerivativeOptionalArgs {
            instrument_type,
            initialization_time,
            full_funding_period,
            minimum_funding_period,
            oracle_type,
        }
    }
}

pub async fn initialize_derivative(
    client: &SDKClient,
    close_authority: Pubkey,
    market_product_group: Pubkey,
    price_oracle: Pubkey,
    clock: Pubkey,
    strike: impl Into<Fractional>,
    optional_args: InitializeDerivativeOptionalArgs,
) -> std::result::Result<Pubkey, SDKError> {
    let instrument_type = optional_args.instrument_type;
    let initialization_time = optional_args.initialization_time;
    let full_funding_period = optional_args.full_funding_period;
    let minimum_funding_period = optional_args.minimum_funding_period;
    let oracle_type = optional_args.oracle_type;
    let strike = strike.into();

    let derivative_metadata = get_derivative_key(
        price_oracle,
        market_product_group,
        instrument_type,
        strike,
        full_funding_period as u64,
        minimum_funding_period,
        initialization_time,
    );

    let ixs = initialize_derivative_ixs(
        close_authority,
        price_oracle,
        market_product_group,
        client.payer.pubkey(),
        clock,
        derivative_metadata,
        instrument_type,
        strike,
        full_funding_period,
        minimum_funding_period,
        initialization_time,
        oracle_type,
    );
    client
        .sign_send_instructions(ixs, vec![&client.payer])
        .await?;
    Ok(derivative_metadata)
}
