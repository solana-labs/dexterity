#![allow(non_snake_case)]
use agnostic_orderbook::state::{Event, Side};
use anchor_lang::solana_program::pubkey::Pubkey;
use constant_fees::update_fees_ix;
use dex::{state::constants::*, utils::numeric::Fractional};
use dexteritysdk::{common::utils::*, processor::orderbook::*, state::SDKProduct};
use solana_sdk::signature::{Keypair, Signer};

mod setup;
use crate::setup::bootstrap_tests;

#[tokio::test]
async fn test_new_order_rounding_logic() {
    let num_products = 0;
    let num_traders = 2;
    let (ctx, traders) = &mut bootstrap_tests(
        "noop_risk_engine",
        "constant_fees",
        "test",
        num_traders,
        num_products,
    )
    .await;

    let mut products: Vec<SDKProduct> = vec![];

    for (i, decimals) in [9, 10].iter().enumerate() {
        let product = Keypair::new();
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
            100000, // min_base_order_size
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
            Fractional::new(1, *decimals),
            *decimals,
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
        })
    }

    // // set zero fees
    ctx.client
        .sign_send_instructions(
            vec![update_fees_ix(
                ctx.fee_model_program_id,
                ctx.payer.pubkey(),
                ctx.fee_model_config_acct,
                ctx.market_product_group,
                anchor_lang::solana_program::system_program::id(),
                constant_fees::UpdateFeesParams {
                    maker_fee_bps: 0,
                    taker_fee_bps: 0,
                },
            )],
            vec![&ctx.payer],
        )
        .await
        .unwrap();

    let product_0 = products[0].clone();
    let _product_1 = products[1].clone();
    let trader_0 = traders[0].clone();
    let trader_1 = traders[1].clone();
    let price = Fractional::new(4923, 9); // Fractional::from_str("0.000004923").unwrap();
    trader_0
        .place_order(
            ctx,
            &product_0,
            Side::Bid,
            1200000, // Fractional::from_str("1200000.0").unwrap(), // size
            price,
        )
        .await
        .unwrap();
    trader_1
        .place_order(
            ctx,
            &product_0,
            Side::Ask,
            200000, // Fractional::from_str("200000.0").unwrap(), // size
            price,
        )
        .await
        .unwrap();

    // GOT TO FIX THE CRANK -- IT HANGS ARBITRARILY ON GitHub
    // trader_1.crank(ctx, &product_0, &[&trader_0]).await.unwrap();

    let trg1 = trader_1.get_trader_risk_group(&ctx.client).await;
    assert_eq_frac(-200000, trg1.trader_positions[0].pending_position);
    assert_eq!(Fractional::new(984600, 6), trg1.pending_cash_balance);

    // assert!(trg0.open_orders.products[0].bid_qty_in_book > ZERO_FRAC);
    // trader_1
    //     .place_order(
    //         ctx,
    //         &product_0,
    //         Side::Ask,
    //         Fractional::from_str("20").unwrap(),
    //         Fractional::from_str("5.4923").unwrap(),
    //     )
    //     .await
    //     .unwrap();

    // trader_1
    //     .crank(ctx, &product_0, &[&trader_0])
    //     .await
    //     .unwrap();
    // let trg0_cb = trader_0.get_trader_risk_group(&ctx.client)
    //     .await
    //     .cash_balance;
    // let trg1_cb = trader_1.get_trader_risk_group(&ctx.client)
    //     .await
    //     .cash_balance;
    // trader_0
    //     .place_order(
    //         ctx,
    //         &product_1,
    //         Side::Bid,
    //         Fractional::from_str("0.1").unwrap(),
    //         Fractional::from_str("0.0000001").unwrap(),
    //     )
    //     .await
    //     .unwrap();
    // trader_1
    //     .place_order(
    //         ctx,
    //         &product_1,
    //         Side::Ask,
    //         Fractional::from_str("0.1").unwrap(),
    //         Fractional::from_str("0.000000001").unwrap(),
    //     )
    //     .await
    //     .unwrap();
    // trader_1
    //     .crank(ctx, &product_1, &[&trader_0])
    //     .await
    //     .unwrap();

    // let trg0 = trader_0.get_trader_risk_group(&ctx.client).await;
    // let trg1 = trader_1.get_trader_risk_group(&ctx.client).await;
    // assert_eq!(
    //     trg0.trader_positions[1].position,
    //     -trg1.trader_positions[1].position
    // );
    // assert!(trg0.trader_positions[1].position.abs() > ZERO_FRAC,);
    // assert!(trg1.trader_positions[1].position.abs() > ZERO_FRAC,);
    // assert_eq!(trg0.cash_balance - trg0_cb, trg1_cb - trg1.cash_balance);
    // assert_eq!(trg0.cash_balance - trg0_cb, trg1_cb - trg1.cash_balance);
    // trader_0
    //     .place_order(
    //         ctx,
    //         &product_1,
    //         Side::Bid,
    //         Fractional::from_str("1000").unwrap(),
    //         Fractional::from_str("0.0000000001").unwrap(),
    //     )
    //     .await
    //     .unwrap();
    // trader_1
    //     .place_order(
    //         ctx,
    //         &product_1,
    //         Side::Ask,
    //         Fractional::from_str("0.1").unwrap(),
    //         Fractional::from_str("0.0000000001").unwrap(),
    //     )
    //     .await
    //     .unwrap();
    // trader_1
    //     .crank(ctx, &product_1, &[&trader_0])
    //     .await
    //     .unwrap();
}
