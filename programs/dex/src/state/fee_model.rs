use crate::{
    utils::{loadable::Loadable, numeric::bps},
    Fractional, MarketProductGroup,
};
use agnostic_orderbook::state::Side;
use anchor_lang::{prelude::*, solana_program::clock::UnixTimestamp};
use bytemuck::{Pod, Zeroable};

#[repr(C)]
#[zero_copy]
#[derive(Debug, Zeroable, Pod)]
pub struct TraderFees {
    pub valid_until: UnixTimestamp,
    pub maker_fee_bps: i32,
    pub taker_fee_bps: i32,
}

#[derive(Copy, Clone, Debug, AnchorDeserialize, AnchorSerialize)]
pub struct TraderFeeParams {
    pub side: Side,
    pub is_aggressor: bool,
    pub matched_quote_qty: Fractional,
    pub matched_base_qty: Fractional,
    pub product: Pubkey,
}

// 10_000 bps == 100%
const MAX_FEE_BPS: i32 = 10_000;
const MIN_FEE_BPS: i32 = -10_000;

fn clamp_fees(fee: i32) -> i32 {
    within_or_zero(fee, MAX_FEE_BPS, MIN_FEE_BPS)
}

fn within_or_zero(x: i32, max: impl Into<i32>, min: impl Into<i32>) -> i32 {
    if x > max.into() || x < min.into() {
        0
    } else {
        x
    }
}

impl TraderFees {
    pub fn new(maker_fee_bps: i32, taker_fee_bps: i32, valid_until: UnixTimestamp) -> Self {
        Self {
            valid_until,
            maker_fee_bps,
            taker_fee_bps,
        }
    }

    pub fn maker_fee_bps(&self, market_product_group: Option<&MarketProductGroup>) -> Fractional {
        let fee = market_product_group
            .map(|mpg| {
                within_or_zero(
                    self.maker_fee_bps,
                    mpg.max_maker_fee_bps,
                    mpg.min_maker_fee_bps,
                )
            })
            .unwrap_or(clamp_fees(self.maker_fee_bps));

        bps(fee as i64)
    }

    pub fn taker_fee_bps(&self, market_product_group: Option<&MarketProductGroup>) -> Fractional {
        let fee = market_product_group
            .map(|mpg| {
                within_or_zero(
                    self.taker_fee_bps,
                    mpg.max_taker_fee_bps,
                    mpg.min_taker_fee_bps,
                )
            })
            .unwrap_or(clamp_fees(self.taker_fee_bps));

        bps(fee as i64)
    }

    pub fn set_taker_fee_bps(&mut self, taker_fee_bps: i32) {
        self.taker_fee_bps = clamp_fees(taker_fee_bps);
    }

    pub fn set_maker_fee_bps(&mut self, maker_fee_bps: i32) {
        self.maker_fee_bps = clamp_fees(maker_fee_bps);
    }
}
