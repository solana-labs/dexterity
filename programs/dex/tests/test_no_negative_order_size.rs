#![allow(non_snake_case)]
use agnostic_orderbook::state::Side;
use dex::utils::numeric::Fractional;

mod setup;
use crate::setup::bootstrap_tests;

#[tokio::test]
async fn test_no_negative_order_size() {
    let (ctx, traders) =
        &mut bootstrap_tests("noop_risk_engine", "constant_fees", "test", 1, 1).await;
    let product_0 = ctx.products[0].clone();
    let trader_0 = traders[0].clone();
    let res = trader_0
        .place_order(
            ctx,
            &product_0,
            Side::Bid,
            Fractional::new(-1000, 1),
            Fractional::from(100),
        )
        .await;
    assert!(res.is_err());
}
