#![allow(non_snake_case)]

use agnostic_orderbook::state::Side;
use anchor_lang::Key;
use dex::{
    state::{constants::*, risk_engine_register::*},
    utils::numeric::{Fractional, ZERO_FRAC},
};
use dexteritysdk::common::utils::*;
use rand::Rng;

mod setup;
use crate::setup::*;

pub const WINDOW_SIZE: usize = 2;
pub const ERROR_STATUS: u16 = 1;
pub const OK_STATUS: u16 = 1 - ERROR_STATUS;
pub const COLLATERAL: i64 = 1_000_000;
pub const LIQUIDATION_INFO_CONST: LiquidationInfo = LiquidationInfo {
    health: HealthStatus::Healthy,
    action: ActionStatus::Approved,
    total_social_loss: ZERO_FRAC,
    liquidation_price: ZERO_FRAC,
    social_losses: [SocialLoss {
        product_index: 0,
        amount: ZERO_FRAC,
    }; MAX_TRADER_POSITIONS],
};

pub const HEALTH_INFO_CONST: HealthInfo = HealthInfo {
    health: HealthStatus::Healthy,
    action: ActionStatus::Approved,
};

// This tests the simple single user, single product case.
#[tokio::test]
async fn test_alpha_risk_engine_simple() {
    log_disable();
    let num_traders = 1;
    let num_products = 1;
    let (ctx, traders) = &mut bootstrap_tests(
        "alpha_risk_engine",
        "constant_fees",
        "test",
        num_traders,
        num_products,
    )
    .await;
    let product_0 = ctx.products[0].clone();
    let trader_0 = traders[0].clone();

    // You cannot place an order with no collateral.
    let error_status = trader_0
        .place_order(ctx, &product_0, Side::Bid, 1, 100)
        .await;
    assert!(error_status.is_err());

    // Checks that collateral was correctly deposited.
    trader_0.deposit(ctx, COLLATERAL).await.unwrap();
    assert_eq_frac(
        trader_0
            .get_trader_risk_group(&ctx.client)
            .await
            .cash_balance,
        COLLATERAL,
    );

    // Placing a bid order with collateral.
    let new_status = trader_0
        .place_order(ctx, &product_0, Side::Bid, 10, 100)
        .await;
    assert!(new_status.is_ok());

    // Placing an ask order with collateral.
    let new_status = trader_0
        .place_order(ctx, &product_0, Side::Ask, 10, 100)
        .await;
    assert!(new_status.is_ok());

    // Placing an order with more collateral than you have should not be ok.
    let new_status = trader_0
        .place_order(ctx, &product_0, Side::Bid, 1000, 10000)
        .await;
    assert!(new_status.is_err());
}

// This tests a more complex multi user, multi product case.
#[tokio::test]
async fn test_alpha_risk_engine_complex() {
    let num_products = 10;
    let num_traders = 10;
    let (ctx, genTraders) = &mut bootstrap_tests(
        "alpha_risk_engine",
        "constant_fees",
        "alpha_risk_eng",
        num_traders,
        num_products,
    )
    .await;
    let products = ctx.products.clone();
    let traders = genTraders.clone();
    let mut rng = rand::thread_rng();

    for trader in &traders {
        trader.deposit(ctx, COLLATERAL).await.unwrap();
        assert_eq_frac(
            trader.get_trader_risk_group(&ctx.client).await.cash_balance,
            COLLATERAL,
        );
    }

    // For every product, every trader will place a bid and an ask.
    for product in products {
        let size = rng.gen_range(1..10);
        let price = rng.gen_range(50..100);
        for trader in &traders {
            assert!(trader
                .place_order(
                    ctx,
                    &product,
                    Side::Bid,
                    Fractional::new(size, 0),
                    Fractional::new(price, 0),
                )
                .await
                .is_ok());
            assert!(trader
                .place_order(
                    ctx,
                    &product,
                    Side::Ask,
                    Fractional::new(size, 0),
                    Fractional::new(price, 0),
                )
                .await
                .is_ok());
        }
    }
}

