use crate::{
    error::DerivativeError,
    state::enums::{AccountTag, ExpirationStatus, InstrumentType},
    InitializeDerivative, InitializeDerivativeParams,
};
use anchor_lang::prelude::*;
use dex::utils::validation::assert;
use solana_program::{
    entrypoint::ProgramResult, program_error::ProgramError, sysvar::clock::Clock,
};

pub fn process(
    context: Context<InitializeDerivative>,
    params: InitializeDerivativeParams,
) -> ProgramResult {
    let accts = context.accounts;
    let mut derivative_metadata = accts.derivative_metadata.load_init()?;
    assert(
        !derivative_metadata.is_initialized(),
        DerivativeError::AccountAlreadyInitialized,
    )?;
    let clock: Clock = bincode::deserialize(&accts.clock.data.borrow())
        .map_err(|_| ProgramError::InvalidArgument)?;
    assert(
        params.initialization_time >= clock.unix_timestamp,
        DerivativeError::InvalidCreationTime,
    )?;
    derivative_metadata.tag = AccountTag::DerivativeMetadata;

    match params.instrument_type {
        InstrumentType::ExpiringCall | InstrumentType::ExpiringPut => {
            assert(
                params.full_funding_period == params.minimum_funding_period,
                DerivativeError::InvalidSettlementTime,
            )?;
        }
        _ => {}
    }

    // Immutable fields
    derivative_metadata.bump = context.bumps["derivative_metadata"] as u64;
    derivative_metadata.instrument_type = params.instrument_type;
    derivative_metadata.strike = params.strike;
    derivative_metadata.initialization_time = params.initialization_time;
    derivative_metadata.full_funding_period = params.full_funding_period;
    derivative_metadata.minimum_funding_period = params.minimum_funding_period;
    derivative_metadata.close_authority = params.close_authority;
    derivative_metadata.market_product_group = *accts.market_product_group.key;
    derivative_metadata.price_oracle = *accts.price_oracle.key;
    derivative_metadata.clock = *accts.clock.key;
    derivative_metadata.oracle_type = params.oracle_type;
    // Mutable fields
    derivative_metadata.expired = ExpirationStatus::Active;
    derivative_metadata.last_funding_time = params.initialization_time;
    Ok(())
}
