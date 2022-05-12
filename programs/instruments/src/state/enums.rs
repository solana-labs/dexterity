use borsh::{BorshDeserialize, BorshSerialize};
use bytemuck::{Pod, Zeroable};
use dex::error::UtilError;

#[derive(Copy, Clone, Debug, PartialEq)]
#[repr(u64)]
pub enum AccountTag {
    Uninitialized,
    DerivativeMetadata,
    FixedIncomeMetadata,
}
impl Default for AccountTag {
    fn default() -> Self {
        AccountTag::Uninitialized
    }
}
unsafe impl Zeroable for AccountTag {}
unsafe impl Pod for AccountTag {}

#[derive(BorshSerialize, BorshDeserialize, Copy, Debug, Clone, PartialEq)]
#[repr(u64)]
pub enum InstrumentType {
    Uninitialized,
    RecurringCall,
    RecurringPut,
    ExpiringCall,
    ExpiringPut,
}
impl Default for InstrumentType {
    fn default() -> Self {
        InstrumentType::Uninitialized
    }
}
unsafe impl Zeroable for InstrumentType {}
unsafe impl Pod for InstrumentType {}

impl InstrumentType {
    pub fn is_recurring(&self) -> std::result::Result<bool, UtilError> {
        match self {
            InstrumentType::RecurringCall | InstrumentType::RecurringPut => Ok(true),
            InstrumentType::ExpiringCall | InstrumentType::ExpiringPut => Ok(false),
            InstrumentType::Uninitialized => Err(UtilError::AccountUninitialized),
        }
    }
}

#[derive(BorshSerialize, BorshDeserialize, Copy, Debug, Clone, PartialEq)]
#[repr(u64)]
pub enum OracleType {
    Uninitialized,
    Pyth,
    Dummy,
}
impl Default for OracleType {
    fn default() -> Self {
        OracleType::Uninitialized
    }
}
unsafe impl Zeroable for OracleType {}
unsafe impl Pod for OracleType {}

#[derive(Copy, Clone, Debug, PartialEq)]
#[repr(u64)]
pub enum ExpirationStatus {
    Active,
    Expired,
}
impl Default for ExpirationStatus {
    fn default() -> Self {
        ExpirationStatus::Active
    }
}
unsafe impl Zeroable for ExpirationStatus {}
unsafe impl Pod for ExpirationStatus {}
