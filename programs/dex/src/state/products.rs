use std::{
    mem::size_of,
    ops::{Deref, DerefMut},
};

use anchor_lang::prelude::*;
use bytemuck::{Pod, Zeroable};
use serde::{Deserialize, Serialize};

use crate::{
    error::{DexError, DomainOrProgramResult},
    state::{constants::MAX_LEGS, enums::ProductStatus, market_product_group::PriceEwma},
    utils::{numeric::ZERO_FRAC, TwoIterators},
    DomainOrProgramError, Fractional, NAME_LEN,
};

#[derive(
    Eq, Debug, PartialEq, Clone, Copy, AnchorDeserialize, AnchorSerialize, Deserialize, Serialize,
)]
#[repr(C, u64)]
/// Unify Outright and Combo
pub enum Product {
    Outright { outright: Outright },
    Combo { combo: Combo },
}

unsafe impl Pod for Product {}

impl Product {
    pub fn get_best_bid(&self) -> Fractional {
        self.prices.bid
    }

    pub fn get_best_ask(&self) -> Fractional {
        self.prices.ask
    }

    pub fn get_prev_best_bid(&self, slot: u64) -> Fractional {
        if slot > self.prices.slot {
            self.prices.bid
        } else {
            self.prices.prev_bid
        }
    }

    pub fn get_prev_best_ask(&self, slot: u64) -> Fractional {
        if slot > self.prices.slot {
            self.prices.ask
        } else {
            self.prices.prev_ask
        }
    }

    pub fn try_to_combo(&self) -> DomainOrProgramResult<&Combo> {
        match self {
            Product::Outright { outright: _ } => Err(DexError::ProductNotCombo.into()),
            Product::Combo { combo: c } => Ok(c),
        }
    }

    pub fn try_to_outright(&self) -> DomainOrProgramResult<&Outright> {
        match self {
            Product::Outright { outright: o } => Ok(o),
            Product::Combo { combo: _ } => Err(DexError::ProductNotOutright.into()),
        }
    }

    pub fn try_to_combo_mut(&mut self) -> DomainOrProgramResult<&mut Combo> {
        match self {
            Product::Outright { outright: _ } => Err(DexError::ProductNotCombo.into()),
            Product::Combo { combo: c } => Ok(c),
        }
    }

    pub fn try_to_outright_mut(&mut self) -> DomainOrProgramResult<&mut Outright> {
        match self {
            Product::Outright { outright: o } => Ok(o),
            Product::Combo { combo: _ } => Err(DexError::ProductNotOutright.into()),
        }
    }

    pub fn get_ratios_and_product_indices(
        &self,
        product_idx: usize,
    ) -> impl Iterator<Item = (i64, usize)> + '_ {
        match self {
            Product::Outright { outright: _ } => TwoIterators::A(([(1, product_idx)]).into_iter()),
            Product::Combo { combo: c } => TwoIterators::B(
                c.legs
                    .iter()
                    .take(c.num_legs)
                    .map(|leg| (leg.ratio, leg.product_index)),
            ),
        }
    }

    #[inline]
    pub fn is_combo(&self) -> bool {
        match self {
            Product::Outright { outright: _ } => false,
            Product::Combo { combo: _ } => true,
        }
    }
}

#[zero_copy]
#[derive(Debug, Eq, PartialEq, AnchorSerialize, AnchorDeserialize, Deserialize, Serialize)] // serde
/// A market product corresponding to one underlying asset
pub struct Outright {
    pub metadata: ProductMetadata,
    pub num_queue_events: usize,
    pub product_status: ProductStatus,
    pub dust: Fractional,
    pub cum_funding_per_share: Fractional,
    pub cum_social_loss_per_share: Fractional,
    pub open_long_interest: Fractional,
    pub open_short_interest: Fractional,
    pub padding: [u64; 14],
}

impl Outright {
    pub fn apply_new_funding(
        &mut self,
        amount_per_share: Fractional,
        cash_decimals: u64,
    ) -> std::result::Result<(), DomainOrProgramError> {
        self.cum_funding_per_share += amount_per_share;
        let target_decimals = (self.base_decimals as i64) - (cash_decimals as i64);
        if self.cum_funding_per_share.has_precision(target_decimals) {
            Ok(())
        } else {
            Err(DexError::FundingPrecisionError.into())
        }
    }

    pub fn apply_social_loss(
        &mut self,
        loss: Fractional,
        cash_decimals: u64,
    ) -> std::result::Result<(), DomainOrProgramError> {
        self.dust = (self.dust + loss).round_unchecked(cash_decimals as u32)?;
        let open_interest = (self.open_long_interest + self.open_short_interest)
            .round_unchecked(self.base_decimals as u32)?;
        if open_interest != ZERO_FRAC {
            let multiplier = self.dust.m / open_interest.m;
            self.dust.m %= open_interest.m;
            self.cum_social_loss_per_share += Fractional {
                m: multiplier * 10_i64.pow(self.base_decimals as u32),
                exp: cash_decimals,
            };
        }
        Ok(())
    }

