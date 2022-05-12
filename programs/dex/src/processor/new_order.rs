use std::{
    borrow::BorrowMut,
    cell::{Ref, RefMut},
};

use anchor_lang::{
    prelude::*,
    solana_program::{
        msg,
        program::{invoke_signed_unchecked, invoke_unchecked},
        program_error::ProgramError,
        program_pack::IsInitialized,
        pubkey::Pubkey,
        system_instruction,
        sysvar::{clock::Clock, Sysvar},
    },
};
use bonfida_utils::InstructionsAccount;
use borsh::BorshSerialize;

use agnostic_orderbook::{
    critbit::Slab,
    state::{read_register, EventQueueHeader, MarketState, OrderSummary, SelfTradeBehavior, Side},
};

use crate::{
    error::{DexError, DomainOrProgramResult, UtilError},
    find_fees_ix,
    state::{
        callback_info::CallBackInfo,
        enums::OrderType,
        fee_model::{TraderFeeParams, TraderFees},
        products::Product,
        risk_engine_register::*,
    },
    utils::{
        cpi::{find_fees, risk_check},
        loadable::Loadable,
        logs::DexOrderSummary,
        numeric::{fp32_mul, u64_to_quote, Fractional, ZERO_FRAC},
        orderbook::{get_bbo, update_prices},
        param::WithAcct,
        validation::{assert, assert_keys_equal},
    },
    DomainOrProgramError, MarketProductGroup, NewOrder, NewOrderParams, TraderRiskGroup,
};

fn validate(ctx: &Context<NewOrder>) -> std::result::Result<(), DomainOrProgramError> {
    let accts = &ctx.accounts;
    let trader_risk_group = accts.trader_risk_group.load()?;
    let market_product_group = accts.market_product_group.load()?;

    assert_keys_equal(trader_risk_group.owner, *accts.user.key)?;
    assert_keys_equal(
        trader_risk_group.market_product_group,
        accts.market_product_group.key(),
    )?;
    assert(
        trader_risk_group.is_initialized(),
        UtilError::AccountUninitialized,
    )?;
    assert(
        market_product_group.is_initialized(),
        UtilError::AccountUninitialized,
    )?;
    assert_keys_equal(
        accts.fee_model_program.key(),
        market_product_group.fee_model_program_id,
    )?;
    assert_keys_equal(
        accts.fee_model_configuration_acct.key(),
        market_product_group.fee_model_configuration_acct,
    )?;

    assert_keys_equal(
        accts.trader_risk_state_acct.key(),
        trader_risk_group.risk_state_account,
    )?;

    assert_keys_equal(
        accts.trader_fee_state_acct.key(),
        trader_risk_group.fee_state_account,
    )?;

    assert_keys_equal(
        accts.risk_output_register.key(),
        market_product_group.risk_output_register,
    )?;

    assert_keys_equal(
        accts.fee_output_register.key(),
        market_product_group.fee_output_register,
    )?;

    assert_keys_equal(
        accts.risk_model_configuration_acct.key(),
        market_product_group.risk_model_configuration_acct,
    )?;
    assert(accts.orderbook.is_writable, DexError::CombosNotRemoved)?;
    Ok(())
}

