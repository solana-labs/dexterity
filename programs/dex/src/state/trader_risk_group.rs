use agnostic_orderbook::state::Side;
use anchor_lang::{
    prelude::*,
    solana_program::{
        clock::UnixTimestamp, msg, program_error::ProgramError, program_pack::IsInitialized,
        pubkey::Pubkey,
    },
};

use crate::{
    error::{DexError, DomainOrProgramError, DomainOrProgramResult},
    state::{
        constants::{
            HEALTH_BUFFER_LEN, MAX_COMBOS, MAX_OPEN_ORDERS_PER_POSITION, MAX_OUTRIGHTS,
            MAX_TRADER_POSITIONS,
        },
        enums::AccountTag,
        market_product_group::MarketProductGroup,
        open_orders::OpenOrders,
        products::Combo,
    },
    utils::{
        loadable::Loadable,
        numeric::{Fractional, ZERO_FRAC},
        validation::assert,
    },
};

#[account(zero_copy)]
/// State account corresponding to a trader on a given market product group
pub struct TraderRiskGroup {
    pub tag: AccountTag,
    pub market_product_group: Pubkey,
    pub owner: Pubkey,
    // Default value is 255 (max int) which corresponds to no position for the product at the corresponding index
    pub active_products: [u8; MAX_OUTRIGHTS],
    pub total_deposited: Fractional,
    pub total_withdrawn: Fractional,
    // Treat cash separately since it is collateral (unless we eventually support spot)
    pub cash_balance: Fractional,
    // Keep track of pending fills for risk calculations (only for takers)
    pub pending_cash_balance: Fractional,
    // Keep track of pending taker fees to be collected in consume_events
    pub pending_fees: Fractional,
    pub valid_until: UnixTimestamp,
    pub maker_fee_bps: i32,
    pub taker_fee_bps: i32,
    pub trader_positions: [TraderPosition; MAX_TRADER_POSITIONS],
    pub risk_state_account: Pubkey,
    pub fee_state_account: Pubkey,
    // Densely packed linked list of open orders
    pub client_order_id: u128,
    pub open_orders: OpenOrders,
}

impl IsInitialized for TraderRiskGroup {
    fn is_initialized(&self) -> bool {
        self.tag == AccountTag::TraderRiskGroup
    }
}

impl Default for TraderRiskGroup {
    fn default() -> Self {
        unsafe { std::mem::zeroed() }
    }
}

impl TraderRiskGroup {
    pub fn find_position_index(&self, position_pk: &Pubkey) -> Option<usize> {
        self.trader_positions
            .iter()
            .position(|pk| &pk.product_key == position_pk)
    }

    pub fn apply_funding(
        &mut self,
        market_product_group: &mut MarketProductGroup,
        trader_position_index: usize,
    ) -> DomainOrProgramResult {
        let trader_position = &mut self.trader_positions[trader_position_index];
        let product_index = trader_position.product_index;
        let market_product =
            market_product_group.market_products[product_index].try_to_outright_mut()?;
        let funding_updated =
            trader_position.last_cum_funding_snapshot != market_product.cum_funding_per_share;
        let social_loss_updated =
            trader_position.last_social_loss_snapshot != market_product.cum_social_loss_per_share;
        if funding_updated || social_loss_updated {
            if !market_product.is_expired() || market_product.num_queue_events == 0 {
                let amount_owed: Fractional = market_product
                    .cum_funding_per_share
                    .checked_sub(trader_position.last_cum_funding_snapshot)?
                    .checked_add(trader_position.last_social_loss_snapshot)?
                    .checked_sub(market_product.cum_social_loss_per_share)?
                    .checked_mul(trader_position.position)?;
                self.cash_balance = self.cash_balance.checked_add(amount_owed)?;
                trader_position.last_cum_funding_snapshot = market_product.cum_funding_per_share;
                trader_position.last_social_loss_snapshot =
                    market_product.cum_social_loss_per_share;
            }
        }
        if market_product.is_expired() && market_product.num_queue_events == 0 {
            let product_key = trader_position.product_key;
            if trader_position.position > ZERO_FRAC {
                market_product.open_long_interest -= trader_position.position;
            } else {
                market_product.open_short_interest += trader_position.position;
            }
            self.open_orders.clear(product_index)?;
            for (combo_index, combo) in market_product_group.active_combos() {
                if combo.has_leg(product_key) {
                    self.open_orders.clear(combo_index)?;
                }
            }
            self.clear(product_key)?;
        }
        Ok(())
    }

