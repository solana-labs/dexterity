use num_derive::FromPrimitive;
use std::{error::Error, fmt::Formatter};

use crate::DomainOrProgramError::ProgramErr;
use anchor_lang::solana_program::{
    decode_error::DecodeError, program_error::ProgramError, pubkey::PubkeyError,
};
use thiserror::Error;

pub type DomainOrProgramResult<T = ()> = std::result::Result<T, DomainOrProgramError>;

#[derive(Error, Debug, Clone, PartialEq)]
pub enum DomainOrProgramError {
    DexErr(DexError),
    UtilErr(UtilError),
    ProgramErr(ProgramError),
    Other { code: u32, msg: String },
}

#[derive(Error, Debug, Copy, Clone, FromPrimitive, PartialEq)]
pub enum UtilError {
    #[error("AccountAlreadyInitialized")]
    AccountAlreadyInitialized,
    #[error("AccountUninitialized")]
    AccountUninitialized,
    #[error("DuplicateProductKey")]
    DuplicateProductKey,
    #[error("PublicKeyMismatch")]
    PublicKeyMismatch,
    #[error("AssertionError")]
    AssertionError,
    #[error("InvalidMintAuthority")]
    InvalidMintAuthority,
    #[error("IncorrectOwner")]
    IncorrectOwner,
    #[error("PublicKeysShouldBeUnique")]
    PublicKeysShouldBeUnique,
    #[error("NotRentExempt")]
    NotRentExempt,
    #[error("NumericalOverflow")]
    NumericalOverflow,
    #[error("Rounding loses precision")]
    RoundError,
    #[error("Division by zero")]
    DivisionbyZero,
    #[error("Invalid return value")]
    InvalidReturnValue,
    #[error("Negative Number Sqrt")]
    SqrtRootError,
    #[error("Zero Price Error")]
    ZeroPriceError,
    #[error("Zero Quantity Error")]
    ZeroQuantityError,
    #[error("Serialization Error")]
    SerializeError,
    #[error("Deerialization Error")]
    DeserializeError,
    #[error("Invalid index for bitset")]
    InvalidBitsetIndex,
}

#[derive(Error, Debug, Copy, Clone, FromPrimitive, PartialEq)]
pub enum DexError {
    #[error("ContractIsExpired")]
    ContractIsExpired,
    #[error("ContractIsNotExpired")]
    ContractIsNotExpired,
    #[error("Invalid system program account provided")]
    InvalidSystemProgramAccount,
    #[error("Invalid AOB program account provided")]
    InvalidAobProgramAccount,
    #[error("A provided state account was not owned by the current program")]
    InvalidStateAccountOwner,
    #[error("The given order index is invalid.")]
    InvalidOrderIndex,
    #[error("The user account has reached its maximum capacity for open orders.")]
    UserAccountFull,
    #[error("The transaction has been aborted.")]
    TransactionAborted,
    #[error("A required user account is missing.")]
    MissingUserAccount,
    #[error("The specified order has not been found.")]
    OrderNotFound,
    #[error("The operation is a no-op")]
    NoOp,
    #[error("The user does not own enough lamports")]
    OutofFunds,
    #[error("The user account is still active")]
    UserAccountStillActive,
    #[error("Market is still active")]
    MarketStillActive,
    #[error("Invalid market signer provided")]
    InvalidMarketSignerAccount,
    #[error("Invalid orderbook account provided")]
    InvalidOrderbookAccount,
    #[error("Invalid market admin account provided")]
    InvalidMarketAdminAccount,
    #[error("Invalid base vault account provided")]
    InvalidBaseVaultAccount,
    #[error("Invalid quote vault account provided")]
    InvalidQuoteVaultAccount,
    #[error("Market product group has no empty slot")]
    FullMarketProductGroup,
    #[error("Missing Market Product")]
    MissingMarketProduct,
    #[error("Invalid Withdrawal Amount")]
    InvalidWithdrawalAmount,
    #[error("Taker Trader has no product")]
    InvalidTakerTrader,
    #[error("Funds negative or fraction")]
    FundsError,
    #[error("Product is inactive")]
    InactiveProductError,
    #[error("Too many open orders")]
    TooManyOpenOrdersError,
    #[error("No more open orders")]
    NoMoreOpenOrdersError,
    #[error("Non zero price tick exponent")]
    NonZeroPriceTickExponentError,
    #[error("Duplicate product name")]
    DuplicateProductNameError,
    #[error("Invalid Risk Response")]
    InvalidRiskResponseError,
    #[error("Invalid Operation for Account Health")]
    InvalidAccountHealthError,
    #[error("Orderbook is empty")]
    OrderbookIsEmptyError,
    #[error("Combos not removed for expired product")]
    CombosNotRemoved,
    #[error("Trader risk group is not liquidable")]
    AccountNotLiquidable,
    #[error("Funding precision is more granular than the limit")]
    FundingPrecisionError,
    #[error("Product decimal precision error")]
    ProductDecimalPrecisionError,
    #[error("Expected product to be an outright product")]
    ProductNotOutright,
    #[error("Expected product to be a combo product")]
    ProductNotCombo,
    #[error("Risk engine returned an invalid social loss vector")]
    InvalidSocialLossCalculation,
    #[error("Risk engine returned invalid product indices in social loss vector")]
    ProductIndexMismatch,
    #[error("Invalid order ID")]
    InvalidOrderID,
    #[error("Invalid bytes for zero-copy deserialization")]
    InvalidBytesForZeroCopyDeserialization,
}