pub fn process<'info>(
    ctx: Context<'_, '_, '_, 'info, NewOrder<'info>>,
    params: NewOrderParams,
) -> DomainOrProgramResult {
    validate(&ctx)?;
    let accts = ctx.accounts;

    let mut trader_risk_group = accts.trader_risk_group.load_mut()?;
    let mut market_product_group = accts.market_product_group.load_mut()?;

    let NewOrderParams {
        side,
        max_base_qty,
        order_type,
        self_trade_behavior,
        match_limit,
        limit_price,
    } = params;
    let orderbook = MarketState::get(&accts.orderbook)?;
    if max_base_qty < u64_to_quote(orderbook.min_base_order_size as u64)? {
        msg!("The base order size is too small.");
        return Err(ProgramError::InvalidArgument.into());
    }
    let (product_index, _) = market_product_group.find_product_index(&accts.product.key())?;
    let product = market_product_group.market_products[product_index];

    // Product validation
    assert(
        !market_product_group.is_expired(&product),
        DexError::ContractIsExpired,
    )?;
    assert_keys_equal(product.orderbook, accts.orderbook.key())?;

    let (post_only, post_allowed) = match order_type {
        OrderType::Limit => (false, true),
        OrderType::ImmediateOrCancel | OrderType::FillOrKill => (false, false),
        OrderType::PostOnly => (true, true),
    };

    let callback_info = CallBackInfo {
        user_account: accts.trader_risk_group.key(),
        open_orders_idx: trader_risk_group.open_orders.get_next_index() as u64,
    };
    assert(accts.orderbook.is_writable, DexError::CombosNotRemoved)?;
    invoke_unchecked(
        &system_instruction::transfer(
            accts.user.key,
            accts.orderbook.key,
            orderbook.cranker_reward,
        ),
        &[
            accts.user.clone(),
            accts.orderbook.clone(),
            accts.system_program.to_account_info(),
        ],
    )?;
    let limit_price_aob =
        get_limit_price_aob(limit_price, product.price_offset, product.tick_size)?;

    let starting_queue_size =
        EventQueueHeader::deserialize(&mut (&accts.event_queue.data.borrow() as &[u8]))
            .map_err(ProgramError::from)?
            .count;

    invoke_signed_unchecked(
        &agnostic_orderbook::instruction::new_order::Accounts {
            market: accts.orderbook.key,
            event_queue: accts.event_queue.key,
            bids: accts.bids.key,
            asks: accts.asks.key,
            authority: accts.market_signer.key,
        }
        .get_instruction(
            accts.aaob_program.key(),
            agnostic_orderbook::instruction::AgnosticOrderbookInstruction::NewOrder as u8,
            agnostic_orderbook::instruction::new_order::Params {
                max_base_qty: max_base_qty.round(product.base_decimals as u32)?.m as u64,
                max_quote_qty: u64::MAX,
                limit_price: limit_price_aob,
                side,
                match_limit,
                callback_info: callback_info.to_vec(),
                post_only,
                post_allowed,
                self_trade_behavior,
            },
        ),
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

    let ending_queue_size =
        EventQueueHeader::deserialize(&mut (&accts.event_queue.data.borrow() as &[u8]))
            .map_err(ProgramError::from)?
            .count;

    let new_events = ending_queue_size.saturating_sub(starting_queue_size);

    update_new_queue_events(
        &product,
        product_index,
        &mut market_product_group,
        new_events,
    )?;

    let OrderSummary {
        posted_order_id,
        total_base_qty,
        total_quote_qty,
        total_base_qty_posted,
    }: OrderSummary = read_register(&accts.event_queue).unwrap().unwrap();

    emit!(DexOrderSummary::new(
        posted_order_id,
        total_base_qty,
        total_quote_qty,
        total_base_qty_posted,
    ));

    {
        let bids = Slab::new_from_acc_info(&accts.bids, orderbook.callback_info_len as usize);
        let asks = Slab::new_from_acc_info(&accts.asks, orderbook.callback_info_len as usize);
        let windows = &market_product_group.ewma_windows.clone();
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
            &mut market_product_group.market_products[product_index].prices,
            best_bid,
            best_ask,
            windows,
        )?;
    }

    let [total_base_qty_dex, matched_base_qty_dex, matched_quote_qty_dex] = process_from_aob(
        total_base_qty,
        total_base_qty_posted,
        total_quote_qty,
        limit_price_aob,
        product.price_offset,
        product.tick_size,
        product.base_decimals,
    )?;
    let is_combo = product.is_combo();
    //// For the snapshot to be sent to risk engine
    let (old_ask_qty_in_book, old_bid_qty_in_book) = (
        trader_risk_group.open_orders.products[product_index].ask_qty_in_book,
        trader_risk_group.open_orders.products[product_index].bid_qty_in_book,
    );

    trader_risk_group.adjust_book_qty(
        product_index,
        total_base_qty_dex.checked_sub(matched_base_qty_dex)?,
        side,
    )?;

    let crossed = matched_quote_qty_dex != ZERO_FRAC;
    update_metadata(
        &product,
        &mut trader_risk_group,
        &mut market_product_group,
        product_index,
        matched_base_qty_dex,
        side,
        crossed,
    )?;

    if crossed || trader_risk_group.valid_until == 0 {
        // Make call into the risk engine if there's a cross or if the trader's fees are uninitialized
        handle_fees(
            accts,
            &Clock::get()?,
            &market_product_group,
            &mut trader_risk_group,
            if crossed {
                matched_quote_qty_dex
            } else {
                ZERO_FRAC
            },
            matched_base_qty_dex,
            accts.product.key(),
            side,
        )?;
    }
    if crossed {
        match side {
            Side::Bid => {
                trader_risk_group.pending_cash_balance = trader_risk_group
                    .pending_cash_balance
                    .checked_sub(matched_quote_qty_dex)?
                    .round(market_product_group.decimals as u32)?;
            }

            Side::Ask => {
                trader_risk_group.pending_cash_balance = trader_risk_group
                    .pending_cash_balance
                    .checked_add(matched_quote_qty_dex)?
                    .round(market_product_group.decimals as u32)?;
            }
        }
    }
    match posted_order_id {
        Some(order_id) => trader_risk_group.add_open_order(product_index, order_id)?,
        None => {}
    }

    // Apply all unsettled funding prior to calling the risk engine
    trader_risk_group.apply_all_funding(&mut market_product_group)?;

    match risk_check(
        &accts.risk_engine_program,
        &accts.market_product_group,
        &accts.trader_risk_group,
        &accts.risk_output_register,
        &accts.trader_risk_state_acct,
        &accts.risk_model_configuration_acct,
        &accts.risk_and_fee_signer,
        ctx.remaining_accounts,
        &OrderInfo {
            total_order_qty: total_base_qty_dex,
            matched_order_qty: matched_base_qty_dex,
            old_ask_qty_in_book,
            old_bid_qty_in_book,
            order_side: side,
            is_combo,
            product_index,
            operation_type: OperationType::NewOrder,
        },
        market_product_group.get_validate_account_health_discriminant(),
        market_product_group.risk_and_fee_bump as u8,
    )? {
        HealthResult::Health { health_info } => {
            if health_info.action != ActionStatus::Approved {
                msg!("health_info.action: {:?}", health_info.action);
                return Err(DexError::InvalidAccountHealthError.into());
            }
        }
        HealthResult::Liquidation {
            liquidation_info: _,
        } => return Err(DexError::InvalidAccountHealthError.into()),
    }

    market_product_group.sequence_number += 1;
    msg!("sequence: {}", market_product_group.sequence_number);
    accts.market_product_group.key().log();
    Ok(())
}