#[tokio::test]
// Tests if the liquidation mechanism functions.
async fn test_alpha_risk_engine_liquidation() {
    log_disable();
    let product_initial_price = Fractional::new(200, 0);
    let product_liquidation_price = Fractional::new(1000, 0);
    let trade_size = Fractional::new(1, 0);
    let user_collateral = Fractional::new(1000, 0);
    let num_products = 1 as u32;
    let num_traders = num_products + 2;

    // This is the price we expect the liquidator to pay the liquidatee for their portfolio at liquidation.
    let EXPECTED_LIQUIDATION_PRICE = 140;

    // The nominal value of the portfolio at liquidation time.
    let final_portfolio_value = user_collateral + product_initial_price - product_liquidation_price;
    println!("final pv {}", final_portfolio_value);

    // We want to incentivize liquidating portfolios before they go underwater, so we give a larger discount if the portfolio is positive.
    let alpha = if final_portfolio_value.m >= 0 {
        Fractional::new(9, 1)
    } else {
        Fractional::from(1)
    };
    let beta = Fractional::new(2, 1);
    let alpha_portfolio_value = alpha.checked_mul(final_portfolio_value).unwrap();
    println!("alpha pv {}", alpha_portfolio_value);
    let liquidation_price = alpha_portfolio_value - (final_portfolio_value * beta);
    println!("liq price {}", liquidation_price);

    let initial_cash = user_collateral + product_initial_price;
    assert_eq!(
        liquidation_price,
        Fractional::new(EXPECTED_LIQUIDATION_PRICE, 0)
    );

    let (ctx, traders) = &mut bootstrap_tests(
        "alpha_risk_engine",
        "constant_fees",
        "liquidable_pos",
        num_traders,
        num_products,
    )
    .await;
    let products = ctx.products.clone();

    // Deposit collateral.
    for trader in traders.iter() {
        trader.deposit(ctx, user_collateral).await.unwrap();
    }

    let _deposit = &traders[2].deposit(ctx, 10000).await.unwrap();

    set_prices(
        ctx,
        &traders[2],
        vec![0],
        &vec![product_initial_price],
        false,
    )
    .await
    .unwrap();

    // Trade
    let new_status = traders[0]
        .place_order(
            ctx,
            &products[0],
            Side::Bid,
            trade_size,
            product_initial_price,
        )
        .await;

    assert!(new_status.is_ok());

    let new_status = traders[1]
        .place_order(
            ctx,
            &products[0],
            Side::Ask,
            trade_size,
            product_initial_price,
        )
        .await;

    assert!(new_status.is_ok());

    let market_product_group_before = ctx.get_market_product_group().await;
    assert_eq!(
        market_product_group_before.market_products[0].prices.bid,
        Fractional::new(100, 0)
    );
    assert_eq!(
        market_product_group_before.market_products[0].prices.ask,
        Fractional::new(300, 0)
    );

    let _cancel = &traders[2].cancel_all_orders(ctx, &[0]).await;
    set_prices(
        ctx,
        &traders[2],
        vec![0],
        &vec![product_liquidation_price],
        false,
    )
    .await
    .unwrap();

    let market_product_group_before = ctx.get_market_product_group().await;

    assert_eq!(
        market_product_group_before.market_products[0].prices.bid,
        Fractional::new(500, 0)
    );
    assert_eq!(
        market_product_group_before.market_products[0].prices.ask,
        Fractional::new(1500, 0)
    );

    let _crank = &traders[0]
        .crank(ctx, &products[0], &[&traders[1]])
        .await
        .unwrap();

    // liquidatee cash before liquidation
    let trader_risk_group_1 = traders[1].get_trader_risk_group(&ctx.client).await;
    let liquidatee_cash_before = trader_risk_group_1.cash_balance;

    // liquidator cash before liquidation
    let trader_risk_group_0 = traders[0].get_trader_risk_group(&ctx.client).await;
    let liquidator_cash_before = trader_risk_group_0.cash_balance;

    // liquidate
    let new_status = traders[0]
        .transfer_position(
            ctx,
            ctx.market_product_group,
            traders[1].account,
            traders[1].risk_state_account,
        )
        .await;

    assert!(new_status.is_ok());

    // liquidatee cash after liquidation
    let trader_risk_group_1 = traders[1].get_trader_risk_group(&ctx.client).await;
    let liquidatee_cash_after = trader_risk_group_1.cash_balance;

    // liquidator cash after liquidation
    let trader_risk_group_0 = traders[0].get_trader_risk_group(&ctx.client).await;
    let liquidator_cash_after = trader_risk_group_0.cash_balance;
    let liquidator_cash_profit = initial_cash.checked_sub(liquidation_price).unwrap();

    assert_eq!(
        liquidator_cash_after
            .checked_sub(liquidator_cash_before)
            .unwrap()
            .round_sf(0),
        liquidator_cash_profit.round_sf(0)
    );
    assert!(liquidatee_cash_after > ZERO_FRAC);
    println!("liquidatee cash before {}", liquidatee_cash_before);
    println!("liquidator cash before {}", liquidator_cash_before);
    println!("liquidatee cash after {}", liquidatee_cash_after);
    println!("liquidator cash after {}", liquidator_cash_after);
    println!("liquidator profit {}", liquidator_cash_profit);
    // check social loss amounts after the liquidation for the products
    {
        let mut total_social_loss_after = ZERO_FRAC;
        let market_product_group_after = ctx.get_market_product_group().await;
        let mut cum_social_loss_diff = ZERO_FRAC;
        for n in 0..num_products {
            let (_, market_product_before) = market_product_group_before
                .find_outright(&products[n as usize].key())
                .unwrap();
            let (_, market_product_after) = market_product_group_after
                .find_outright(&products[n as usize].key())
                .unwrap();
            // Check cum_social_loss_per_share change is equal for all products
            if n == 0 {
                cum_social_loss_diff = market_product_after.cum_social_loss_per_share
                    - market_product_before.cum_social_loss_per_share;
            } else {
                assert_eq!(
                    cum_social_loss_diff,
                    market_product_after.cum_social_loss_per_share
                        - market_product_before.cum_social_loss_per_share
                )
            }
            if market_product_after.cum_social_loss_per_share == ZERO_FRAC {
                continue;
            }
            let open_interest_before = market_product_before.open_short_interest
                + market_product_before.open_long_interest;
            let open_interest_after =
                market_product_after.open_short_interest + market_product_after.open_long_interest;
            total_social_loss_after += market_product_after.cum_social_loss_per_share
                * (open_interest_after)
                + market_product_after.dust
                - (market_product_before.cum_social_loss_per_share * (open_interest_before)
                    + market_product_before.dust);
        }
        // There should be no social loss because open interest dropped to 0 after the liquidation
        let m = ZERO_FRAC.exp.min(total_social_loss_after.exp).min(1) as u32;
        assert_eq!(total_social_loss_after.round_sf(m), ZERO_FRAC,);
    }
}
