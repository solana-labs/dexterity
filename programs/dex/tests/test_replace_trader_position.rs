#![allow(non_snake_case)]
use agnostic_orderbook::state::Side;
use anchor_lang::Key;
use dex::state::{constants::*, enums::*};
use dexteritysdk::{bootstrap::setup_combo, common::utils::*, state::SDKProduct};

mod setup;
use crate::setup::bootstrap_tests;

#[tokio::test]
async fn test_replace_trader_position() -> SDKResult {
    let (ctx, traders) = &mut bootstrap_tests(
        "noop_risk_engine",
        "constant_fees",
        "test",
        1,
        MAX_TRADER_POSITIONS as u32 + 1,
    )
    .await;
    let combo = setup_combo(
        ctx,
        ctx.products[0..2]
            .iter()
            .map(SDKProduct::key)
            .collect::<Vec<_>>()
            .as_slice(),
        0,
    )
    .await?;

    let trader = traders[0].clone();
    let products = ctx.products.clone();

    // Assert positions Uninitialized to start
    {
        let trg = trader.get_trader_risk_group(&ctx.client).await;
        for pos in &trg.trader_positions {
            assert_eq!(pos.tag, AccountTag::Uninitialized);
        }
    }
    // Activate max positions
    for product in &products[..MAX_TRADER_POSITIONS] {
        trader.place_order(ctx, product, Side::Ask, 40, 100).await?;
    }
    // Assert all positions have now been initialized
    {
        let trg = trader.get_trader_risk_group(&ctx.client).await;
        for pos in &trg.trader_positions {
            assert_eq!(pos.tag, AccountTag::TraderPosition);
        }
    }
    // Place a combo that includes the first 2 products.
    trader
        .place_combo_order(ctx, &combo, Side::Ask, 10, 100)
        .await?;
    // Cancel an order and try to place new order for a MAX_TRADER_POSITIONS+1 product.
    // This should fail because there exists a combo with open orders where a leg corresponds to the product that was cancelled.
    // This means the trader position cannot be replaced and therefore there are not enough positions.
    {
        let trg = trader.get_trader_risk_group(&ctx.client).await;
        let first_order = trg.open_orders.orders[trg.open_orders.products[0].head_index].id;
        trader.cancel(ctx, &products[0], first_order).await?;

        let res = trader
            .place_order(ctx, &products[MAX_TRADER_POSITIONS], Side::Ask, 40, 100)
            .await;
        assert!(res.is_err(), "should fail");
    }
    // Cancel an order and place an order for a different product. This should replace the existing position since it is zero'd out (w/ no combos)
    {
        let trg = trader.get_trader_risk_group(&ctx.client).await;
        let fourth_order = trg.open_orders.orders[trg.open_orders.products[3].head_index].id;
        let new_status = trader.cancel(ctx, &products[3], fourth_order).await;
        assert!(new_status.is_ok());
        trader
            .place_order(ctx, &products[MAX_TRADER_POSITIONS], Side::Ask, 41, 100) // second tx must be different from failed one above
            .await
            .unwrap();
    }

    // Assert third position was replaced
    {
        let trg = trader.get_trader_risk_group(&ctx.client).await;
        assert_eq!(
            trg.trader_positions[3].product_key,
            products[MAX_TRADER_POSITIONS].key,
        );
    }

    Ok(())
}
