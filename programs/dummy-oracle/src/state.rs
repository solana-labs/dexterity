use borsh::{BorshDeserialize, BorshSerialize};

use solana_program::pubkey::Pubkey;

#[derive(BorshDeserialize, BorshSerialize, Copy, Clone, Debug, PartialEq)]
#[repr(u8)]
pub enum AccountTag {
    Uninitialized,
    OraclePrice,
}

#[repr(C)]
#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct OraclePrice {
    pub tag: AccountTag,
    pub price: i64,
    pub decimals: u64,
    pub slot: u64,
    pub update_authority: Pubkey,
}

impl OraclePrice {
    pub const LEN: u64 = 1 // tag
  + 8  // price
  + 8  // decimals
  + 8  // slot
  + 32 // update_authority
  ;

    pub fn is_initialized(&self) -> bool {
        self.tag == AccountTag::OraclePrice
    }
}
