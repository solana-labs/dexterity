#![allow(non_snake_case)]
use agnostic_orderbook::state::Side;
use anchor_lang::Key;
use dex::utils::numeric::{Fractional, ZERO_FRAC};
use dexteritysdk::bootstrap::setup_combo;
use itertools::Itertools;

mod setup;
use crate::setup::bootstrap_tests;

pub const COLLATERAL: i64 = 1_000_000;

#[tokio::test]
async fn test_combo_orders() {
    let (ctx, traders) =
        &mut bootstrap_tests("noop_risk_engine", "constant_fees", "test", 2, 2).await;
    let combo = setup_combo(
        ctx,
        ctx.products
            .iter()
            .map(|p| p.key())
            .sorted()
            .collect::<Vec<_>>()
            .as_slice(),
        0,
    )
    .await
    .unwrap();

    let trader_0 = traders[0].clone();
    let trader_1 = traders[1].clone();
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

    trader_0
        .place_combo_order(
            ctx,
            &combo,
            Side::Ask,
            Fractional::new(1000, 1),
            Fractional::new(2, 1),
        )
        .await
        .unwrap();

    trader_0
        .place_combo_order(ctx, &combo, Side::Ask, Fractional::new(1000, 1), 0)
        .await
        .unwrap();

    trader_0
        .place_combo_order(
            ctx,
            &combo,
            Side::Ask,
            Fractional::new(1000, 1),
            Fractional::new(-1, 1),
        )
        .await
        .unwrap();

    trader_1
        .place_combo_order(
            ctx,
            &combo,
            Side::Bid,
            Fractional::new(2000, 1),
            Fractional::new(2, 1),
        )
        .await
        .unwrap();

    let trader_risk_group_1 = trader_1.get_trader_risk_group(&ctx.client).await;
    let combo_group_data = ctx.get_market_product_group().await;
    let (_, combo_0) = combo_group_data.active_combos().next().unwrap();
    assert_eq!(
        trader_risk_group_1.pending_cash_balance,
        Fractional::new(1, 1) * Fractional::new(1000, 1)
    );
    for leg in combo_0.legs.iter().take(combo_0.num_legs) {
        let i = leg.product_index;
        let t_i = trader_risk_group_1.active_products[i] as usize;
        let position = trader_risk_group_1.trader_positions[t_i];
        let expected_pos = Fractional::new(leg.ratio, 0) * Fractional::new(2000, 1);
        assert_eq!(position.pending_position, expected_pos);
    }

    trader_1
        .place_combo_order(
            ctx,
            &combo,
            Side::Bid,
            Fractional::new(1000, 1),
            Fractional::new(2, 1),
        )
        .await
        .unwrap();

    let trader_risk_group_1 = trader_1.get_trader_risk_group(&ctx.client).await;
    let combo_group_data = ctx.get_market_product_group().await;
    let (_, combo_0) = combo_group_data.active_combos().next().unwrap();
    assert_eq!(
        trader_risk_group_1.pending_cash_balance,
        (Fractional::new(1, 1) - Fractional::new(2, 1)) * Fractional::new(1000, 1)
    );
    for leg in combo_0.legs.iter().take(combo_0.num_legs) {
        let i = leg.product_index;
        let t_i = trader_risk_group_1.active_products[i] as usize;
        let position = trader_risk_group_1.trader_positions[t_i];
        let expected_pos = Fractional::new(leg.ratio, 0) * Fractional::new(3000, 1);
        assert_eq!(position.pending_position, expected_pos);
    }

    let mut traders = vec![
        trader_0.account,
        trader_0.fee_acct,
        trader_0.risk_state_account,
        trader_1.account,
        trader_1.fee_acct,
        trader_1.risk_state_account,
    ];
    traders.sort_by(|a, b| a.as_ref().cmp(b.as_ref()));
    ctx.crank_raw(
        combo.key,
        combo.market_signer,
        combo.orderbook,
        combo.event_queue,
        &trader_1.keypair,
        traders.as_mut_slice(),
        10,
    )
    .await
    .unwrap();

    let trader_risk_group_0 = trader_0.get_trader_risk_group(&ctx.client).await;
    let trader_risk_group_1 = trader_1.get_trader_risk_group(&ctx.client).await;
    let combo_group_data = ctx.get_market_product_group().await;
    let (_, combo_0) = combo_group_data.active_combos().next().unwrap();
    let expected_balance =
        (Fractional::new(1, 1) - Fractional::new(2, 1)) * Fractional::new(1000, 1);
    assert_eq!(trader_risk_group_0.cash_balance, -expected_balance);
    assert_eq!(trader_risk_group_1.cash_balance, expected_balance);
    assert_eq!(trader_risk_group_0.pending_cash_balance, ZERO_FRAC);
    assert_eq!(trader_risk_group_1.pending_cash_balance, ZERO_FRAC);
    for leg in combo_0.legs().iter() {
        let i = leg.product_index;
        let t_i = trader_risk_group_0.active_products[i] as usize;
        let position_0 = trader_risk_group_0.trader_positions[t_i];
        let i = leg.product_index;
        let t_i = trader_risk_group_1.active_products[i] as usize;
        let position_1 = trader_risk_group_1.trader_positions[t_i];
        let expected_pos = Fractional::new(leg.ratio, 0) * Fractional::new(3000, 1);
        assert_eq!(position_0.position, -expected_pos);
        assert_eq!(position_1.position, expected_pos);
        assert_eq!(position_0.pending_position, ZERO_FRAC);
        assert_eq!(position_1.pending_position, ZERO_FRAC);
    }
}
