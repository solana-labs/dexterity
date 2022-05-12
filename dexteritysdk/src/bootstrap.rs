use std::{
    cell::RefCell,
    sync::{Arc, RwLock},
};

use anchor_client::Cluster;
use anchor_lang::Key;
use anchor_spl::token::accessor::authority;
use arrayvec::ArrayVec;
use pyth_client::{
    load_mapping, load_price, load_product, CorpAction, PriceStatus, PriceType, Product, PythError,
    PROD_HDR_SIZE,
};
use solana_program::{
    program_pack::Pack, pubkey::Pubkey, system_instruction::create_account, system_program, sysvar,
    sysvar::clock::Clock,
};
use solana_program_test::{ProgramTest, ProgramTestContext};
use solana_sdk::{
    account::{Account, ReadableAccount},
    client::{Client, SyncClient},
    commitment_config::CommitmentConfig,
    genesis_config::ClusterType,
    signature::{Keypair, Signer},
};
use std::str::{FromStr, Utf8Error};

use crate::{
    admin::DexAdmin,
    common::{utils::*, KeypairD},
    instrument::{initialize_derivative, InstrumentAdmin},
    oracle::{create_clock::*, create_oracle::*, update_clock::*, update_oracle::*},
    processor::{
        combo::initialize_combo_ixs, market_product::*, market_product_group::*, orderbook::*,
        trader_risk_group::*,
    },
    sdk_client::{ClientSubset, SDKClient},
    state::*,
    trader::SDKTrader,
    BootstrapConfig, OptionalBootstrapFields, RiskEngines, SDKContext, FIND_FEES_DISCRIMINANT,
    MINT_DECIMALS,
};
use constant_fees::initialize_trader_fee_acct_ix;
use dex::{
    state::{constants::*, enums::*, market_product_group::*, trader_risk_group::*},
    utils::numeric::{Fractional, ZERO_FRAC},
};
use instruments::state::enums::{InstrumentType, OracleType};

// TODO: Refactor this out.
#[derive(Clone, Copy)]
pub struct PythAttrIter<'a> {
    data: &'a [u8],
}

impl<'a> Iterator for PythAttrIter<'a> {
    type Item = (&'a str, &'a str);

    fn next(&mut self) -> Option<Self::Item> {
        if self.data.is_empty() {
            return None;
        }
        let (key, data) = get_attr_str(self.data);
        let (val, data) = get_attr_str(data);
        self.data = data;
        Some((key, val))
    }
}

// example usage of pyth-client account structure
// bootstrap all product and pricing accounts from root mapping account

fn get_attr_str(buf: &[u8]) -> (&str, &[u8]) {
    if buf.is_empty() {
        return ("", &[]);
    }
    let len = buf[0] as usize;
    let str = std::str::from_utf8(&buf[1..len + 1]).expect("attr should be ascii or utf-8");
    let remaining_buf = &buf[len + 1..];
    (str, remaining_buf)
}

pub async fn ensure_mint(config: &mut BootstrapConfig, client: &SDKClient) -> SDKResult<KeypairD> {
    // todo include mint_decimals in config
    Ok(
        match (&config.optional.mint, &config.optional.mint_authority) {
            (Some(mint), Some(mint_authority)) => mint.clone(),
            (Some(mint), None) => mint.clone(),
            (None, Some(mint_authority)) => {
                let mint = KeypairD::from(Keypair::new());
                create_mint2(&client, &mint, &mint_authority, MINT_DECIMALS).await?;
                config.optional.mint = Some(mint.clone());
                mint
            }
            (None, None) => {
                let mint = KeypairD::from(Keypair::new());
                create_mint2(&client, &mint, &config.payer, MINT_DECIMALS).await?;
                config.optional.mint_authority = Some(config.payer.clone());
                config.optional.mint = Some(mint.clone());
                mint
            }
        },
    )
}