    pub fn is_removable(&self) -> bool {
        self.open_long_interest == ZERO_FRAC && self.open_short_interest == ZERO_FRAC
    }

    pub fn is_expired(&self) -> bool {
        self.product_status == ProductStatus::Expired
    }

    pub fn update_open_interest_change(
        &mut self,
        trade_size: Fractional,
        buyer_short_position: Fractional,
        seller_long_position: Fractional,
    ) -> DomainOrProgramResult {
        match (
            buyer_short_position < trade_size,
            seller_long_position < trade_size,
        ) {
            (true, true) => {
                self.open_long_interest = self
                    .open_long_interest
                    .checked_add(trade_size)?
                    .checked_sub(buyer_short_position)?
                    .checked_sub(seller_long_position)?;
            }
            (true, false) => {
                self.open_long_interest =
                    self.open_long_interest.checked_sub(buyer_short_position)?;
            }
            (false, true) => {
                self.open_long_interest =
                    self.open_long_interest.checked_sub(seller_long_position)?;
            }
            (false, false) => {
                self.open_long_interest = self.open_long_interest.checked_sub(trade_size)?;
            }
        };
        self.open_short_interest = self.open_long_interest;
        Ok(())
    }
}

#[zero_copy]
#[derive(Debug, Eq, PartialEq, Pod, AnchorSerialize, AnchorDeserialize, Deserialize, Serialize)] // serde
/// Shared fields between Outright and Combo products
pub struct ProductMetadata {
    pub bump: u64,
    pub product_key: Pubkey,
    pub name: [u8; NAME_LEN],
    pub orderbook: Pubkey,
    // Negative+Fractional Price
    pub tick_size: Fractional,
    pub base_decimals: u64,
    pub price_offset: Fractional,
    pub contract_volume: Fractional,
    // Prices
    pub prices: PriceEwma,
}

unsafe impl Zeroable for ProductMetadata {}

#[zero_copy]
#[derive(Debug, Eq, PartialEq, AnchorSerialize, AnchorDeserialize, Deserialize, Serialize)] // serde
/// A market product with multiple legs that are each outrights
pub struct Combo {
    pub metadata: ProductMetadata,
    pub num_legs: usize,
    pub legs: [Leg; MAX_LEGS],
}

impl Default for Combo {
    fn default() -> Self {
        unsafe { std::mem::zeroed() }
    }
}

impl Combo {
    pub fn legs(&self) -> &[Leg] {
        &self.legs[..self.num_legs]
    }

    pub fn has_leg(&self, product_key: Pubkey) -> bool {
        self.legs
            .iter()
            .take(self.num_legs)
            .any(|l| l.product_key == product_key)
    }

    pub fn get_product_key_seeds(&self) -> Vec<u8> {
        let mut seeds = Vec::<u8>::with_capacity((size_of::<Pubkey>() + 1) * self.num_legs);
        for leg in self.legs.iter().take(self.num_legs) {
            seeds.extend(leg.product_key.to_bytes().iter());
        }
        for leg in self.legs.iter().take(self.num_legs) {
            seeds.extend((leg.ratio as i8).to_le_bytes().iter());
        }
        seeds
    }
}

#[zero_copy]
#[derive(
    Debug, Default, Eq, AnchorSerialize, AnchorDeserialize, PartialEq, Deserialize, Serialize,
)] // serde
/// One part of a combo. Each leg corresponds to an outright with the ratio determining
/// relative weighting
pub struct Leg {
    pub product_index: usize,
    pub product_key: Pubkey,
    pub ratio: i64,
}

impl Deref for Outright {
    type Target = ProductMetadata;

    fn deref(&self) -> &Self::Target {
        &self.metadata
    }
}

impl DerefMut for Outright {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.metadata
    }
}

impl Deref for Combo {
    type Target = ProductMetadata;

    fn deref(&self) -> &Self::Target {
        &self.metadata
    }
}

impl DerefMut for Combo {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.metadata
    }
}

impl Default for Outright {
    fn default() -> Self {
        unsafe { std::mem::zeroed() }
    }
}

unsafe impl Zeroable for Product {}

impl DerefMut for Product {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self {
            Product::Outright { outright: x } => &mut x.metadata,
            Product::Combo { combo: x } => &mut x.metadata,
        }
    }
}

impl Deref for Product {
    type Target = ProductMetadata;

    fn deref(&self) -> &Self::Target {
        match self {
            Product::Outright { outright: x } => &x.metadata,
            Product::Combo { combo: x } => &x.metadata,
        }
    }
}

impl Default for Product {
    fn default() -> Self {
        unsafe { std::mem::zeroed() }
    }
}