fn handle_fees(
    accts: &NewOrder,
    clock: &Clock,
    market_product_group: &MarketProductGroup,
    trader_risk_group: &mut TraderRiskGroup,
    matched_quote_qty: Fractional,
    matched_base_qty: Fractional,
    product: Pubkey,
    side: Side,
) -> DomainOrProgramResult {
    if trader_risk_group.valid_until <= clock.unix_timestamp {
        let fee_params = TraderFeeParams {
            side,
            is_aggressor: true,
            matched_base_qty,
            matched_quote_qty,
            product,
        };
        find_fees(
            &accts.fee_model_program,
            accts.market_product_group.as_ref(),
            &accts.trader_risk_group,
            &accts.trader_fee_state_acct,
            &accts.fee_model_configuration_acct,
            &accts.fee_output_register,
            &accts.risk_and_fee_signer,
            market_product_group.get_find_fees_discriminant(),
            &fee_params,
            market_product_group.risk_and_fee_bump as u8,
        )?;
    }

    let computed_fees = TraderFees::load(&accts.fee_output_register)?;
    let taker_fees = computed_fees
        .taker_fee_bps(Some(market_product_group))
        .checked_mul(matched_quote_qty)?;

    trader_risk_group.pending_fees = trader_risk_group.pending_fees.checked_add(taker_fees)?;
    trader_risk_group.valid_until = computed_fees.valid_until;
    trader_risk_group.maker_fee_bps = computed_fees.maker_fee_bps;
    trader_risk_group.taker_fee_bps = computed_fees.taker_fee_bps;
    Ok(())
}

fn update_new_queue_events(
    product: &Product,
    product_index: usize,
    market_product_group: &mut MarketProductGroup,
    new_events: u64,
) -> DomainOrProgramResult {
    for (_, i) in product.get_ratios_and_product_indices(product_index) {
        let outright = market_product_group.market_products[i].try_to_outright_mut()?;
        outright.num_queue_events = outright
            .num_queue_events
            .saturating_add(new_events as usize);
    }
    Ok(())
}