pub async fn bootstrap_mpg(config: &mut BootstrapConfig, client: SDKClient) -> SDKResult<DexAdmin> {
    let mint = ensure_mint(config, &client).await?;
    let config = config.clone();
    let fee_collector = config
        .optional
        .fee_collector_and_wallet
        .unwrap_or_else(|| (config.payer.clone(), Pubkey::new_unique()))
        .0;

    let market_product_group_keypair = KeypairD::new();
    let risk_output_register_keypair = KeypairD::new();
    let fee_output_register_keypair = KeypairD::new();
    let risk_model_config_acct = KeypairD::new();
    let (fee_model_config_acct, _) = Pubkey::find_program_address(
        &[
            b"fee_model_config_acct",
            market_product_group_keypair.pubkey().as_ref(),
        ],
        &config.fee_model_program_id,
    );
    let (vault, vault_bump) = Pubkey::find_program_address(
        &[
            b"market_vault",
            market_product_group_keypair.pubkey().as_ref(),
        ],
        &config.dex_program_id,
    );

    // init fee_collector_wallet
    let fee_collector_ata = create_token_account(&client, &mint.pubkey(), &fee_collector.pubkey())
        .await
        .unwrap();

    create_fee_program_config_acct(
        &client,
        &fee_model_config_acct,
        &config.fee_model_program_id,
        market_product_group_keypair.pubkey(),
    )
    .await?;

    let mut product_group_name: [u8; NAME_LEN] = [0_u8; NAME_LEN];
    product_group_name
        .clone_from_slice(format!("{:width$}", config.group_name, width = NAME_LEN).as_bytes());

    initialize_market_product_group(
        &client,
        &market_product_group_keypair,
        &risk_output_register_keypair,
        &fee_output_register_keypair,
        config.dex_program_id,
        config.optional.mint.as_ref().unwrap().pubkey(),
        vault,
        &config.payer,
        fee_collector.pubkey(),
        config.fee_model_program_id,
        fee_model_config_acct,
        risk_model_config_acct.pubkey(),
        config.risk_engine_program_id,
        product_group_name,
        config.risk_disc_len,
        config.fees_disc_len,
        config.health_disc.to_le_bytes(),
        config.liq_disc.to_le_bytes(),
        config.create_risk_state_disc.to_le_bytes(),
        config.fees_disc.to_le_bytes(),
    )
    .await
    .unwrap();

    let trader_risk_state_account_len = match config.risk_engine_name {
        RiskEngines::Other(name) => {
            println!(
                "Initializing risk engine \"{}\" must be done manually.",
                &name
            );
            0
        }
        RiskEngines::NOOP | RiskEngines::ALPHA => 0,
    };

    // validation
    let market_product_group = client
        .get_anchor_account::<MarketProductGroup>(market_product_group_keypair.pubkey())
        .await;

    assert_eq!(market_product_group.tag, AccountTag::MarketProductGroup);
    assert_eq!(market_product_group.authority, config.payer.pubkey());
    assert_eq!(
        market_product_group.vault_mint,
        config.optional.mint.unwrap().pubkey()
    );
    assert_eq!(market_product_group.vault_bump, vault_bump as u16);
    assert_eq!(
        market_product_group.fee_model_program_id,
        config.fee_model_program_id
    );
    assert_eq!(market_product_group.fee_collector, fee_collector.pubkey());
    assert_eq!(
        market_product_group.fee_model_configuration_acct,
        fee_model_config_acct,
    );
    assert_eq!(market_product_group.collected_fees, ZERO_FRAC);
    assert_eq!(market_product_group.decimals, 6);
    assert_eq!(
        market_product_group.risk_engine_program_id,
        config.risk_engine_program_id
    );
    assert_eq!(market_product_group.active_flags_products.inner, [0, 0]);

    Ok(DexAdmin::new(
        SDKContext {
            client: client.clone(),
            dex_program_id: config.dex_program_id,
            aaob_program_id: config.aaob_program_id,
            risk_engine_program_id: config.risk_engine_program_id,
            instruments_program_id: config.instruments_program_id,
            dummy_oracle_program_id: config.dummy_oracle_program_id,
            fee_model_program_id: config.fee_model_program_id,
            fee_model_config_acct,
            vault,
            vault_mint: mint.pubkey(),
            market_product_group: market_product_group_keypair.pubkey(),
            product_group_name,
            payer: config.payer.clone(),
            products: Vec::new(),
            combo_products: Vec::new(),
            out_register_risk_info: risk_output_register_keypair.pubkey(),
            fee_output_register: fee_output_register_keypair.pubkey(),
            fee_collector: fee_collector.pubkey(),
            additional_risk_accts: ArrayVec::new(),
            risk_model_config_acct: risk_model_config_acct.pubkey(),
            trader_risk_state_account_len,
        },
        config.payer,
        fee_collector,
        fee_collector_ata,
    ))
}

pub async fn ensure_fee_model() -> SDKResult<()> {
    Ok(())
}

