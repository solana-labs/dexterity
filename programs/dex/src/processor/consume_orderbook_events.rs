use bonfida_utils::InstructionsAccount;
use std::{
    cell::{Ref, RefMut},
    ops::Deref,
    rc::Rc,
};

use agnostic_orderbook::{
    instruction::consume_events,
    state::{Event, EventQueue, EventQueueHeader, Side},
};
use anchor_lang::{
    prelude::*,
    solana_program::{
        account_info::AccountInfo, clock::Clock, entrypoint::ProgramResult, msg,
        program::invoke_signed_unchecked, program_error::ProgramError, program_pack::IsInitialized,
        pubkey::Pubkey, sysvar::Sysvar,
    },
};
use borsh::BorshDeserialize;

use crate::{
    error::{DexError, DomainOrProgramError, DomainOrProgramResult, UtilError},
    find_fees_ix,
    state::{
        callback_info::CallBackInfo,
        constants::CALLBACK_INFO_LEN,
        fee_model::{TraderFeeParams, TraderFees},
        market_product_group::MarketProductGroup,
        products::{Outright, Product},
        risk_engine_register::{OperationType, OrderInfo},
        trader_risk_group::TraderRiskGroup,
    },
    utils::{
        cpi::find_fees,
        loadable::Loadable,
        numeric::{Fractional, ZERO_FRAC},
        param::WithAcct,
        validation::{assert, assert_keys_equal},
    },
    validate_account_health_ix, ConsumeOrderbookEvents, ConsumeOrderbookEventsParams,
};

fn validate(accts: &ConsumeOrderbookEvents) -> DomainOrProgramResult {
    let market_product_group = accts.market_product_group.load()?;
    assert_keys_equal(
        *accts.fee_model_configuration_acct.key,
        market_product_group.fee_model_configuration_acct,
    )?;
    assert_keys_equal(
        *accts.fee_model_program.key,
        market_product_group.fee_model_program_id,
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
        accts.fee_output_register.key(),
        market_product_group.fee_output_register,
    )?;
    Ok(())
}

pub fn process<'a, 'b, 'c, 'info>(
    ctx: Context<'a, 'b, 'c, 'info, ConsumeOrderbookEvents<'info>>,
    params: ConsumeOrderbookEventsParams,
) -> DomainOrProgramResult {
    let accts = ctx.accounts;
    validate(accts)?;
    let mut market_product_group = WithAcct::new(
        accts.market_product_group.as_ref(),
        accts.market_product_group.load_mut()?,
    );

    let ConsumeOrderbookEventsParams { max_iterations } = params;

    let (product_index, product) = market_product_group.find_product_index(&accts.product.key())?;
    let product = *product;

    // Validation
    assert_keys_equal(product.orderbook, accts.orderbook.key())?;
    let is_expired = market_product_group.is_expired(&product);

    let event_queue_header =
        EventQueueHeader::deserialize(&mut (&accts.event_queue.data.borrow() as &[u8]))
            .map_err(ProgramError::from)?;
    let event_queue = EventQueue::new(
        event_queue_header,
        Rc::clone(&accts.event_queue.data),
        CALLBACK_INFO_LEN as usize,
    );

    let clock = &Clock::get()?;
    let mut total_iterations = 0;
    for event in event_queue.iter().take(max_iterations as usize) {
        let consume_event_result = consume_event(
            ctx.remaining_accounts,
            &mut market_product_group,
            product_index,
            event,
            &product,
            &accts.fee_model_configuration_acct,
            &accts.fee_output_register,
            &accts.fee_model_program,
            &accts.risk_and_fee_signer,
            is_expired,
            clock,
        );
        match consume_event_result {
            Ok(_) => total_iterations += 1,
            Err(DomainOrProgramError::DexErr(DexError::MissingUserAccount)) => {
                msg!("Missing required user account");
                break;
            }
            Err(e) => {
                msg!("Encountered unexpected error while consuming event");
                return Err(e);
            }
        }
    }

    if total_iterations == 0 {
        msg!("Failed to complete one iteration");
        return Err(DexError::NoOp.into());
    }

    let pop_events_instruction = agnostic_orderbook::instruction::consume_events::Accounts {
        market: accts.orderbook.key,
        event_queue: accts.event_queue.key,
        authority: accts.market_signer.key,
        reward_target: accts.reward_target.key,
    }
    .get_instruction(
        accts.aaob_program.key(),
        agnostic_orderbook::instruction::AgnosticOrderbookInstruction::ConsumeEvents as u8,
        agnostic_orderbook::instruction::consume_events::Params {
            number_of_entries_to_consume: total_iterations,
        },
    );

    invoke_signed_unchecked(
        &pop_events_instruction,
        &[
            accts.aaob_program.clone(),
            accts.orderbook.clone(),
            accts.event_queue.clone(),
            accts.market_signer.clone(),
            accts.reward_target.clone(),
        ],
        &[&[accts.product.key.as_ref(), &[product.bump as u8]]],
    )?;
    market_product_group.sequence_number += 1;
    msg!("sequence: {}", market_product_group.sequence_number);
    accts.market_product_group.key().log();
    Ok(())
}

