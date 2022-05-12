#![allow(non_snake_case)]
use agnostic_orderbook::state::Side;
use anchor_lang::solana_program::pubkey::Pubkey;
use dex::{
    state::constants::*,
    utils::numeric::{Fractional, ZERO_FRAC},
};
use dexteritysdk::{common::utils::*, processor::orderbook::*, state::Order};
use rand::prelude::SliceRandom;
use solana_sdk::signature::{Keypair, Signer};

mod setup;
use crate::setup::bootstrap_tests;

pub const WINDOW_SIZE: usize = 2;

#[tokio::test]
async fn test_orderbook_layering() {
    log_disable();
    let (ctx, traders) =
        &mut bootstrap_tests("noop_risk_engine", "constant_fees", "test", 1, 1).await;
    let product_0 = ctx.products[0].clone();
    let trader_0 = traders[0].clone();
    let mut bid_amount = 0;
    let mut count = 0;
    let window_size = WINDOW_SIZE;
    let mut rng = rand::thread_rng();
    let mut bid_prices = (0..50).collect::<Vec<i64>>();
    let best_bid = 1000;
    bid_prices.shuffle(&mut rng);
    for bids in bid_prices.chunks(window_size) {
        let len = bids.len();
        trader_0
            .place_orders(
                ctx,
                &product_0,
                bids.iter()
                    .map(|b| {
                        Order::new(
                            Side::Bid.into(),
                            Fractional::new(1000, 1),
                            Fractional::new(best_bid - *b, 0)
                                .round_unchecked(4)
                                .unwrap(),
                        )
                    })
                    .collect(),
            )
            .await
            .unwrap();
        count += len as u64;
        bid_amount += len * 100;
    }
    let mut ask_amount = 0;
    let mut rng = rand::thread_rng();
    let mut ask_prices = (0..50).collect::<Vec<i64>>();
    let best_ask = 1001;
    ask_prices.shuffle(&mut rng);
    for asks in ask_prices.chunks(window_size) {
        let len = asks.len();
        let res = trader_0
            .place_orders(
                ctx,
                &product_0,
                asks.iter()
                    .map(|a| {
                        Order::new(
                            Side::Ask.into(),
                            Fractional::new(1000, 1),
                            Fractional::new(best_ask + *a, 0)
                                .round_unchecked(4)
                                .unwrap(),
                        )
                    })
                    .collect(),
            )
            .await;
        if res.is_err() {
            assert_eq!(count, MAX_OPEN_ORDERS_PER_POSITION);
            break;
        }
        count += len as u64;
        ask_amount += len * 100;
    }
    let trader_risk_group_0 = trader_0.get_trader_risk_group(&ctx.client).await;
    let position = trader_risk_group_0.trader_positions[0];
    assert_eq!(
        trader_risk_group_0.open_orders.products[0].bid_qty_in_book,
        Fractional::new(bid_amount as i64, 0)
    );
    assert_eq!(
        trader_risk_group_0.open_orders.products[0].ask_qty_in_book,
        Fractional::new(ask_amount as i64, 0)
    );
    assert_eq!(position.position, ZERO_FRAC);
    assert_eq!(position.pending_position, ZERO_FRAC);
    assert_eq!(
        trader_risk_group_0.open_orders.products[0].num_open_orders,
        count
    );
    let mut order_ids: Vec<u128> = vec![];
    let mut ptr = trader_risk_group_0.open_orders.products[0].head_index as usize;
    let order = trader_risk_group_0.open_orders.orders[ptr];
    assert_eq!(order.prev, SENTINEL);
    let mut order_count = 0;
    while ptr != SENTINEL {
        let order = trader_risk_group_0.open_orders.orders[ptr];
        assert_ne!(order.id, 0);
        order_ids.push(order.id);
        ptr = order.next;
        order_count += 1;
    }
    assert_eq!(order_count, count);
    let mut rng = rand::thread_rng();
    order_ids.shuffle(&mut rng);
    let window_size = WINDOW_SIZE;
    for (_i, o_ids) in order_ids.chunks(window_size).enumerate() {
        let _res = trader_0
            .cancel_orders(ctx, &product_0, o_ids.iter().copied().collect())
            .await;
        // res.map_err(|e| dbg!(e)).unwrap();
        count -= o_ids.len() as u64;
    }
    let trader_risk_group_0 = trader_0.get_trader_risk_group(&ctx.client).await;
    assert_eq!(count, 0);
    let _position = trader_risk_group_0.trader_positions[0];
    assert!(trader_risk_group_0.open_orders.products[0].bid_qty_in_book == ZERO_FRAC);
    assert!(trader_risk_group_0.open_orders.products[0].ask_qty_in_book == ZERO_FRAC);
}

#[tokio::test]
async fn test_fill_up_market_product_group() {
    let (ctx, _traders) =
        &mut bootstrap_tests("noop_risk_engine", "constant_fees", "test", 1, 128).await;
    let product = Keypair::new();
    let (market_signer, _) =
        Pubkey::find_program_address(&[product.pubkey().as_ref()], &ctx.dex_program_id);
    let (orderbook_key, _bids_key, _asks_key, _eq_key) =
        create_orderbook(&ctx.client, ctx.aaob_program_id, market_signer)
            .await
            .unwrap();
    let name_str = format!("product{:width$}", "N", width = NAME_LEN - 7);
    let mut name = [0; NAME_LEN];
    name.clone_from_slice(name_str.as_bytes());
    let res = ctx
        .initialize_market_product(
            product.pubkey(),
            orderbook_key,
            name,
            Fractional::new(100, 4),
            4,
            0,
        )
        .await;
    assert!(res.is_err());
}