pub async fn bootstrap_full(
    group_name: &str,
    n_products: u32,
    n_traders: u32,
    client: SDKClient,
    mut config: BootstrapConfig,
) -> std::result::Result<(DexAdmin, Vec<SDKTrader>), SDKError> {
    let mut ctx = bootstrap_mpg(&mut config, client).await?;
    let client = &ctx.client;

    let mut products: Vec<SDKProduct> = vec![];
    let payer = config.payer.clone();
    let authority = ctx.authority.clone();
    let instrument_admin = InstrumentAdmin::new(
        client.clone(),
        authority,
        ctx.market_product_group.clone(),
        payer,
    );
    
    let url = config.url.clone();
    let (clock, oracle) = match url {
        Some(cluster) => {
            match cluster {
                Cluster::Localnet => {
                    default_clock_and_oracle(&client, &ctx, &instrument_admin).await
                }
                // TODO: Add instrument type to config.
                Cluster::Mainnet => get_pyth_oracle(Cluster::Mainnet, "BTCUSD", &client).await,
                Cluster::Devnet => get_pyth_oracle(Cluster::Devnet, "BTCUSD", &client).await,
                _ => panic!("Not a valid cluster"),
            }
        }

        None => default_clock_and_oracle(&client, &ctx, &instrument_admin).await,
    };
    println!("clock {} oracle {}", clock, oracle);

    for i in 0..n_products {
        let product_pubkey = if clock == sysvar::clock::id() {
            instrument_admin
                .initialize_derivative(
                    oracle,
                    clock,
                    0 as i64,
                    initialize_derivative::InitializeDerivativeOptionalArgs::new(
                        InstrumentType::RecurringCall,
                        get_curr_time(&client).await + 10,
                        3600,
                        30,
                        OracleType::Pyth,
                    ),
                )
                .await
                .unwrap()
        } else {
            instrument_admin
                .initialize_derivative(
                    oracle,
                    clock,
                    // TODO: fix strike price
                    i as i64,
                    initialize_derivative::InitializeDerivativeOptionalArgs::default(),
                )
                .await
                .unwrap()
        };

        // log_disable();
        let (market_signer, _) =
            Pubkey::find_program_address(&[product_pubkey.as_ref()], &config.dex_program_id);

        println!("Creating orderbook {} ({})", i, config.aaob_program_id);
        let (orderbook_key, bids_key, asks_key, eq_key) =
            create_orderbook(&ctx.client, config.aaob_program_id, market_signer)
                .await
                .unwrap();
        let name_str = format!("product{:width$}", i, width = NAME_LEN - 7);
        let mut name: [u8; NAME_LEN] = Default::default();
        name.clone_from_slice(name_str.as_bytes());
        ctx.initialize_market_product(
            product_pubkey,
            orderbook_key,
            name,
            Fractional::new(100, 4),
            7,
            0,
        )
        .await
        .unwrap();
        products.push(SDKProduct {
            name,
            key: product_pubkey,
            orderbook: orderbook_key,
            bids: bids_key,
            asks: asks_key,
            event_queue: eq_key,
            market_signer,
        })
    }

    let mut traders: Vec<SDKTrader> = Vec::with_capacity(n_traders as usize);
    for trader in (0..n_traders).map(|_| KeypairD::new()) {
        // trader account (not trader risk group)
        ctx.client
            .sign_send_instructions(
                vec![create_account(
                    &ctx.payer.pubkey(),
                    &trader.pubkey(),
                    100_000_000,
                    0,
                    &system_program::id(),
                )],
                vec![&trader, &ctx.payer],
            )
            .await?;
        let sdk_trader = ctx.register_trader(trader).await.unwrap();

        mint_to(
            &ctx.client,
            &ctx.vault_mint,
            &sdk_trader.wallet,
            &config.optional.mint_authority.as_ref().unwrap(),
            NUM_TOKENS,
        )
        .await
        .unwrap();

        // assertions
        let account = ctx.client.get_account(sdk_trader.wallet).await?;
        let token_account =
            spl_token::state::Account::unpack_unchecked(account.data.as_slice()).unwrap();
        assert_eq!(token_account.owner, sdk_trader.key());

        let trader_risk_group = sdk_trader.get_trader_risk_group(&ctx.client).await;
        assert_eq!(trader_risk_group.tag, AccountTag::TraderRiskGroup);
        assert_eq!(
            trader_risk_group.market_product_group,
            ctx.market_product_group,
        );
        assert_eq!(trader_risk_group.owner, sdk_trader.key());
        traders.push(sdk_trader);
    }

    ctx.as_mut().load_products().await?;
    Ok((ctx, traders))
}

