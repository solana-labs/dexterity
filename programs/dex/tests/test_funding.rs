#![allow(non_snake_case)]

use agnostic_orderbook::state::Side;
use anchor_lang::solana_program::sysvar::clock::Clock;
use borsh::BorshDeserialize;
use dexteritysdk::{
    common::{utils::*, KeypairD},
    instrument::settle_derivative,
    oracle::{update_clock::*, update_oracle::*},
    processor::update_trader_funding,
};

use dex::utils::numeric::{Fractional, ZERO_FRAC};
use dummy_oracle::state::OraclePrice;
use instruments::state::derivative_metadata::DerivativeMetadata;

use crate::setup::bootstrap_tests;

mod setup;

pub const WINDOW_SIZE: usize = 2;
pub const ERROR_STATUS: u16 = 1;
pub const OK_STATUS: u16 = 1 - ERROR_STATUS;
pub const COLLATERAL: i64 = 1_000_000;

#[tokio::test]
async fn test_dummy_oracle() {
    let n_products = 1;
    let (ctx, _) = &mut bootstrap_tests(
        "noop_risk_engine",
        "constant_fees",
        "cancl_pos_under",
        2,
        n_products,
    )
    .await;
    let market_product_group = ctx.get_market_product_group().await;
    let instrument_pubkey = market_product_group.market_products[0].product_key;

    let derivative_metadata: Box<DerivativeMetadata> = ctx
        .client
        .get_anchor_account::<DerivativeMetadata>(instrument_pubkey)
        .await;

    // update & read clock time
    update_clock_account(
        &ctx.client,
        ctx.dummy_oracle_program_id,
        &ctx.payer,
        solana_program::system_program::id(),
        1,
        2,
        3,
        4,
        5,
    )
    .await
    .unwrap();
    let account = ctx
        .client
        .get_account(derivative_metadata.clock)
        .await
        .unwrap();
    let clock: Clock = bincode::deserialize(&account.data.clone()).unwrap();
    assert_eq!(clock.slot, 1);
    assert_eq!(clock.epoch_start_timestamp, 2);
    assert_eq!(clock.epoch, 3);
    assert_eq!(clock.leader_schedule_epoch, 4);
    assert_eq!(clock.unix_timestamp, 5);

    // update & read oracle price - positive price
    update_oracle_price_account(
        &ctx.client,
        ctx.dummy_oracle_program_id,
        &ctx.payer,
        solana_program::system_program::id(),
        20,
        6,
    )
    .await
    .unwrap();
    //read oracle price
    let account = ctx
        .client
        .get_account(derivative_metadata.price_oracle)
        .await
        .unwrap();
    let oracle_price = OraclePrice::try_from_slice(&account.data.clone()).unwrap();
    assert_eq!(oracle_price.price, 20);
    assert_eq!(oracle_price.decimals, 6);

    // update & read oracle price - negative price
    update_oracle_price_account(
        &ctx.client,
        ctx.dummy_oracle_program_id,
        &ctx.payer,
        solana_program::system_program::id(),
        -20,
        7,
    )
    .await
    .unwrap();

    //read oracle price
    let account = ctx
        .client
        .get_account(derivative_metadata.price_oracle)
        .await
        .unwrap();
    let oracle_price = OraclePrice::try_from_slice(&account.data.clone()).unwrap();
    assert_eq!(oracle_price.price, -20);
    assert_eq!(oracle_price.decimals, 7);

    // change the authority and make sure it does not work
    //Todo: clock and price accounts are not connected. Also, anyone can update the price write now, if it signs the message.
    let err = match update_clock_account(
        &ctx.client,
        ctx.dummy_oracle_program_id,
        &KeypairD::new(),
        solana_program::system_program::id(),
        clock.slot + 1,
        clock.epoch_start_timestamp + 2,
        clock.epoch + 3,
        clock.leader_schedule_epoch + 4,
        clock.unix_timestamp + 5,
    )
    .await
    {
        Ok(_) => OK_STATUS,
        Err(_) => ERROR_STATUS,
    };

    assert_eq!(err, OK_STATUS); // This should be negative as write now authority is a random key pair. (BUG)
    let err = match update_oracle_price_account(
        &ctx.client,
        ctx.dummy_oracle_program_id,
        &KeypairD::new(),
        solana_program::system_program::id(),
        -20,
        7,
    )
    .await
    {
        Ok(_) => OK_STATUS,
        Err(_) => ERROR_STATUS,
    };

    assert_eq!(err, ERROR_STATUS);
}

