#![allow(non_snake_case)]

use agnostic_orderbook::state::Side;
use anchor_lang::Key;
use dex::{
    state::{constants::*, enums::*},
    utils::numeric::{bps, Fractional},
};
use dexteritysdk::common::utils::*;

use crate::setup::bootstrap_tests;

mod setup;

#[tokio::test]
async fn test_simple_trade() {
    let (ctx, traders) =
        &mut bootstrap_tests("noop_risk_engine", "constant_fees", "test", 3, 2).await;

    // set non-zero fees
    let taker_fee = bps(200);
    let maker_fee = bps(10);
    ctx.update_fees(10, 200).await.unwrap();

    let product_0 = ctx.products[0].clone();
    let trader_0 = traders[0].clone();
    trader_0
        .place_order(ctx, &product_0, Side::Bid, 100, 100)
        .await
        .unwrap();

    let trader_risk_group_0 = trader_0.get_trader_risk_group(&ctx.client).await;

    assert!(trader_risk_group_0.is_active_product(0).unwrap());

    let tpi = trader_risk_group_0.active_products[0] as usize;
    let trader_position_00 = trader_risk_group_0.trader_positions[tpi];

    assert_eq!(trader_position_00.tag, AccountTag::TraderPosition);
    assert_eq!(trader_position_00.product_key, product_0.key());
    assert_eq!(trader_position_00.product_index, 0);
    assert_eq_frac(
        trader_risk_group_0.open_orders.products[0].ask_qty_in_book,
        0,
    );
    assert_eq_frac(
        trader_risk_group_0.open_orders.products[0].bid_qty_in_book,
        100,
    );

    trader_0
        .place_order(ctx, &product_0, Side::Ask, 100, Fractional::new(1001000, 4))
        .await
        .unwrap();

    let trader_risk_group_0 = trader_0.get_trader_risk_group(&ctx.client).await;
    assert!(trader_risk_group_0.is_active_product(0).unwrap());
    assert_eq_frac(
        trader_risk_group_0.open_orders.products[0].ask_qty_in_book,
        100,
    );
    let market_product_group = ctx.get_market_product_group().await;
    assert_eq!(market_product_group.name, ctx.product_group_name);

    let market_product_0 = market_product_group
        .find_product_index(&product_0.key())
        .unwrap()
        .1;
    assert_eq!(market_product_0.name, product_0.name);
    assert_eq_frac(market_product_0.get_best_bid(), Fractional::new(1000, 1));
    assert_eq_frac(market_product_0.get_best_ask(), Fractional::new(1001, 1));

    assert_eq_frac(market_product_0.prices.bid, Fractional::new(1000, 1));
    assert_eq_frac(market_product_0.prices.ask, Fractional::new(1001, 1));

    let trader_1 = traders[1].clone();
    trader_1
        .place_order(ctx, &product_0, Side::Ask, 60, 100)
        .await
        .unwrap();

    let trader_2 = traders[2].clone();
    trader_2
        .place_order(ctx, &product_0, Side::Ask, 10, 100)
        .await
        .unwrap();

    let trader_risk_group_1 = trader_1.get_trader_risk_group(&ctx.client).await;
    let product_idx0 = market_product_group
        .find_product_index(&product_0.key())
        .unwrap()
        .0;
    assert!(trader_risk_group_1.is_active_product(product_idx0).unwrap());
    let trader_position_1_0 = trader_risk_group_1.trader_positions
        [trader_risk_group_1.active_products[product_idx0] as usize];
    assert_eq!(trader_position_1_0.tag, AccountTag::TraderPosition);
    assert_eq!(trader_position_1_0.product_key, product_0.key());
    assert_eq!(trader_position_1_0.product_index, product_idx0);
    assert_eq_frac(
        trader_risk_group_1.open_orders.products[product_idx0].ask_qty_in_book,
        0,
    );
    assert_eq_frac(
        trader_risk_group_1.open_orders.products[product_idx0].bid_qty_in_book,
        0,
    );
    assert_eq_frac(trader_position_1_0.pending_position, -60);
    assert_eq_frac(trader_risk_group_1.pending_cash_balance, 60 * 100);
    assert_eq_frac(trader_risk_group_1.pending_fees, 60 * 100 * taker_fee);

    let trader_risk_group_2 = trader_2.get_trader_risk_group(&ctx.client).await;
    assert!(trader_risk_group_2.is_active_product(product_idx0).unwrap());
    let tpi = trader_risk_group_2.active_products[product_idx0] as usize;
    let trader_position_2_0 = trader_risk_group_2.trader_positions[tpi];
    assert_eq_frac(trader_position_2_0.pending_position, -10);
    assert_eq_frac(trader_risk_group_2.pending_cash_balance, 1000);
    assert_eq_frac(trader_risk_group_2.pending_fees, 10 * 100 * taker_fee);
    assert_eq_frac(trader_position_2_0.position, 0);
    assert_eq_frac(trader_risk_group_2.cash_balance, 0);

    trader_1
        .crank(ctx, &product_0, &[&trader_0, &trader_2])
        .await
        .unwrap();

    let trader_risk_group_1 = trader_1.get_trader_risk_group(&ctx.client).await;
    assert!(trader_risk_group_1.is_active_product(product_idx0).unwrap());
    let tpi = trader_risk_group_1.active_products[product_idx0] as usize;
    let trader_position_1_0 = trader_risk_group_1.trader_positions[tpi];
    assert_eq_frac(trader_position_1_0.pending_position, 0);
    assert_eq_frac(trader_risk_group_1.pending_cash_balance, 0);
    assert_eq_frac(trader_risk_group_1.pending_fees, 0);
    assert_eq_frac(trader_position_1_0.position, -60);
    assert_eq_frac(
        trader_risk_group_1.cash_balance,
        60 * 100 * (1 + (-taker_fee)),
    );

    let trader_risk_group_0 = trader_0.get_trader_risk_group(&ctx.client).await;
    assert!(trader_risk_group_0.is_active_product(product_idx0).unwrap());
    let tpi = trader_risk_group_0.active_products[product_idx0] as usize;
    let trader_position_00 = trader_risk_group_0.trader_positions[tpi];
    assert_eq_frac(
        trader_risk_group_0.open_orders.products[product_idx0].bid_qty_in_book,
        30,
    );
    assert_eq_frac(trader_position_00.position, 70);
    assert_eq_frac(
        trader_risk_group_0.cash_balance,
        -70 * 100 * (maker_fee + 1), // 6006 bc of 10 bip maker fee
    );
    assert!(trader_risk_group_0.open_orders.products[product_idx0].head_index as usize != SENTINEL);

    let trader_risk_group_2 = trader_2.get_trader_risk_group(&ctx.client).await;
    assert!(trader_risk_group_2.is_active_product(product_idx0).unwrap());
    let tpi = trader_risk_group_2.active_products[product_idx0] as usize;
    let trader_position_2_0 = trader_risk_group_2.trader_positions[tpi];
    assert_eq_frac(trader_position_2_0.pending_position, 0);
    assert_eq_frac(trader_risk_group_2.pending_cash_balance, 0);
    assert_eq_frac(trader_risk_group_2.pending_fees, 0);
    assert_eq_frac(trader_position_2_0.position, -10);
    assert_eq_frac(
        trader_risk_group_2.cash_balance,
        10 * 100 * (1 + (-taker_fee)),
    );
    let trader_risk_group_0 = trader_0.get_trader_risk_group(&ctx.client).await;
    assert!(trader_risk_group_0.is_active_product(product_idx0).unwrap());
    let tpi = trader_risk_group_0.active_products[product_idx0] as usize;
    let trader_position_00 = trader_risk_group_0.trader_positions[tpi];
    assert_eq_frac(
        trader_risk_group_0.open_orders.products[product_idx0].bid_qty_in_book,
        30,
    );
    assert_eq_frac(trader_position_00.position, Fractional::from(70));
    assert_eq_frac(
        trader_risk_group_0.cash_balance,
        -70 * 100 * (maker_fee + 1), // maker fee 10 bips
    );
    assert!(trader_risk_group_0.open_orders.products[product_idx0].head_index as usize != SENTINEL);

    let order_index = trader_risk_group_0.open_orders.products[product_idx0].head_index;
    let order = trader_risk_group_0.open_orders.orders[order_index as usize];
    let order_id = order.id;

    let market_product_group = ctx.get_market_product_group().await;
    let market_product_0 = market_product_group
        .find_outright(&product_0.key())
        .unwrap()
        .1;
    assert!(market_product_0.open_long_interest == Fractional::new(700, 1));
    assert!(market_product_0.open_short_interest == Fractional::new(700, 1));

    trader_0.cancel(ctx, &product_0, order_id).await.unwrap();

    let trader_risk_group_0 = trader_0.get_trader_risk_group(&ctx.client).await;
    assert!(trader_risk_group_0.is_active_product(product_idx0).unwrap());
    let t1_p1_i = trader_risk_group_0.active_products[product_idx0] as usize;
    let trader_position_00 = trader_risk_group_0.trader_positions[t1_p1_i];
    assert_eq_frac(trader_position_00.position, 70);
    assert_eq_frac(
        trader_risk_group_0.cash_balance,
        -70 * 100 * (maker_fee + 1), // 10 bips fee
    );
}