    pub fn compute_unsettled_funding(
        &self,
        market_product_group: &MarketProductGroup,
    ) -> std::result::Result<Fractional, DomainOrProgramError> {
        let mut funding = ZERO_FRAC;
        for trader_index in 0..self.trader_positions.len() {
            let position = self.trader_positions[trader_index];
            if !position.is_initialized() {
                continue;
            }
            let idx = position.product_index;
            let market_product = market_product_group.market_products[idx].try_to_outright()?;
            let amount_owed: Fractional = market_product
                .cum_funding_per_share
                .checked_sub(position.last_cum_funding_snapshot)?
                .checked_add(position.last_social_loss_snapshot)?
                .checked_sub(market_product.cum_social_loss_per_share)?
                .checked_mul(position.position)?;
            funding = funding.checked_add(amount_owed)?;
        }
        Ok(funding)
    }

    pub fn apply_all_funding(
        &mut self,
        market_product_group: &mut MarketProductGroup,
    ) -> DomainOrProgramResult {
        for trader_index in 0..self.trader_positions.len() {
            if !self.trader_positions[trader_index].is_initialized() {
                continue;
            }
            self.apply_funding(market_product_group, trader_index)?;
        }
        Ok(())
    }

    pub fn add_open_order(&mut self, index: usize, order_id: u128) -> DomainOrProgramResult {
        // TODO: consider reinstating is_active check at some point
        let num_open_orders = self.open_orders.products[index].num_open_orders;

        assert(
            num_open_orders < MAX_OPEN_ORDERS_PER_POSITION,
            DexError::TooManyOpenOrdersError,
        )?;

        self.open_orders.products[index].num_open_orders += 1;
        self.open_orders.total_open_orders += 1;
        self.open_orders
            .add_open_order(index, order_id)
            .map_err(Into::into)
    }

    pub fn remove_open_order(&mut self, index: usize, order_id: u128) -> DomainOrProgramResult {
        // TODO: consider reinstating is_active check at some point
        let num_open_orders = self.open_orders.products[index].num_open_orders;
        assert(num_open_orders > 0, DexError::NoMoreOpenOrdersError)?;

        // msg!("Removing order index {}", index);
        self.open_orders.products[index].num_open_orders -= 1;
        msg!(
            "Remaining open orders {}",
            self.open_orders.products[index].num_open_orders
        );
        self.open_orders.total_open_orders = self.open_orders.total_open_orders.saturating_sub(1);
        self.open_orders
            .remove_open_order(index, order_id)
            .map_err(Into::into)
    }

    pub fn activate_if_uninitialized<'a>(
        &mut self,
        product_index: usize,
        product_key: &Pubkey,
        funding: Fractional,
        social_loss: Fractional,
        active_combo_products: impl Iterator<Item = (usize, &'a Combo)>,
    ) -> DomainOrProgramResult {
        if self.is_active_product(product_index)? {
            return Ok(());
        }
        let has_uninitialized_positions = self.trader_positions.iter().any(|p| !p.is_initialized());
        let combos_with_open_orders: Vec<(usize, &Combo)> = if !has_uninitialized_positions {
            active_combo_products
                .filter(|(idx, _)| self.open_orders.products[*idx].num_open_orders > 0)
                .collect::<Vec<_>>()
        } else {
            vec![]
        };

        for (trader_position_index, trader_position) in self.trader_positions.iter_mut().enumerate()
        {
            // try to replace empty position if possible
            if trader_position.is_initialized() {
                if has_uninitialized_positions {
                    continue;
                }
                if trader_position.is_active() {
                    continue;
                }
                if self.open_orders.products[trader_position.product_index].num_open_orders > 0 {
                    continue;
                }
                if combos_with_open_orders
                    .iter()
                    .any(|(_, c)| c.has_leg(trader_position.product_key))
                {
                    continue;
                }
                msg!("Replacing unused trader position");
            }
            self.active_products[product_index] = trader_position_index as u8;
            trader_position.tag = AccountTag::TraderPosition;
            trader_position.product_key = *product_key;
            trader_position.product_index = product_index;
            trader_position.last_cum_funding_snapshot = funding;
            trader_position.last_social_loss_snapshot = social_loss;
            return Ok(());
        }
        msg!("All trader positions are occupied");
        Err(ProgramError::InvalidAccountData.into())
    }

