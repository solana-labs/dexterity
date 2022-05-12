use bytemuck::{Pod, Zeroable};

use crate::utils::{
    loadable::Loadable,
    numeric::{Fractional, ZERO_FRAC},
};

use agnostic_orderbook::state::Side;

use crate::{
    error::{DomainOrProgramError, DomainOrProgramResult},
    state::constants::{HEALTH_BUFFER_LEN, MAX_OUTRIGHTS, MAX_TRADER_POSITIONS},
};

use anchor_lang::prelude::*;

#[derive(Copy, AnchorSerialize, AnchorDeserialize, Clone, PartialEq, Debug)]
#[repr(u8)]
#[allow(missing_docs)]
pub enum OperationType {
    NewOrder,
    CancelOrder,
    CheckHealth,
    PositionTransfer,
    ConsumeEvents,
}

#[account(zero_copy)]
pub struct RiskOutputRegister {
    pub risk_engine_output: HealthResult,
}

#[derive(Copy, AnchorSerialize, AnchorDeserialize, Clone, PartialEq, Debug)]
pub enum HealthResult {
    Health { health_info: HealthInfo },
    Liquidation { liquidation_info: LiquidationInfo },
}

#[derive(Copy, AnchorSerialize, AnchorDeserialize, Clone, PartialEq, Debug)]
pub struct HealthInfo {
    pub health: HealthStatus,
    pub action: ActionStatus,
}

#[derive(Copy, AnchorSerialize, AnchorDeserialize, Clone, PartialEq, Debug)]
pub struct LiquidationInfo {
    pub health: HealthStatus,
    pub action: ActionStatus,
    pub total_social_loss: Fractional,
    // Risk Engine mark price of portfolio
    // Price liquidator pays to take on position
    /*
    A: Liquidatee, B: Liquidator
    tsl = 20
    A's liquidation_price = 180
          A's final cash = 180 - 20 = 160
    Before
       A: -1000 Cash +X Foo
       B: 500 Cash
    After
       A: 160 Cash 0 Foo
       B: 500 (start) - 180 (price) - 1000 (A's cash) = -680
          -680 Cash +X Foo

    liquidator_cash_profit = price + A's cash
    liquidation_price = price (Only risk knows this)
     */
    pub liquidation_price: Fractional,
    /*
       // Pseudocode
       assert_eq(
           social_losses.iter().map(|x| x.amount).sum(), total_social_loss
       )?;
       for social_loss in social_losses.iter().enumerate() {
           let mut product = mpg.get(social_loss.product_index);
           product.apply_social_loss(social_loss.amount)?;
       }
    */
    pub social_losses: [SocialLoss; MAX_TRADER_POSITIONS],
}

#[derive(Copy, AnchorSerialize, AnchorDeserialize, Clone, PartialEq, Debug)]
pub enum HealthStatus {
    Healthy,
    /*
            1. Allows all orders to be cancelled
            2. transfer_position is blocked
    */
    Unhealthy,
    /*
        1. Allows all orders to be cancelled
        2. transfer_position is allowed
        3. All posts are blocked
    */
    Liquidatable,
    NotLiquidatable,
}

#[derive(Copy, AnchorSerialize, AnchorDeserialize, Clone, PartialEq, Debug)]
#[repr(u8)]
pub enum ActionStatus {
    Approved,
    NotApproved,
}

#[derive(Copy, AnchorSerialize, AnchorDeserialize, Clone, PartialEq, Debug)]
pub struct SocialLoss {
    pub product_index: usize,
    pub amount: Fractional,
}

impl Default for SocialLoss {
    fn default() -> Self {
        SocialLoss {
            product_index: 0,
            amount: ZERO_FRAC,
        }
    }
}

impl SocialLoss {
    pub fn is_active(&self) -> bool {
        self.product_index < MAX_OUTRIGHTS && self.amount != ZERO_FRAC
    }
}

#[derive(Copy, AnchorSerialize, AnchorDeserialize, Clone, PartialEq, Debug)]
pub struct OrderInfo {
    pub total_order_qty: Fractional,
    pub matched_order_qty: Fractional,
    pub order_side: Side,
    pub is_combo: bool,
    pub product_index: usize,
    pub operation_type: OperationType,
    pub old_ask_qty_in_book: Fractional,
    pub old_bid_qty_in_book: Fractional,
}

impl Default for OrderInfo {
    fn default() -> Self {
        OrderInfo {
            total_order_qty: ZERO_FRAC,
            matched_order_qty: ZERO_FRAC,
            order_side: Side::Bid,
            is_combo: false,
            product_index: 0,
            operation_type: OperationType::CheckHealth,
            old_ask_qty_in_book: ZERO_FRAC,
            old_bid_qty_in_book: ZERO_FRAC,
        }
    }
}