impl From<UtilError> for ProgramError {
    fn from(e: UtilError) -> Self {
        ProgramError::Custom(e as u32)
    }
}

impl<T> DecodeError<T> for UtilError {
    fn type_of() -> &'static str {
        "UtilError"
    }
}

impl std::fmt::Display for DomainOrProgramError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            DomainOrProgramError::ProgramErr(p) => write!(f, "{}", p),
            DomainOrProgramError::UtilErr(p) => write!(f, "{}", p),
            DomainOrProgramError::DexErr(p) => write!(f, "{}", p),
            DomainOrProgramError::Other { code, msg } => {
                write!(f, "DomainOrProgramError::Other code: {} msg: {}", code, msg)
            }
        }
    }
}

impl From<DomainOrProgramError> for ProgramError {
    fn from(e: DomainOrProgramError) -> Self {
        match e {
            DomainOrProgramError::DexErr(e) => e.into(),
            DomainOrProgramError::UtilErr(e) => e.into(),
            DomainOrProgramError::Other { code, msg: _ } => ProgramError::Custom(code),
            DomainOrProgramError::ProgramErr(e) => e,
        }
    }
}

impl From<DexError> for ProgramError {
    fn from(e: DexError) -> Self {
        ProgramError::Custom(e as u32)
    }
}

impl From<anchor_lang::error::Error> for DomainOrProgramError {
    fn from(e: anchor_lang::error::Error) -> Self {
        ProgramError::from(e).into()
    }
}

impl From<PubkeyError> for DomainOrProgramError {
    fn from(e: PubkeyError) -> Self {
        ProgramError::from(e).into()
    }
}

impl From<ProgramError> for DomainOrProgramError {
    fn from(e: ProgramError) -> Self {
        DomainOrProgramError::ProgramErr(e)
    }
}

impl From<DexError> for DomainOrProgramError {
    fn from(e: DexError) -> Self {
        DomainOrProgramError::DexErr(e)
    }
}

impl From<UtilError> for DomainOrProgramError {
    fn from(e: UtilError) -> Self {
        DomainOrProgramError::UtilErr(e)
    }
}

impl<T> DecodeError<T> for DexError {
    fn type_of() -> &'static str {
        "DexError"
    }
}
