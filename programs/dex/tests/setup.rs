use agnostic_orderbook::state::Side;
use dexteritysdk::{
    admin::DexAdmin,
    common::utils::{log_disable, *},
    sdk_client::SDKClient,
    trader::SDKTrader,
    BootstrapConfig, OptionalBootstrapFields, RiskEngines, SDKContext,
    ANCHOR_CREATE_RISK_STATE_ACCOUNT_DISCRIMINANT, ANCHOR_VALIDATE_ACCOUNT_HEALTH_DISCRIMINANT,
    ANCHOR_VALIDATE_ACCOUNT_LIQUIDATION_DISCRIMINANT, FIND_FEES_DISCRIMINANT,
};
use solana_program::{pubkey::Pubkey, system_program};
use solana_program_test::ProgramTest;
use solana_sdk::signature::{Keypair, Signer};

use dex::{state::constants::*, utils::numeric::*};

#[allow(dead_code)]
pub(crate) async fn set_prices(
    ctx: &SDKContext,
    trader: &SDKTrader,
    product_indices: Vec<usize>,
    prices: &Vec<Fractional>,
    cancel_orders: bool,
) -> SDKResult {
    assert_eq!(product_indices.len(), prices.len());
    assert!(prices.len() <= MAX_OUTRIGHTS);
    trader.deposit(ctx, 1_000_000).await.unwrap();
    let err = set_product_prices(ctx, trader, prices).await;
    assert!(err.is_ok());
    if cancel_orders {
        trader
            .cancel_all_orders(ctx, &product_indices)
            .await
            .unwrap();
    }

    Ok(())
}

#[allow(dead_code)]
async fn set_product_prices(
    ctx: &SDKContext,
    trader: &SDKTrader,
    prices: &Vec<Fractional>,
) -> SDKResult {
    assert!(prices.len() <= MAX_OUTRIGHTS);
    for (i, &price) in prices.iter().enumerate() {
        let offset = if price.m % 2 == 0 {
            price.m / 2
        } else {
            (price.m + 1) / 2
        };
        trader
            .place_order(
                ctx,
                &ctx.products[i],
                Side::Bid,
                Fractional::new(1, 0),
                Fractional::new(price.m - offset, price.exp),
            )
            .await?;

        trader
            .place_order(
                ctx,
                &ctx.products[i],
                Side::Ask,
                Fractional::new(1, 0),
                Fractional::new(price.m + offset, price.exp),
            )
            .await?;
    }
    Ok(())
}

pub(crate) fn load_test_config(risk_engine: &str) -> BootstrapConfig {
    let (name, risk_engine_program_id, health_disc, liq_disc, create_risk_state_disc, disc_len) =
        match risk_engine {
            "noop_risk_engine" => (
                RiskEngines::NOOP,
                noop_risk_engine::ID,
                ANCHOR_VALIDATE_ACCOUNT_HEALTH_DISCRIMINANT,
                ANCHOR_VALIDATE_ACCOUNT_LIQUIDATION_DISCRIMINANT,
                ANCHOR_CREATE_RISK_STATE_ACCOUNT_DISCRIMINANT,
                8,
            ),
            "alpha_risk_engine" => (
                RiskEngines::ALPHA,
                alpha_risk_engine::ID,
                ANCHOR_VALIDATE_ACCOUNT_HEALTH_DISCRIMINANT,
                ANCHOR_VALIDATE_ACCOUNT_LIQUIDATION_DISCRIMINANT,
                ANCHOR_CREATE_RISK_STATE_ACCOUNT_DISCRIMINANT,
                8,
            ),
            _ => panic!("unrecognized risk engine"),
        };

    BootstrapConfig {
        url: None,
        group_name: "my-group".to_string(),
        risk_engine_name: name,
        payer: Keypair::new().into(),
        fee_model_program_id: Pubkey::new_unique(),
        dex_program_id: dex::ID,
        aaob_program_id: agnostic_orderbook::id(),
        instruments_program_id: instruments::ID,
        dummy_oracle_program_id: Pubkey::new_unique(),
        health_disc,
        liq_disc,
        create_risk_state_disc,
        fees_disc: FIND_FEES_DISCRIMINANT as u64,
        risk_disc_len: disc_len as u64,
        risk_engine_program_id,
        fees_disc_len: 1,
        optional: OptionalBootstrapFields {
            mint_authority: Some(Keypair::new().into()),
            ..Default::default()
        },
    }
}

pub async fn bootstrap_tests(
    risk_engine: &str,
    fee_model: &str,
    group_name: &str,
    n_traders: u32,
    n_products: u32,
) -> (DexAdmin, Vec<SDKTrader>) {
    log_disable();
    let mut config = load_test_config(risk_engine);
    let mut program_test = ProgramTest::default();
    program_test.add_program("dex", config.dex_program_id, None);
    program_test.add_program("agnostic_orderbook", config.aaob_program_id, None);
    program_test.add_program(risk_engine, config.risk_engine_program_id, None);
    program_test.add_program(fee_model, config.fee_model_program_id, None);
    program_test.add_program("instruments", config.instruments_program_id, None);
    program_test.add_program("dummy_oracle", config.dummy_oracle_program_id, None);
    program_test.add_account(
        config.optional.mint_authority.as_ref().unwrap().pubkey(),
        solana_sdk::account::Account {
            lamports: 100_000_000_000,
            data: vec![],
            owner: system_program::id(),
            ..solana_sdk::account::Account::default()
        },
    );
    let prg_test_ctx = program_test.start_with_context().await;
    config.payer = (&prg_test_ctx.payer).into();
    let client = SDKClient::from_banks(&prg_test_ctx.banks_client, &prg_test_ctx.payer)
        .await
        .unwrap();
    dexteritysdk::bootstrap_full(group_name, n_products, n_traders, client, config)
        .await
        .unwrap()
}
