#![allow(non_snake_case)]

use agnostic_orderbook::state::Side;
use anchor_lang::solana_program::program_pack::Pack;
use dexteritysdk::{common::utils::*, MINT_DECIMALS};
use solana_sdk::account::ReadableAccount;

use dex::utils::numeric::{bps, Fractional};

use crate::setup::bootstrap_tests;

mod setup;

#[tokio::test]
async fn test_sweep_fees() -> SDKResult {
    let (ctx, traders) =
        &mut bootstrap_tests("noop_risk_engine", "constant_fees", "test", 2, 1).await;
    let maker = &traders[0].clone();
    let taker = &traders[1].clone();
    let product = &ctx.products[0].clone();

    let maker_fee = bps(100);
    let taker_fee = bps(200);
    let deposit = 1000;
    let size = 10;
    let fill_price = 10;
    let quote = size * fill_price; // 100

    let maker_fees_paid = quote * maker_fee;
    let taker_fees_paid = quote * taker_fee;

    let maker_cash_balance = deposit - quote + (-maker_fees_paid); // buyer
    let taker_cash_balance = deposit + quote + (-taker_fees_paid); // seller

    let total_fees_collected = maker_fees_paid + taker_fees_paid;

    ctx.update_fees(100, 200).await.unwrap();

    maker.deposit(ctx, 1000).await.unwrap();
    taker.deposit(ctx, 1000).await.unwrap();
    // check vault has been funded
    {
        let vault_wallet = spl_token::state::Account::unpack(
            ctx.client.get_account(ctx.vault).await.unwrap().data(),
        )
        .unwrap();
        let vault_amount_ui = spl_token::amount_to_ui_amount(vault_wallet.amount, MINT_DECIMALS);
        assert_eq_frac(
            Fractional::new(vault_wallet.amount as i64, MINT_DECIMALS as u64),
            2000,
        );
        assert_eq!(vault_amount_ui as i64, deposit * 2);
    }

    maker
        .place_order(ctx, product, Side::Bid, size, fill_price)
        .await
        .unwrap();
    taker
        .place_order(ctx, product, Side::Ask, size, 1)
        .await
        .unwrap();
    {
        // maker has no pending fees
        let trg_maker = maker.get_trader_risk_group(&ctx.client).await;
        assert_eq_frac(trg_maker.pending_fees, 0);
        assert_eq_frac(trg_maker.total_deposited, deposit);
        assert_eq_frac(trg_maker.cash_balance, deposit);

        // taker has pending fees
        let trg_taker = taker.get_trader_risk_group(&ctx.client).await;
        assert_eq_frac(trg_taker.pending_fees, size * fill_price * taker_fee);
        assert_eq_frac(trg_taker.total_deposited, deposit);
        assert_eq_frac(trg_taker.cash_balance, deposit);
    }
    taker.crank(ctx, product, &[maker]).await.unwrap();
    {
        let trg_maker = maker.get_trader_risk_group(&ctx.client).await;
        assert_eq_frac(trg_maker.pending_fees, 0);
        assert_eq_frac(trg_maker.cash_balance, maker_cash_balance);

        let trg_taker = taker.get_trader_risk_group(&ctx.client).await;
        assert_eq_frac(trg_taker.pending_fees, 0);
        assert_eq_frac(trg_taker.cash_balance, taker_cash_balance);

        let mpg = ctx.get_market_product_group().await;
        assert_eq_frac(mpg.collected_fees, total_fees_collected);
    }
    ctx.sweep_fees().await.unwrap();
    // check vault has been debited and fee_wallet credited
    {
        let vault_wallet = spl_token::state::Account::unpack(
            ctx.client.get_account(ctx.vault).await.unwrap().data(),
        )
        .unwrap();
        assert_eq_frac(
            Fractional::new(vault_wallet.amount as i64, MINT_DECIMALS as u64),
            deposit * 2 + (-total_fees_collected),
        );

        let fee_wallet = spl_token::state::Account::unpack(
            ctx.client
                .get_account(ctx.fee_collector_wallet)
                .await
                .unwrap()
                .data(),
        )
        .unwrap();
        assert_eq_frac(
            Fractional::new(fee_wallet.amount as i64, MINT_DECIMALS as u64),
            total_fees_collected,
        );
    }
    Ok(())
}
