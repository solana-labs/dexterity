use ::std::cell::Ref;
use bonfida_utils::InstructionsAccount;

use anchor_lang::{
    prelude::*,
    solana_program::{
        msg,
        program::invoke_signed_unchecked,
        program_error::ProgramError,
        program_pack::IsInitialized,
        sysvar::{clock::Clock, Sysvar},
    },
};
use borsh::BorshSerialize;
use itertools::Itertools;

use agnostic_orderbook::{
    critbit::Slab,
    state::{read_register, MarketState, OrderSummary, Side},
};

use crate::{
    error::{DexError, DomainOrProgramResult, UtilError},
    state::risk_engine_register::*,
    utils::{
        cpi::risk_check,
        loadable::Loadable,
        numeric::{Fractional, ZERO_FRAC},
        orderbook::{get_bbo, update_prices},
        validation::{assert, assert_keys_equal},
    },
    CancelOrder, CancelOrderParams,
};

pub fn process_from_aob(base_size: u64, base_decimals: u64) -> Fractional {
    Fractional::new(base_size as i64, base_decimals)
}

fn validate(accts: &CancelOrder) -> DomainOrProgramResult {
    let trader_risk_group = accts.trader_risk_group.load()?;
    let market_product_group = accts.market_product_group.load_mut()?;
    assert_keys_equal(
        trader_risk_group.market_product_group,
        accts.market_product_group.key(),
    )?;
    assert(
        market_product_group.is_initialized(),
        UtilError::AccountUninitialized,
    )?;
    assert(
        trader_risk_group.is_initialized(),
        UtilError::AccountUninitialized,
    )?;
    assert_keys_equal(
        accts.risk_engine_program.key(),
        market_product_group.risk_engine_program_id,
    )?;
    // Check if risk register keys are equal
    assert_keys_equal(
        accts.trader_risk_state_acct.key(),
        trader_risk_group.risk_state_account,
    )?;

    assert_keys_equal(
        accts.risk_output_register.key(),
        market_product_group.risk_output_register,
    )?;

    assert_keys_equal(
        accts.risk_model_configuration_acct.key(),
        market_product_group.risk_model_configuration_acct,
    )?;

    Ok(())
}

pub fn process<'info>(
    ctx: Context<'_, '_, '_, 'info, CancelOrder<'info>>,
    params: CancelOrderParams,
) -> DomainOrProgramResult {
    let accts = ctx.accounts;
    validate(accts)?;

    let CancelOrderParams { order_id } = params;
    let mut trader_risk_group = accts.trader_risk_group.load_mut()?;
    let mut market_product_group = accts.market_product_group.load_mut()?;
    let voluntary_cancel = trader_risk_group.owner == *accts.user.key;
    if !voluntary_cancel {
        // Apply all unsettled funding prior to calling the risk engine
        trader_risk_group.apply_all_funding(&mut market_product_group)?;
        // If a user is canceling another user's orders:
        // Validate that the user whose orders are being canceled is a liquidation candidate
        let risk_engine_output = risk_check(
            &accts.risk_engine_program,
            &accts.market_product_group,
            &accts.trader_risk_group,
            &accts.risk_output_register,
            &accts.trader_risk_state_acct,
            &accts.risk_model_configuration_acct,
            &accts.risk_signer,
            ctx.remaining_accounts,
            &OrderInfo {
                operation_type: OperationType::CheckHealth,
                ..Default::default()
            },
            market_product_group.get_validate_account_health_discriminant(),
            market_product_group.risk_and_fee_bump as u8,
        )?;

        let health_info = match risk_engine_output {
            HealthResult::Health { health_info: v } => v,
            HealthResult::Liquidation {
                liquidation_info: _,
            } => return Err(DexError::InvalidAccountHealthError.into()),
        };

        // Only allow canceling if account is unhealthy or worse
        match health_info.health {
            HealthStatus::Healthy => {
                msg!("Account is healthy, orders can only be canceled by the user");
                return Err(DexError::InvalidAccountHealthError.into());
            }
            _ => {
                msg!("User's orders can be cancelled by any signer");
            }
        }
    }

    let windows = &market_product_group.ewma_windows.clone();
    let (product_index, _) = market_product_group.find_product_index(&accts.product.key())?;
    let product = &mut market_product_group.market_products[product_index];

    // Validation
    assert_keys_equal(product.orderbook, accts.orderbook.key())?;
    let cancel_order_instruction = agnostic_orderbook::instruction::cancel_order::Accounts {
        market: accts.orderbook.key,
        event_queue: accts.event_queue.key,
        bids: accts.bids.key,
        asks: accts.asks.key,
        authority: accts.market_signer.key,
    }
    .get_instruction(
        accts.aaob_program.key(),
        agnostic_orderbook::instruction::AgnosticOrderbookInstruction::CancelOrder as u8,
        agnostic_orderbook::instruction::cancel_order::Params { order_id },
    );
    // If the order was filled, the AOB will return and error.
    // TODO: Do we need to have special behavior to resolve this error.
    invoke_signed_unchecked(
        &cancel_order_instruction,
        &[
            accts.aaob_program.clone(),
            accts.orderbook.clone(),
            accts.market_signer.clone(),
            accts.event_queue.clone(),
            accts.bids.clone(),
            accts.asks.clone(),
        ],
        &[&[accts.product.key.as_ref(), &[product.bump as u8]]],
    )?;
    let orderbook = MarketState::get(accts.orderbook.as_ref())?;
    let bids = Slab::new_from_acc_info(&accts.bids, orderbook.callback_info_len as usize);
    let asks = Slab::new_from_acc_info(&accts.asks, orderbook.callback_info_len as usize);
    let best_bid = get_bbo(
        bids.find_max(),
        &bids,
        Side::Bid,
        product.tick_size,
        product.price_offset,
    )?;
    let best_ask = get_bbo(
        asks.find_min(),
        &asks,
        Side::Ask,
        product.tick_size,
        product.price_offset,
    )?;
    update_prices(
        &Clock::get()?,
        &mut product.prices,
        best_bid,
        best_ask,
        windows,
    )?;

    let order_summary: OrderSummary = read_register(accts.event_queue.as_ref())
        .map_err(ProgramError::from)?
        .unwrap();
    trader_risk_group.remove_open_order(product_index, order_id)?;
    let side = agnostic_orderbook::state::get_side_from_order_id(order_id);
    let order_qty = process_from_aob(order_summary.total_base_qty, product.base_decimals).abs();
    trader_risk_group.decrement_book_size(product_index, side, order_qty)?;
    market_product_group.sequence_number += 1;
    msg!("sequence: {}", market_product_group.sequence_number);
    accts.market_product_group.key().log();
    Ok(())
}