    pub fn adjust_book_qty(
        &mut self,
        product_index: usize,
        qty: Fractional,
        side: Side,
    ) -> DomainOrProgramResult {
        let open_orders = &mut self.open_orders.products[product_index];

        match side {
            Side::Bid => {
                open_orders.bid_qty_in_book = open_orders.bid_qty_in_book.checked_add(qty)?
            }
            Side::Ask => {
                open_orders.ask_qty_in_book = open_orders.ask_qty_in_book.checked_add(qty)?
            }
        }
        Ok(())
    }

    pub fn decrement_book_size(
        &mut self,
        product_index: usize,
        side: Side,
        qty: Fractional,
    ) -> DomainOrProgramResult {
        let open_orders = &mut self.open_orders.products[product_index];

        match side {
            Side::Bid => {
                open_orders.bid_qty_in_book = open_orders.bid_qty_in_book.checked_sub(qty)?
            }
            Side::Ask => {
                open_orders.ask_qty_in_book = open_orders.ask_qty_in_book.checked_sub(qty)?
            }
        }
        Ok(())
    }

    pub fn is_active_product(
        &self,
        index: usize,
    ) -> std::result::Result<bool, DomainOrProgramError> {
        if !self.is_initialized() {
            msg!("TraderRiskGroup is not initialized");
            return Err(ProgramError::InvalidAccountData.into());
        }
        if index >= MAX_OUTRIGHTS {
            msg!(
                "Product index is out of bounds. index: {}, max products: {}",
                index,
                MAX_OUTRIGHTS
            );
            return Err(ProgramError::InvalidAccountData.into());
        }
        Ok(self.active_products[index] != u8::MAX)
    }

    pub fn clear(&mut self, product_key: Pubkey) -> DomainOrProgramResult {
        let trader_position_index = match self.find_position_index(&product_key) {
            Some(i) => i,
            None => {
                return Err(ProgramError::InvalidAccountData.into());
            }
        };
        let trader_position = &mut self.trader_positions[trader_position_index];
        self.active_products[trader_position.product_index] = u8::max_value();
        trader_position.tag = AccountTag::Uninitialized;
        trader_position.product_key = Pubkey::default();
        trader_position.position = ZERO_FRAC;
        trader_position.pending_position = ZERO_FRAC;
        trader_position.product_index = 0;
        trader_position.last_cum_funding_snapshot = ZERO_FRAC;
        trader_position.last_social_loss_snapshot = ZERO_FRAC;
        Ok(())
    }
}

#[zero_copy]
#[derive(Debug)]
pub struct TraderPosition {
    pub tag: AccountTag,
    pub product_key: Pubkey,
    pub position: Fractional,
    pub pending_position: Fractional,
    pub product_index: usize,
    pub last_cum_funding_snapshot: Fractional,
    pub last_social_loss_snapshot: Fractional,
}
impl IsInitialized for TraderPosition {
    fn is_initialized(&self) -> bool {
        self.tag == AccountTag::TraderPosition
    }
}
impl TraderPosition {
    pub fn is_active(&self) -> bool {
        self.position != ZERO_FRAC || self.pending_position != ZERO_FRAC
    }
}