pub fn process_fill_from_event_queue(
    quote_size_eq: u64,
    base_size_eq: u64,
    price_offset: Fractional,
    tick_size: Fractional,
    base_decimals: u64,
) -> std::result::Result<[Fractional; 2], ProgramError> {
    let base_qty_dex = Fractional::new(base_size_eq as i64, base_decimals);
    let offset_quote_qty_dex = Fractional::new(quote_size_eq as i64, base_decimals);
    let quote_qty_dex = offset_quote_qty_dex
        .checked_mul(tick_size)?
        .checked_sub(price_offset.checked_mul(base_qty_dex)?)?;
    Ok([base_qty_dex, quote_qty_dex])
}

pub fn process_out_from_event_queue(base_size_eq: u64, base_decimals: u64) -> Fractional {
    Fractional::new(base_size_eq as i64, base_decimals)
}

fn consume_event<'c, 'info>(
    accounts: &'c [AccountInfo<'info>],
    market_product_group: &mut WithAcct<'_, 'info, RefMut<'_, MarketProductGroup>>,
    product_index: usize,
    event: Event,
    product: &Product,
    fee_model_configuration: &AccountInfo<'info>,
    fee_output_register: &AccountInfo<'info>,
    fee_model_program: &AccountInfo<'info>,
    fee_and_risk_signer: &AccountInfo<'info>,
    is_expired: bool,
    clock: &Clock,
) -> DomainOrProgramResult {
    match event {
        Event::Fill {
            taker_side,
            maker_order_id: _,
            quote_size,
            base_size,
            maker_callback_info,
            taker_callback_info,
        } => {
            let (maker_loader, maker_fees, mut taker) =
                find_participants(&maker_callback_info, &taker_callback_info, accounts)?;
            let mut maker = MakerInfo {
                risk_group: maker_loader,
                fee_state: maker_fees,
            };
            let self_trade = maker.risk_group.key() == taker.key();
            let [total_base_qty_dex, total_quote_qty_dex] = process_fill_from_event_queue(
                quote_size,
                base_size,
                product.price_offset,
                product.tick_size,
                product.base_decimals,
            )?;
            {
                let mut maker_risk_group = maker.risk_group.load_mut()?;
                if maker_risk_group.valid_until <= clock.unix_timestamp {
                    let fee_params = TraderFeeParams {
                        side: taker_side.opposite(),
                        is_aggressor: false,
                        matched_quote_qty: total_quote_qty_dex,
                        matched_base_qty: total_base_qty_dex,
                        product: product.product_key,
                    };
                    find_fees(
                        &fee_model_program,
                        market_product_group.acct,
                        &maker.risk_group,
                        &maker.fee_state,
                        &fee_model_configuration,
                        &fee_output_register,
                        &fee_and_risk_signer,
                        market_product_group.get_find_fees_discriminant(),
                        &fee_params,
                        market_product_group.risk_and_fee_bump as u8,
                    )?;
                }
                let computed_fees = TraderFees::load(fee_output_register)?;
                maker_risk_group.valid_until = computed_fees.valid_until;
                maker_risk_group.maker_fee_bps = computed_fees.maker_fee_bps;
                maker_risk_group.taker_fee_bps = computed_fees.taker_fee_bps;
            }

            update_cash_balance(
                market_product_group,
                &mut maker,
                &mut taker,
                taker_side,
                total_quote_qty_dex,
                self_trade,
            )?;
            if !is_expired {
                maker.risk_group.load_mut()?.decrement_book_size(
                    product_index,
                    taker_side.opposite(),
                    total_base_qty_dex.abs(),
                )?;
                for (ratio, i) in product.get_ratios_and_product_indices(product_index) {
                    let (taker_index, maker_index) = {
                        assert(
                            taker.load()?.is_initialized(),
                            UtilError::AccountUninitialized,
                        )?;
                        assert(
                            maker.risk_group.load()?.is_initialized(),
                            UtilError::AccountUninitialized,
                        )?;
                        let taker_index = taker.load()?.active_products[i] as usize;
                        let maker_index = maker.risk_group.load()?.active_products[i] as usize;
                        maker
                            .risk_group
                            .load_mut()?
                            .apply_funding(market_product_group, maker_index)?;
                        taker
                            .load_mut()?
                            .apply_funding(market_product_group, taker_index)?;
                        (taker_index, maker_index)
                    };
                    if !self_trade {
                        update_positions_no_self_trade(
                            maker.risk_group.load_mut()?,
                            taker.load_mut()?,
                            maker_index,
                            taker_index,
                            taker_side,
                            market_product_group.market_products[i].try_to_outright_mut()?,
                            total_base_qty_dex,
                            Fractional::from(ratio),
                        )?;
                    }
                    let signed_ratio = match taker_side {
                        Side::Bid => -ratio,
                        Side::Ask => ratio,
                    };
                    let taker_pos = &mut taker.load_mut()?.trader_positions[taker_index];
                    taker_pos.pending_position = taker_pos
                        .pending_position
                        .checked_add(total_base_qty_dex.checked_mul(signed_ratio)?)?;
                }
            }
        }
        Event::Out {
            side,
            order_id,
            base_size,
            callback_info,
            delete,
        } => {
            if (!delete && base_size == 0) || is_expired {
                // PASS
            } else {
                let user_callback_info = &CallBackInfo::try_from_slice(&callback_info[..])
                    .map_err(|_| UtilError::DeserializeError)?;
                let user_account_info = find_acct(accounts, &user_callback_info.user_account)?;
                let order_index = user_callback_info.open_orders_idx as usize;

                let trader_risk_group_loader =
                    AccountLoader::<TraderRiskGroup>::try_from(user_account_info)?;
                let mut trader_risk_group = trader_risk_group_loader.load_mut()?;
                let total_base_qty_dex =
                    process_out_from_event_queue(base_size, product.base_decimals);
                if base_size != 0 {
                    trader_risk_group.decrement_book_size(
                        product_index,
                        side,
                        total_base_qty_dex,
                    )?;
                }

                if delete {
                    trader_risk_group.open_orders.remove_open_order_by_index(
                        product_index,
                        order_index,
                        order_id,
                    )?;
                }
            }
        }
    };
    for (_, i) in product.get_ratios_and_product_indices(product_index) {
        let mut outright = market_product_group.market_products[i].try_to_outright_mut()?;
        outright.num_queue_events = outright.num_queue_events.saturating_sub(1);
    }
    Ok(())
}

