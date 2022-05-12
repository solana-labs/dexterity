use agnostic_orderbook::state::OrderSummary;
use anchor_lang::prelude::*;
#[event]
pub struct DexOrderSummary {
    pub posted_order_id: Option<u128>,
    pub total_base_qty: u64,
    pub total_quote_qty: u64,
    pub total_base_qty_posted: u64,
}

impl DexOrderSummary {
    pub fn new(
        posted_order_id: Option<u128>,
        total_base_qty: u64,
        total_quote_qty: u64,
        total_base_qty_posted: u64,
    ) -> Self {
        DexOrderSummary {
            posted_order_id,
            total_base_qty,
            total_quote_qty,
            total_base_qty_posted,
        }
    }
    pub fn from(order_summary: &OrderSummary) -> Self {
        DexOrderSummary::new(
            order_summary.posted_order_id,
            order_summary.total_base_qty,
            order_summary.total_quote_qty,
            order_summary.total_base_qty_posted,
        )
    }
}
