#![allow(non_snake_case)]

use agnostic_orderbook::state::{SelfTradeBehavior, Side};

use dexteritysdk::common::utils::*;

use crate::setup::bootstrap_tests;

mod setup;

#[tokio::test]
async fn test_self_trade__abort_transaction() -> SDKResult {
    let (ctx, traders) =
        &mut bootstrap_tests("noop_risk_engine", "constant_fees", "test", 1, 1).await;
    let trader = &traders[0].clone();
    let product = &ctx.products[0].clone();

    trader.place_order(ctx, product, Side::Bid, 40, 110).await?;
    assert!(trader
        .place_order_with_self_trade_behavior(
            ctx,
            product,
            Side::Ask,
            10,
            100,
            SelfTradeBehavior::AbortTransaction,
            &[],
            dex::state::enums::OrderType::Limit,
        )
        .await
        .is_err());

    let trg = trader.get_trader_risk_group(&ctx.client).await;
    let open_order = &trg.open_orders.products[0];
    assert_eq_frac(open_order.bid_qty_in_book, 40);
    assert_eq_frac(open_order.ask_qty_in_book, 0);
    Ok(())
}

#[tokio::test]
async fn test_self_trade__cancel_provide() -> SDKResult {
    // Cancel provide - the order on the provide side is cancelled. Matching for the current order continues and essentially bypasses the self-provided order.
    let (ctx, traders) =
        &mut bootstrap_tests("noop_risk_engine", "constant_fees", "test", 1, 1).await;
    let trader = &traders[0].clone();
    let product = &ctx.products[0].clone();

    trader.place_order(ctx, product, Side::Bid, 40, 110).await?;
    trader
        .place_order_with_self_trade_behavior(
            ctx,
            product,
            Side::Ask,
            10,
            100,
            SelfTradeBehavior::CancelProvide,
            &[],
            dex::state::enums::OrderType::Limit,
        )
        .await?;
    trader.crank(ctx, product, &[]).await?;
    let trg = trader.get_trader_risk_group(&ctx.client).await;
    let open_order = &trg.open_orders.products[0];
    assert_eq_frac(open_order.bid_qty_in_book, 0);
    assert_eq_frac(open_order.ask_qty_in_book, 10);
    Ok(())
}

#[tokio::test]
async fn test_self_trade__decrement_take() -> SDKResult {
    // Decrement take - the orders are matched together
    let (ctx, traders) =
        &mut bootstrap_tests("noop_risk_engine", "constant_fees", "test", 1, 1).await;
    let trader = &traders[0].clone();
    let product = &ctx.products[0].clone();

    trader.place_order(ctx, product, Side::Ask, 40, 100).await?;

    trader
        .place_order_with_self_trade_behavior(
            ctx,
            product,
            Side::Bid,
            10,
            110,
            SelfTradeBehavior::DecrementTake,
            &[],
            dex::state::enums::OrderType::Limit,
        )
        .await?;
    trader.crank(ctx, product, &[]).await?;
    let trg = trader.get_trader_risk_group(&ctx.client).await;
    let open_order = &trg.open_orders.products[0];
    assert_eq_frac(open_order.bid_qty_in_book, 0);
    assert_eq_frac(open_order.ask_qty_in_book, 30);
    Ok(())
}
