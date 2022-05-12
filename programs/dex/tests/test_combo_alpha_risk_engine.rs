#![allow(non_snake_case)]
use agnostic_orderbook::state::Side;
use anchor_lang::{solana_program::program_pack::Pack, Key};
use dex::utils::numeric::Fractional;
use dexteritysdk::{bootstrap::setup_combo, common::utils::*};
use itertools::Itertools;

use solana_sdk::signature::Signer;

mod setup;
use crate::setup::*;

pub const WINDOW_SIZE: usize = 2;
pub const ERROR_STATUS: u16 = 1;
pub const OK_STATUS: u16 = 1 - ERROR_STATUS;
pub const COLLATERAL: i64 = 1_000_000;

#[tokio::test]
async fn test_combo_trade_risk_engine() {
    let (ctx, traders) =
        &mut bootstrap_tests("alpha_risk_engine", "constant_fees", "test", 3, 4).await;

    let combo_1 = setup_combo(
        ctx,
        ctx.products[0..2]
            .iter()
            .map(|p| p.key())
            .sorted()
            .collect::<Vec<_>>()
            .as_slice(),
        0,
    )
    .await
    .unwrap();
    let combo_2 = setup_combo(
        ctx,
        ctx.products[2..4]
            .iter()
            .map(|p| p.key())
            .sorted()
            .collect::<Vec<_>>()
            .as_slice(),
        1,
    )
    .await
    .unwrap();

    let trader_0 = traders[0].clone();
    let trader_1 = traders[1].clone();

    trader_0.deposit(ctx, COLLATERAL).await.unwrap();
    trader_1.deposit(ctx, COLLATERAL).await.unwrap();

    trader_0
        .place_combo_order(
            ctx,
            &combo_1,
            Side::Bid,
            Fractional::new(1, 0),
            Fractional::new(-1, 0),
        )
        .await
        .unwrap();

    trader_1
        .place_combo_order(
            ctx,
            &combo_1,
            Side::Ask,
            Fractional::new(1, 0),
            Fractional::new(-1, 0),
        )
        .await
        .unwrap();

    trader_0
        .place_combo_order(
            ctx,
            &combo_2,
            Side::Bid,
            Fractional::new(1, 0),
            Fractional::new(-1, 0),
        )
        .await
        .unwrap();

    trader_1
        .place_combo_order(
            ctx,
            &combo_2,
            Side::Ask,
            Fractional::new(1, 0),
            Fractional::new(-1, 0),
        )
        .await
        .unwrap();
}

#[tokio::test]
async fn test_combo_test() -> SDKResult {
    let (ctx, traders) =
        &mut bootstrap_tests("alpha_risk_engine", "constant_fees", "test", 3, 2).await;
    let product_0 = ctx.products[0].clone();
    let product_1 = ctx.products[1].clone();
    let trader_0 = traders[0].clone();
    let trader_1 = traders[1].clone();
    let trader_2 = traders[2].clone();

    /*
    Need 3 traders: one to set mark price
    and other two to show risk comparison
    */
    // Transfer funds here
    trader_0.deposit(ctx, COLLATERAL).await.unwrap();

    let account = ctx.client.get_account(trader_1.wallet).await?;
    let token_account =
        spl_token::state::Account::unpack_unchecked(account.data.as_slice()).unwrap();
    assert_eq!(token_account.owner, trader_1.keypair.pubkey());

    trader_1.deposit(ctx, COLLATERAL).await.unwrap();
    trader_2.deposit(ctx, COLLATERAL).await.unwrap();

    // Creating a history of prices for combo legs
    trader_1
        .place_order(
            ctx,
            &product_0,
            Side::Bid,
            Fractional::new(100, 1),
            Fractional::new(980000, 4),
        )
        .await
        .unwrap();

    trader_1
        .place_order(
            ctx,
            &product_0,
            Side::Ask,
            Fractional::new(100, 1),
            Fractional::new(1040000, 4),
        )
        .await
        .unwrap();

    trader_1
        .place_order(
            ctx,
            &product_1,
            Side::Bid,
            Fractional::new(100, 1),
            Fractional::new(900000, 4),
        )
        .await
        .unwrap();

    trader_1
        .place_order(
            ctx,
            &product_1,
            Side::Ask,
            Fractional::new(100, 1),
            Fractional::new(1100000, 4),
        )
        .await
        .unwrap();

    let market_product_group = ctx.get_market_product_group().await;
    assert_eq!(market_product_group.name, ctx.product_group_name);
    assert_eq!(market_product_group.active_flags_products.inner[0], 3);

    let market_product_0 = market_product_group
        .find_outright(&product_0.key())
        .unwrap()
        .1;

    let prices_0 = market_product_0.prices;
    assert_eq!(prices_0.bid, Fractional::new(980000, 4));
    assert_eq!(prices_0.ask, Fractional::new(1040000, 4),);

    // Build combo here
    let combo = setup_combo(
        ctx,
        ctx.products
            .iter()
            .map(|p| p.key())
            .collect::<Vec<_>>()
            .as_slice(),
        0,
    )
    .await?;

    trader_2
        .place_order(
            ctx,
            &product_0,
            Side::Bid,
            Fractional::new(1000, 1),
            Fractional::new(800000, 4),
        )
        .await
        .unwrap();

    trader_2
        .place_order(
            ctx,
            &product_1,
            Side::Ask,
            Fractional::new(1000, 1),
            Fractional::new(1200000, 4),
        )
        .await
        .unwrap();
    trader_0
        .place_combo_order(
            ctx,
            &combo,
            Side::Bid,
            Fractional::new(1000, 1),
            Fractional::new(-11, 1),
        )
        .await
        .unwrap();
    Ok(())
}
