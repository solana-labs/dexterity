#![allow(non_snake_case)]
use agnostic_orderbook::state::{Event, Side};
use anchor_lang::Key;
use dex::{
    state::{constants::*, market_product_group::*, trader_risk_group::*},
    utils::numeric::Fractional,
};
use dexteritysdk::{
    common::KeypairD,
    processor::{orderbook::create_orderbook_with_params, update_product_funding},
    state::SDKProduct,
};
use itertools::Itertools;
use solana_program::pubkey::Pubkey;
use solana_sdk::signature::Signer;

use dexteritysdk::bootstrap::setup_combo;

mod setup;
use crate::setup::bootstrap_tests;

pub const ERROR_STATUS: u16 = 1;
pub const OK_STATUS: u16 = 1 - ERROR_STATUS;
pub const COLLATERAL: i64 = 1_000_000;

#[tokio::test]
async fn test_expiration() {
    let n_products = 0;
    let (ctx, traders) = &mut bootstrap_tests(
        "noop_risk_engine",
        "constant_fees",
        "test_expiration",
        3,
        n_products,
    )
    .await;

    let mut products = vec![];
    let mut signers = vec![];

    for i in 0..2 {
        let product = KeypairD::new();
        let (market_signer, _) =
            Pubkey::find_program_address(&[product.pubkey().as_ref()], &ctx.dex_program_id);
        let event_size = Event::compute_slot_size(40) as u64;
        let (orderbook_key, bids_key, asks_key, eq_key) = create_orderbook_with_params(
            &ctx.client,
            ctx.aaob_program_id,
            market_signer,
            75 + event_size * 5000,
            10000,
            10000,
            1, // min_base_order_size
            1000,
        )
        .await
        .unwrap();
        let name_str = format!("product{:width$}", i, width = NAME_LEN - 7);
        let mut name: [u8; NAME_LEN] = Default::default();
        name.clone_from_slice(name_str.as_bytes());
        ctx.initialize_market_product(
            product.pubkey(),
            orderbook_key,
            name,
            Fractional::new(1, 1),
            6,
            0,
        )
        .await
        .unwrap();
        products.push(SDKProduct {
            name,
            key: product.pubkey(),
            orderbook: orderbook_key,
            bids: bids_key,
            asks: asks_key,
            event_queue: eq_key,
            market_signer,
        });
        signers.push(product);
    }

    let combo = setup_combo(
        ctx,
        products
            .iter()
            .map(|p| p.key())
            .sorted()
            .collect::<Vec<_>>()
            .as_slice(),
        0,
    )
    .await
    .unwrap();

    traders[0]
        .place_order(ctx, &products[0], Side::Bid, 10, 100)
        .await
        .unwrap();

    traders[0]
        .place_order(ctx, &products[0], Side::Ask, 5, 200)
        .await
        .unwrap();

    traders[1]
        .place_order(ctx, &products[0], Side::Ask, 5, 100)
        .await
        .unwrap();

    traders[1]
        .crank(ctx, &products[0], &[&traders[0]])
        .await
        .unwrap();

    traders[2]
        .place_order(ctx, &products[0], Side::Ask, 3, 100)
        .await
        .unwrap();

    traders[2]
        .place_combo_order(ctx, &combo, Side::Bid, 5, -Fractional::new(4, 1))
        .await
        .unwrap();

    traders[2]
        .place_combo_order(ctx, &combo, Side::Ask, 5, -Fractional::new(2, 1))
        .await
        .unwrap();

    traders[1]
        .place_combo_order(ctx, &combo, Side::Bid, 5, -Fractional::new(2, 1))
        .await
        .unwrap();

    update_product_funding::update_product_funding(
        &ctx.client,
        ctx.market_product_group,
        &signers[0],
        Fractional::from(10),
        true,
    )
    .await
    .unwrap();

    let mpg = ctx
        .client
        .get_anchor_account::<MarketProductGroup>(ctx.market_product_group)
        .await;
    assert_eq!(
        mpg.market_products[0]
            .try_to_outright()
            .unwrap()
            .cum_funding_per_share,
        Fractional::new(10, 0)
    );
    assert!(
        mpg.market_products[0]
            .try_to_outright()
            .unwrap()
            .num_queue_events
            > 0
    );
    println!(
        "{}",
        mpg.market_products[0]
            .try_to_outright()
            .unwrap()
            .num_queue_events
    );

    for trader in traders.iter() {
        trader
            .apply_funding(ctx, ctx.market_product_group)
            .await
            .unwrap();
        let trg = ctx
            .client
            .get_anchor_account::<TraderRiskGroup>(trader.account)
            .await;
        assert_eq!(
            trg.trader_positions[0].last_cum_funding_snapshot,
            Fractional::new(0, 0)
        );
    }

    traders[2]
        .crank(ctx, &products[0], &[&traders[0]])
        .await
        .unwrap();

    let mpg = ctx
        .client
        .get_anchor_account::<MarketProductGroup>(ctx.market_product_group)
        .await;
    assert_eq!(
        mpg.market_products[0]
            .try_to_outright()
            .unwrap()
            .cum_funding_per_share,
        Fractional::new(10, 0)
    );
    assert!(
        mpg.market_products[0]
            .try_to_outright()
            .unwrap()
            .num_queue_events
            > 0
    );
    println!(
        "{}",
        mpg.market_products[0]
            .try_to_outright()
            .unwrap()
            .num_queue_events
    );

    for trader in traders.iter() {
        trader
            .apply_funding(ctx, ctx.market_product_group)
            .await
            .unwrap();
        let trg = ctx
            .client
            .get_anchor_account::<TraderRiskGroup>(trader.account)
            .await;
        assert_eq!(
            trg.trader_positions[0].last_cum_funding_snapshot,
            Fractional::new(0, 0)
        );
    }

    let mut trader_keys = vec![
        traders[0].account,
        traders[0].fee_acct,
        traders[1].account,
        traders[1].fee_acct,
        traders[2].account,
        traders[2].fee_acct,
    ];
    trader_keys.sort_by(|a, b| a.as_ref().cmp(b.as_ref()));
    ctx.crank_raw(
        combo.key,
        combo.market_signer,
        combo.orderbook,
        combo.event_queue,
        &traders[0].keypair,
        trader_keys.as_mut_slice(),
        4,
    )
    .await
    .unwrap();

    let mpg = ctx
        .client
        .get_anchor_account::<MarketProductGroup>(ctx.market_product_group)
        .await;
    assert_eq!(
        mpg.market_products[0]
            .try_to_outright()
            .unwrap()
            .cum_funding_per_share,
        Fractional::new(10, 0)
    );
    assert_eq!(
        mpg.market_products[0]
            .try_to_outright()
            .unwrap()
            .num_queue_events,
        0
    );

    let mut trgs = vec![];
    for trader in traders.iter() {
        trader
            .apply_funding(ctx, ctx.market_product_group)
            .await
            .unwrap();
        trgs.push(
            ctx.client
                .get_anchor_account::<TraderRiskGroup>(trader.account)
                .await,
        );
    }
}