fn update_positions_no_self_trade<'info>(
    mut maker: RefMut<TraderRiskGroup>,
    mut taker: RefMut<TraderRiskGroup>,
    maker_index: usize,
    taker_index: usize,
    taker_side: Side,
    market_product: &mut Outright,
    base_size: Fractional,
    ratio: Fractional,
) -> ProgramResult {
    let maker_pos = &mut maker.trader_positions[maker_index];
    let taker_pos = &mut taker.trader_positions[taker_index];
    let (buyer, seller) = {
        match taker_side {
            Side::Bid => (taker_pos, maker_pos),
            Side::Ask => (maker_pos, taker_pos),
        }
    };
    if ratio > ZERO_FRAC {
        market_product.update_open_interest_change(
            base_size * ratio,
            buyer.position.min(ZERO_FRAC).abs(),
            seller.position.max(ZERO_FRAC),
        )?;
    } else {
        market_product.update_open_interest_change(
            (base_size * ratio).abs(),
            seller.position.min(ZERO_FRAC).abs(),
            buyer.position.max(ZERO_FRAC),
        )?;
    }
    seller.position = seller.position.checked_sub(base_size.checked_mul(ratio)?)?;
    buyer.position = buyer.position.checked_add(base_size.checked_mul(ratio)?)?;
    Ok(())
}