fn update_metadata(
    product: &Product,
    trader_risk_group: &mut TraderRiskGroup,
    market_product_group: &MarketProductGroup,
    product_index: usize,
    matched_base_qty_dex: Fractional,
    side: Side,
    crossed: bool,
) -> DomainOrProgramResult {
    for (ratio, i) in product.get_ratios_and_product_indices(product_index) {
        let outright = market_product_group.market_products[i].try_to_outright()?;
        trader_risk_group.activate_if_uninitialized(
            i,
            &outright.product_key,
            outright.cum_funding_per_share,
            outright.cum_social_loss_per_share,
            market_product_group.active_combos(),
        )?;
        if crossed {
            let trader_position_index = trader_risk_group.active_products[i] as usize;
            let trader_position = &mut trader_risk_group.trader_positions[trader_position_index];
            match side {
                Side::Bid => {
                    trader_position.pending_position = trader_position
                        .pending_position
                        .checked_add(matched_base_qty_dex.checked_mul(Fractional::from(ratio))?)?
                }
                Side::Ask => {
                    trader_position.pending_position = trader_position
                        .pending_position
                        .checked_sub(matched_base_qty_dex.checked_mul(Fractional::from(ratio))?)?
                }
            }
        }
    }
    Ok(())
}

#[inline(always)]
pub fn get_limit_price_aob(
    price: Fractional,
    price_offset: Fractional,
    tick_size: Fractional,
) -> std::result::Result<u64, ProgramError> {
    /*
        Adjusts the passed-in limit price by adding a positive offset, dividing by the market tick
        size and coercing the output to a u64.
        This creates a remapping of the bytes such that the following property holds

        (-price_offset) / tick_size maps to 0x00000000
        (2^32 - 1 - price_offset) / tick_size maps to 0xFFFFFFFF

        Lexigraphical byte ordering of the integers (sorting by bytes) and numerical ordering
        are both preserved in this representation.
    */
    let price_ticks_raw = price.checked_add(price_offset)?.checked_div(tick_size)?;
    let price_ticks = price_ticks_raw.round_sf(0);
    if price_ticks != price_ticks_raw {
        msg!(
            "Not exact tick, converting to nearest tick {} -> {}",
            price_ticks_raw,
            price_ticks,
        );
    }
    // AOB price needs to be shifted up by 32 bits to create a fixec point representation
    let limit_price = price_ticks.m << 32;
    Ok(limit_price as u64)
}

#[inline(always)]
pub fn process_from_aob(
    total_base_qty_aob: u64,
    total_base_qty_posted_aob: u64,
    total_quote_qty_aob: u64,
    limit_price_aob: u64,
    price_offset: Fractional,
    tick_size: Fractional,
    base_decimal: u64,
) -> std::result::Result<[Fractional; 3], ProgramError> {
    /*
        When processing trades from the order book, the matched quantity (in cash)
        is computed as sum((fill_price_i + price_offset) * base_size_i).
        Our desired target is sum(fill_price_i * base_size_i), so we subtract out
        price_offset * sum(base_size_i). The AAOB returns total_base_qty_posted - total_base_qty
        as the matched_quantity = sum(base_size_i). So we perform the transformation and
        convert the ticks back into prices.
        Naming convention suffixes:
        - AOB price space variables: _aob
        - DEX price space variables: _dex
    */
    // Compute number of matched base fills (AOB-base space)
    let matched_base_qty_aob = (total_base_qty_aob - total_base_qty_posted_aob) as i64;
    let total_base_dex = Fractional::new(total_base_qty_aob as i64, base_decimal);
    let total_base_matched_dex = Fractional::new(matched_base_qty_aob, base_decimal);
    // Compute number of matched quote fills (AOB-quote space)
    let total_quote_qty_posted_aob = fp32_mul(total_base_qty_posted_aob, limit_price_aob);
    let matched_quote_qty_aob = total_quote_qty_aob - total_quote_qty_posted_aob;
    // Undo tick size division (AOB -> DEX)
    let match_quote_qty_with_offset_dex =
        Fractional::new(matched_quote_qty_aob as i64, base_decimal).checked_mul(tick_size)?;
    let quote_offset_dex = total_base_matched_dex.checked_mul(price_offset)?;
    // Adjust DEX offset
    let matched_quote_qty_dex = match_quote_qty_with_offset_dex.checked_sub(quote_offset_dex)?;
    Ok([
        total_base_dex,
        total_base_matched_dex,
        matched_quote_qty_dex,
    ])
}