#[tokio::test]
async fn test_funding() {
    let n_products = 1;
    let (ctx, traders) = &mut bootstrap_tests(
        "noop_risk_engine",
        "constant_fees",
        "test_funding",
        2,
        n_products,
    )
    .await;

    let market_product_group = ctx.get_market_product_group().await;

    let instrument_pubkey = market_product_group.market_products[0].product_key;

    let derivative_metadata: Box<DerivativeMetadata> = ctx
        .client
        .get_anchor_account::<DerivativeMetadata>(instrument_pubkey)
        .await;

    assert_eq_frac(derivative_metadata.strike, 0);

    let initial_prod_price = Fractional::new(25, 0);
    let trade_size = Fractional::new(10, 0);
    let collaterals = [Fractional::new(1000000, 0), Fractional::new(1000000, 0)];
    let oracle_price = 2025000000;
    let oracle_decimals = 6;
    let product_funding = Fractional::new(oracle_price, oracle_decimals) - initial_prod_price;

    // create asset
    let product = &ctx.products[0];

    // Transfer funds
    for (trader, collateral) in traders.iter().zip(collaterals.iter()) {
        trader.deposit(ctx, *collateral).await.unwrap();
    }

    // trade an asset for two parties one long one short
    {
        traders[0]
            .place_order(ctx, product, Side::Bid, trade_size, initial_prod_price)
            .await
            .unwrap();
        {
            let trader_risk_group_0 = traders[0].get_trader_risk_group(&ctx.client).await;
            assert_eq_frac(
                trader_risk_group_0.open_orders.products[0].ask_qty_in_book,
                0,
            );
            assert_eq_frac(
                trader_risk_group_0.open_orders.products[0].bid_qty_in_book,
                trade_size,
            );
        }
        traders[1]
            .place_order(ctx, product, Side::Ask, trade_size, initial_prod_price)
            .await
            .unwrap();

        let trader_risk_group_ask = traders[1].get_trader_risk_group(&ctx.client).await;

        assert_eq_frac(
            trader_risk_group_ask.open_orders.products[0].ask_qty_in_book,
            0,
        );
        assert_eq_frac(
            trader_risk_group_ask.open_orders.products[0].bid_qty_in_book,
            0,
        );
    }

    // Crank
    {
        traders[0]
            .crank(ctx, product, &[&traders[0], &traders[1]])
            .await
            .unwrap();

        let trader_risk_group_0 = traders[0].get_trader_risk_group(&ctx.client).await;

        assert_eq_frac(
            trader_risk_group_0.open_orders.products[0].ask_qty_in_book,
            0,
        );
        assert_eq_frac(
            trader_risk_group_0.open_orders.products[0].bid_qty_in_book,
            0,
        );
    }

    // set mark price
    {
        traders[0]
            .place_order(
                ctx,
                product,
                Side::Bid,
                trade_size,
                initial_prod_price - Fractional::new(1, 0),
            )
            .await
            .unwrap();
        {
            let trader_risk_group_0 = traders[0].get_trader_risk_group(&ctx.client).await;
            assert_eq_frac(
                trader_risk_group_0.open_orders.products[0].ask_qty_in_book,
                0,
            );
            assert_eq_frac(
                trader_risk_group_0.open_orders.products[0].bid_qty_in_book,
                trade_size,
            );
        }
        traders[1]
            .place_order(ctx, product, Side::Ask, trade_size, initial_prod_price + 1)
            .await
            .unwrap();

        let trader_risk_group_ask = traders[1].get_trader_risk_group(&ctx.client).await;

        assert_eq_frac(
            trader_risk_group_ask.open_orders.products[0].ask_qty_in_book,
            trade_size,
        );
        assert_eq_frac(
            trader_risk_group_ask.open_orders.products[0].bid_qty_in_book,
            0,
        );
    }

    // check funding
    {
        let market_product_group = ctx.get_market_product_group().await;

        // check open interests
        {
            let (_, market_product) = market_product_group.find_outright(&product.key).unwrap();
            assert_eq_frac(market_product.cum_funding_per_share, ZERO_FRAC);
        }
        for i in 1..2 {
            let trader_risk_group = traders[i].get_trader_risk_group(&ctx.client).await;
            let tpi = trader_risk_group.active_products[0] as usize;
            let trader_position = trader_risk_group.trader_positions[tpi];
            assert_eq!(trader_position.last_cum_funding_snapshot, ZERO_FRAC);
        }
    }

    // update oracle price
    update_oracle_price_account(
        &ctx.client,
        ctx.dummy_oracle_program_id,
        &ctx.payer,
        solana_program::system_program::id(),
        oracle_price,
        oracle_decimals,
    )
    .await
    .unwrap();

    {
        // Change time to 101, as this is less than the funding period, funding should not happen
        update_clock_account(
            &ctx.client,
            ctx.dummy_oracle_program_id,
            &ctx.payer,
            solana_program::system_program::id(),
            102,
            102,
            102,
            102,
            102,
        )
        .await
        .unwrap();
        let account = ctx
            .client
            .get_account(derivative_metadata.clock)
            .await
            .unwrap();
        let clock: Clock = bincode::deserialize(&account.data.clone()).ok().unwrap();
        assert_eq!(clock.slot, 102);
        assert_eq!(clock.epoch_start_timestamp, 102);
        assert_eq!(clock.epoch, 102);
        assert_eq!(clock.leader_schedule_epoch, 102);
        assert_eq!(clock.unix_timestamp, 102);

        update_clock_account(
            &ctx.client,
            ctx.dummy_oracle_program_id,
            &ctx.payer,
            solana_program::system_program::id(),
            103,
            103,
            103,
            103,
            103,
        )
        .await
        .unwrap();

        // update funding for the product
        // Todo: When I add the following block the next settle_derivative function would not work
        let err = settle_derivative::settle_derivative(
            &ctx.client,
            ctx.market_product_group,
            derivative_metadata.price_oracle,
            derivative_metadata.clock,
            instrument_pubkey,
        )
        .await;
        assert!(!err.is_ok());

        // Change time to 1000
        update_clock_account(
            &ctx.client,
            ctx.dummy_oracle_program_id,
            &ctx.payer,
            solana_program::system_program::id(),
            1000,
            1000,
            1000,
            1000,
            1000,
        )
        .await
        .unwrap();
        let clock_account = ctx
            .client
            .get_account(derivative_metadata.clock)
            .await
            .unwrap();
        let clock: Clock = bincode::deserialize(&clock_account.data.clone()).unwrap();
        assert_eq!(clock.slot, 1000);
        assert_eq!(clock.epoch_start_timestamp, 1000);
        assert_eq!(clock.epoch, 1000);
        assert_eq!(clock.leader_schedule_epoch, 1000);
        assert_eq!(clock.unix_timestamp, 1000);
        //update funding for the product
        let res = settle_derivative::settle_derivative(
            &ctx.client,
            ctx.market_product_group,
            derivative_metadata.price_oracle,
            derivative_metadata.clock,
            instrument_pubkey,
        )
        .await;
        assert!(res.is_ok());
    }

    // check funding after changing the time to 1000 and update product funding
    // now the product funding should have been changed but trader funding should still be zero
    {
        let market_product_group = ctx.get_market_product_group().await;
        // check open interests
        {
            let (_, market_product) = market_product_group.find_outright(&product.key).unwrap();
            assert_eq_frac(market_product.cum_funding_per_share, product_funding);
        }
        for i in 1..2 {
            let trader_risk_group = traders[i].get_trader_risk_group(&ctx.client).await;
            let tpi = trader_risk_group.active_products[0] as usize;
            let trader_position = trader_risk_group.trader_positions[tpi];
            assert_eq!(trader_position.last_cum_funding_snapshot, ZERO_FRAC);
        }
    }

    // update trader funding
    for i in 0..2 {
        update_trader_funding::update_trader_funding(
            &ctx.client,
            traders[i].account,
            ctx.market_product_group,
        )
        .await
        .unwrap();
    }

    // check funding, now the trader funding should also have been changed
    {
        let market_product_group = ctx.get_market_product_group().await;
        // check open interests
        {
            let market_product = market_product_group.find_outright(&product.key).unwrap().1;
            assert_eq_frac(market_product.cum_funding_per_share, product_funding);
        }
        for i in 1..2 {
            let trader_risk_group = traders[i].get_trader_risk_group(&ctx.client).await;
            let tpi = trader_risk_group.active_products[0] as usize;
            let trader_position = trader_risk_group.trader_positions[tpi];
            assert_eq!(trader_position.last_cum_funding_snapshot, product_funding);
        }
    }
}