/// update_cash_balance:
/// 1. moves cash between maker and taker
/// 2. pays maker fees calculated in this Ix
/// 3. settles previously calculated taker fees
fn update_cash_balance(
    market_product_group: &mut MarketProductGroup,
    maker: &mut MakerInfo,
    taker: &mut AccountLoader<TraderRiskGroup>,
    taker_side: Side,
    quote_size: Fractional,
    self_trade: bool,
) -> DomainOrProgramResult {
    if !self_trade {
        // safe to borrow both maker and taker at once since they will not be aliased
        let (mut buyer, mut seller) = match taker_side {
            Side::Bid => (taker.load_mut()?, maker.risk_group.load_mut()?),
            Side::Ask => (maker.risk_group.load_mut()?, taker.load_mut()?),
        };
        seller.cash_balance = seller.cash_balance.checked_add(quote_size)?;
        buyer.cash_balance = buyer.cash_balance.checked_sub(quote_size)?;
    }
    // mutate taker
    let taker_pending_fees = {
        let mut taker = taker.load_mut()?;
        taker.pending_cash_balance = taker.pending_cash_balance.checked_add(match taker_side {
            Side::Bid => quote_size,
            Side::Ask => -quote_size,
        })?;
        taker.cash_balance = taker.cash_balance.checked_sub(taker.pending_fees)?;
        taker.pending_fees
    };
    // mutate maker
    {
        let mut maker_risk_group = maker.risk_group.load_mut()?;
        let maker_fee = TraderFees::new(
            maker_risk_group.maker_fee_bps,
            maker_risk_group.taker_fee_bps,
            maker_risk_group.valid_until,
        )
        .maker_fee_bps(Some(market_product_group))
        .checked_mul(quote_size)?;
        maker_risk_group.cash_balance = maker_risk_group.cash_balance.checked_sub(maker_fee)?;

        market_product_group.collected_fees = market_product_group
            .collected_fees
            .checked_add(maker_fee)?
            .checked_add(taker_pending_fees)?;
    }
    // mutate taker
    {
        let mut taker = taker.load_mut()?;
        taker.pending_fees = ZERO_FRAC;
    }
    Ok(())
}

struct MakerInfo<'c, 'info> {
    risk_group: AccountLoader<'info, TraderRiskGroup>,
    fee_state: &'c AccountInfo<'info>,
}

fn find_participants<'c, 'info>(
    maker_callback_info: &Vec<u8>,
    taker_callback_info: &Vec<u8>,
    accounts: &'c [AccountInfo<'info>],
) -> std::result::Result<
    (
        AccountLoader<'info, TraderRiskGroup>,
        &'c AccountInfo<'info>,
        AccountLoader<'info, TraderRiskGroup>,
    ),
    DomainOrProgramError,
> {
    let maker_key = &CallBackInfo::try_from_slice(&maker_callback_info[..])
        .map_err(|_| UtilError::DeserializeError)?
        .user_account;
    let taker_key = &CallBackInfo::try_from_slice(&taker_callback_info[..])
        .map_err(|_| UtilError::DeserializeError)?
        .user_account;
    let maker_risk_group_acct = find_acct(accounts, maker_key)?;
    let maker_risk_group_loader = AccountLoader::try_from(maker_risk_group_acct)?;
    let taker_risk_group_acct = find_acct(accounts, taker_key)?;
    let taker_risk_group_loader = AccountLoader::try_from(taker_risk_group_acct)?;
    let maker_fee_acct_key = {
        // Must not mutably load anything since the binary search will borrow from the accounts slice,
        // which can cause a borrow error since a mutable and immutable borrow cannot co-occur.
        let maker_risk_group: Ref<TraderRiskGroup> = maker_risk_group_loader.load()?;
        maker_risk_group.fee_state_account
    };
    let maker_fee_acct = find_acct(accounts, &maker_fee_acct_key)?;
    Ok((
        maker_risk_group_loader,
        maker_fee_acct,
        taker_risk_group_loader,
    ))
}

fn find_acct<'c, 'info>(
    accounts: &'c [AccountInfo<'info>],
    key: &Pubkey,
) -> std::result::Result<&'c AccountInfo<'info>, DexError> {
    let idx = accounts.binary_search_by_key(key, |a| *a.key);
    match idx {
        Ok(idx) => Ok(&accounts[idx]),
        Err(_) => {
            use itertools::Itertools;
            let is_sorted = accounts.iter().tuple_windows().all(|(a, b)| a.key <= b.key);
            if !is_sorted {
                msg!("Trader and fee accounts must be sorted by client")
            } else {
                msg!(
                    "Could not find {:?} in {:?}",
                    key,
                    accounts.iter().map(|a| a.key).collect::<Vec<_>>()
                );
            }
            Err(DexError::MissingUserAccount)
        }
    }
}