pub async fn setup_combo(
    admin_ctx: &DexAdmin,
    products: &[Pubkey],
    product_index: u8,
) -> SDKResult<SDKCombo> {
    let mut seeds: Vec<u8> = Vec::new();

    let ratios: Vec<i8> = vec![1, -1];
    let mut combo_legs = ratios
        .into_iter()
        .zip(products)
        .collect::<Vec<(i8, &Pubkey)>>();
    combo_legs.sort_by(|(_, a), (_, b)| a.as_ref().cmp(b.as_ref()));

    for (i, (ratio, &product)) in combo_legs.iter().enumerate() {
        // legs[i] = Leg {
        //     product_index: i,
        //     product_key: product,
        //     ratio: *ratio as i64,
        // };
        seeds.extend(product.to_bytes().iter());
    }
    for (ratio, _) in combo_legs.iter() {
        seeds.extend(ratio.to_le_bytes().iter());
    }

    // Format of the seeds is [product_key_1, ..., product_key_N, [ratio_1, ..., ratio_N]]
    let (product_key, _bump) = Pubkey::find_program_address(
        &seeds.chunks(32).collect::<Vec<&[u8]>>(),
        &admin_ctx.dex_program_id,
    );
    let (market_signer, _bump) =
        Pubkey::find_program_address(&[product_key.as_ref()], &admin_ctx.dex_program_id);

    let (orderbook_key, bids_key, asks_key, eq_key) =
        create_orderbook(&admin_ctx.client, admin_ctx.aaob_program_id, market_signer)
            .await
            .unwrap();

    let ratios = combo_legs.iter().map(|(i, _)| *i).collect::<Vec<i8>>();
    let products = combo_legs
        .iter()
        .map(|(_, p)| *p)
        .copied()
        .collect::<Vec<Pubkey>>();

    let name_str = format!("combo{:width$}", product_index, width = NAME_LEN - 5);
    let mut name: [u8; NAME_LEN] = Default::default();
    name.clone_from_slice(name_str.as_bytes());

    admin_ctx
        .initialize_combo(
            orderbook_key,
            name,
            &products,
            Fractional::new(100, 4),
            Fractional::from(2_i64.pow(16)),
            4,
            ratios,
        )
        .await?;
    Ok(SDKCombo {
        key: product_key,
        name,
        orderbook: orderbook_key,
        bids: bids_key,
        asks: asks_key,
        market_signer,
        event_queue: eq_key,
    })
}

pub async fn default_clock_and_oracle(
    client: &SDKClient,
    ctx: &DexAdmin,
    instrument_admin: &InstrumentAdmin,
) -> (Pubkey, Pubkey) {
    let clock = create_clock_account(
        client,
        ctx.dummy_oracle_program_id,
        &instrument_admin.payer,
        solana_program::system_program::id(),
    )
    .await
    .unwrap();

    let oracle = create_oracle_price_account(
        client,
        ctx.dummy_oracle_program_id,
        &instrument_admin.payer,
        solana_program::system_program::id(),
        CreateOracleOptionalArgs::default(),
    )
    .await
    .unwrap();

    (clock, oracle)
}

pub async fn get_pyth_oracle(
    cluster: Cluster,
    instrument: &str,
    client: &SDKClient,
) -> (Pubkey, Pubkey) {
    let map_account = match cluster {
        Cluster::Mainnet => client
            .get_account(Pubkey::from_str("AHtgzX45WTKfkPG53L6WYhGEXwQkN1BVknET3sVsLL8J").unwrap())
            .await
            .unwrap(),
        Cluster::Devnet => client
            .get_account(Pubkey::from_str("BmA9Z6FjioHJPpjT39QazZyhDRUdZy2ezwx4GiDdE2u2").unwrap())
            .await
            .unwrap(),
        _ => unreachable!(),
    };

    let map_acct = load_mapping(&map_account.data).unwrap();

    let mut oracle_key = None;

    for (i, p) in map_acct
        .products
        .iter()
        .take(map_acct.num as usize)
        .enumerate()
    {
        let acct = client
            .get_account(Pubkey::new_from_array(p.val))
            .await
            .unwrap();
        let prod = match load_product(&acct.data) {
            Ok(p) => p,
            Err(e) => {
                println!("Error loading product {} account: {:?}", i, e);
                continue;
            }
        };
        let mut attr_iterator = PythAttrIter { data: &prod.attr };
        oracle_key = attr_iterator.find_map(|(key, val)| {
            if key == "generic_symbol" && val == instrument {
                Some(Pubkey::new_from_array(prod.px_acc.val))
            } else {
                None
            }
        });

        if oracle_key.is_some() {
            break;
        }
    }
    match oracle_key {
        Some(k) => (sysvar::clock::id(), k),
        None => {
            panic!("Instrument not found in Pyth")
        }
    }
}

pub async fn get_curr_time(client: &SDKClient) -> i64 {
    let clock_data = client.get_account(sysvar::clock::id()).await.unwrap();
    let clock: Clock = bincode::deserialize(&clock_data.data).unwrap();

    clock.unix_timestamp
}
