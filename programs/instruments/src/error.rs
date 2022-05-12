use dex::error::DomainOrProgramError;
use num_derive::FromPrimitive;
use solana_program::{decode_error::DecodeError, program_error::ProgramError};
use thiserror::Error;

#[derive(Error, Debug, Copy, Clone, FromPrimitive, PartialEq)]
pub enum DerivativeError {
    #[error("AccountAlreadyInitialized")]
    AccountAlreadyInitialized,
    #[error("InvalidSettlementTime")]
    InvalidSettlementTime,
    #[error("InvalidCreationTime")]
    InvalidCreationTime,
    #[error("UninitializedAccount")]
    UninitializedAccount,
    #[error("InvalidSequenceNumber")]
    InvalidSequenceNumber,
    #[error("UnsettledAccounts")]
    UnsettledAccounts,
    #[error("InvalidOracleConfig")]
    InvalidOracleConfig,
    #[error("NumericalOverflow")]
    NumericalOverflow,
    #[error("CannotBeDeleted")]
    CannotBeDeleted,
    #[error("ContractIsExpired")]
    ContractIsExpired,
    #[error("InvalidDate")]
    InvalidDate,
    #[error("InvalidAccount")]
    InvalidAccount,
}

impl From<DerivativeError> for ProgramError {
    fn from(e: DerivativeError) -> Self {
        ProgramError::Custom(e as u32)
    }
}

impl<T> DecodeError<T> for DerivativeError {
    fn type_of() -> &'static str {
        "DerivativeError"
    }
}

impl From<DerivativeError> for DomainOrProgramError {
    fn from(e: DerivativeError) -> Self {
        DomainOrProgramError::Other {
            code: e as u32,
            msg: format!("{}", e),
        }
    }
}
