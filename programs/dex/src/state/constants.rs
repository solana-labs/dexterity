use crate::utils::numeric::Fractional;

use anchor_lang::prelude::*;

#[constant]
pub const NAME_LEN: usize = 16;

#[constant]
pub const MAX_OUTRIGHTS: usize = 128;

#[constant]
pub const MAX_PRODUCTS: usize = 256;

#[constant]
pub const HEALTH_BUFFER_LEN: usize = 32;

#[constant]
pub const MAX_TRADER_POSITIONS: usize = 16;

#[constant]
pub const MAX_OPEN_ORDERS_PER_POSITION: u64 = 256;

#[constant]
pub const MAX_OPEN_ORDERS: usize = 1024;

#[constant]
pub const ANCHOR_DISCRIMINANT_LEN: usize = 8;

pub const NO_BID_PRICE: Fractional = Fractional {
    m: i64::MIN,
    exp: 0,
};

pub const NO_ASK_PRICE: Fractional = Fractional {
    m: i64::MAX,
    exp: 0,
};

#[constant]
pub const SENTINEL: usize = 0;

/// The length in bytes of the callback information in the associated asset agnostic orderbook
#[constant]
pub const CALLBACK_INFO_LEN: u64 = 40;
/// The length in bytes of the callback identifer prefix in the associated asset agnostic orderbook
#[constant]
pub const CALLBACK_ID_LEN: u64 = 32;

#[constant]
pub const MAX_COMBOS: usize = 128;

#[constant]
pub const MAX_LEGS: usize = 4;

// timing constants
#[constant]
pub const SLOTS_1_MIN: u64 = 150;

#[constant]
pub const SLOTS_5_MIN: u64 = 750;

#[constant]
pub const SLOTS_15_MIN: u64 = 2250;

#[constant]
pub const SLOTS_60_MIN: u64 = 9000;
