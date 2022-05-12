use anchor_lang::prelude::*;
use bytemuck::{Pod, Zeroable};
use serde::{Deserialize, Serialize};

use crate::utils::loadable::Loadable;

#[derive(
    Copy, AnchorSerialize, AnchorDeserialize, Clone, Debug, PartialEq, Deserialize, Serialize,
)]
#[repr(u64)]
pub enum AccountTag {
    Uninitialized,
    MarketProductGroup,
    TraderRiskGroup,
    TraderPosition,
    MarketProductGroupWithCombos,
    ComboGroup,
    Combo,
    RiskProfile,
}

impl Default for AccountTag {
    fn default() -> Self {
        AccountTag::Uninitialized
    }
}

unsafe impl Zeroable for AccountTag {}

unsafe impl Pod for AccountTag {}

impl AccountTag {
    pub fn to_bytes(&self) -> [u8; 8] {
        match self {
            AccountTag::Uninitialized => 0_u64.to_le_bytes(),
            AccountTag::MarketProductGroup => 1_u64.to_le_bytes(),
            AccountTag::TraderRiskGroup => 2_u64.to_le_bytes(),
            AccountTag::TraderPosition => 3_u64.to_le_bytes(),
            AccountTag::MarketProductGroupWithCombos => 4_u64.to_le_bytes(),
            AccountTag::ComboGroup => 5_u64.to_le_bytes(),
            AccountTag::Combo => 6_u64.to_le_bytes(),
            AccountTag::RiskProfile => 7_u64.to_le_bytes(),
        }
    }
}

#[derive(
    Eq, Copy, AnchorSerialize, AnchorDeserialize, Clone, Debug, PartialEq, Deserialize, Serialize,
)]
#[repr(u64)]
pub enum ProductStatus {
    Uninitialized,
    Initialized,
    Expired,
}

impl Default for ProductStatus {
    fn default() -> Self {
        ProductStatus::Uninitialized
    }
}

unsafe impl Zeroable for ProductStatus {}

unsafe impl Pod for ProductStatus {}

#[derive(AnchorDeserialize, AnchorSerialize, Debug, PartialEq, Clone, Deserialize, Serialize)] // serde
#[repr(u64)]
pub enum OrderType {
    Limit,
    ImmediateOrCancel,
    FillOrKill,
    PostOnly,
}
