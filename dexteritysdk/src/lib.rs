#![allow(unused_imports, unused_variables)]
#![cfg(not(target_arch = "bpf"))]

use std::{
    cell::RefCell,
    ops::Deref,
    rc::Rc,
    sync::{Arc, RwLock},
};

use agnostic_orderbook::state::MarketState;
use anchor_client::{Client as AnchorClient, Cluster, Cluster::Localnet};
use anchor_lang::Key;
use anyhow::anyhow;
use arrayvec::ArrayVec;
use solana_program::{
    account_info::AccountInfo, clock::UnixTimestamp, program_pack::Pack, pubkey::Pubkey,
    system_instruction::create_account, system_program,
};
use solana_program_test::{ProgramTest, ProgramTestContext};
use solana_sdk::{
    account::Account,
    client::{Client, SyncClient},
    commitment_config::CommitmentConfig,
    signature::{read_keypair_file, Keypair, Signer},
};

use constant_fees::initialize_trader_fee_acct_ix;
use dex::{
    state::{constants::*, enums::*, market_product_group::*, trader_risk_group::*},
    utils::numeric::{Fractional, ZERO_FRAC},
};

pub use crate::bootstrap::bootstrap_full;
use crate::{
    common::{utils::*, KeypairD},
    processor::{market_product::*, market_product_group::*, orderbook::*, trader_risk_group::*},
    sdk_client::{ClientSubset, SDKClient},
    state::*,
    trader::SDKTrader,
};

pub mod admin;
pub mod bootstrap;
pub mod common;
pub mod context;
pub mod instrument;
pub mod oracle;
pub mod processor;
pub mod sdk_client;
pub mod state;
pub mod trader;

use serde::{Deserialize, Serialize};

pub const ANCHOR_VALIDATE_ACCOUNT_HEALTH_DISCRIMINANT: u64 = 16754576316527260711;
pub const ANCHOR_VALIDATE_ACCOUNT_LIQUIDATION_DISCRIMINANT: u64 = 13444787341969615152;
pub const ANCHOR_CREATE_RISK_STATE_ACCOUNT_DISCRIMINANT: u64 = 565056906074257608;
pub const VALIDATE_ACCOUNT_HEALTH_DISCRIMINANT: u64 = 0;
pub const VALIDATE_ACCOUNT_LIQUIDATION_DISCRIMINANT: u64 = 1;
pub const FIND_FEES_DISCRIMINANT: u8 = 0;
pub const MINT_DECIMALS: u8 = 6;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum RiskEngines {
    NOOP,
    ALPHA,
    Other(String),
}

#[derive(Serialize, Deserialize)]
pub struct ConnectConfig {
    pub url: Cluster,
    pub payer: KeypairD,
    pub dex_program_id: Pubkey,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BootstrapConfig {
    pub url: Option<Cluster>,
    pub group_name: String,
    pub risk_engine_name: RiskEngines,
    pub payer: KeypairD,

    pub fee_model_program_id: Pubkey,
    pub risk_engine_program_id: Pubkey,
    pub dex_program_id: Pubkey,
    pub instruments_program_id: Pubkey,
    pub dummy_oracle_program_id: Pubkey,
    pub aaob_program_id: Pubkey,

    pub risk_disc_len: u64,
    pub fees_disc_len: u64,
    pub health_disc: u64,
    pub create_risk_state_disc: u64,
    pub liq_disc: u64,
    pub fees_disc: u64,

    pub optional: OptionalBootstrapFields,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OptionalBootstrapFields {
    pub mint_authority: Option<KeypairD>,
    pub mint: Option<KeypairD>,
    pub fee_collector_and_wallet: Option<(KeypairD, Pubkey)>,
}

pub struct SDKContext {
    pub client: SDKClient,
    pub product_group_name: [u8; NAME_LEN],
    pub trader_risk_state_account_len: usize,
    // cached products, reload if necessary
    pub products: Vec<SDKProduct>,
    pub combo_products: Vec<SDKCombo>,
    // program_ids
    pub dex_program_id: Pubkey,
    pub aaob_program_id: Pubkey,
    pub risk_engine_program_id: Pubkey,
    pub instruments_program_id: Pubkey,
    pub dummy_oracle_program_id: Pubkey,
    pub fee_model_program_id: Pubkey,
    // accts
    pub market_product_group: Pubkey,
    pub payer: KeypairD,
    pub vault: Pubkey,
    pub vault_mint: Pubkey,
    pub fee_model_config_acct: Pubkey,
    pub risk_model_config_acct: Pubkey,
    pub out_register_risk_info: Pubkey,
    pub fee_output_register: Pubkey,
    pub fee_collector: Pubkey,
    pub additional_risk_accts: ArrayVec<Pubkey, 4>, // todo clean this up and integrate w/ dex
}

impl SDKContext {
    /// Initialize the SDK against a running dexterity instance
    pub async fn connect(
        url: Cluster,
        payer: impl Into<KeypairD>,
        dex_program_id: Pubkey,
        aaob_program_id: Pubkey,
        instruments_program_id: Pubkey,
        dummy_oracle_program_id: Pubkey,
        market_product_group_key: Pubkey,
        trader_risk_state_account_len: usize,
    ) -> SDKResult<SDKContext> {
        let payer = payer.into();
        let client = SDKClient::from_rpc(
            AnchorClient::new_with_options(
                url,
                Rc::new(clone_keypair(&payer)),
                CommitmentConfig::processed(),
            )
            .program(dex_program_id)
            .rpc(),
            &payer,
        )?;
        let mpg = client
            .get_anchor_account::<MarketProductGroup>(market_product_group_key)
            .await;
        let (vault, _) = Pubkey::find_program_address(
            &[b"market_vault", market_product_group_key.as_ref()],
            &dex_program_id,
        );

        let mut ctx = SDKContext {
            client,
            dex_program_id,
            aaob_program_id,
            risk_engine_program_id: mpg.risk_engine_program_id,
            instruments_program_id,
            dummy_oracle_program_id,
            fee_model_program_id: mpg.fee_model_program_id,
            payer,
            fee_model_config_acct: mpg.fee_model_configuration_acct,
            vault,
            vault_mint: mpg.vault_mint,
            product_group_name: mpg.name,
            market_product_group: market_product_group_key,
            products: Vec::new(),
            combo_products: Vec::new(),
            out_register_risk_info: mpg.risk_output_register,
            fee_output_register: mpg.fee_output_register,
            fee_collector: mpg.fee_collector,
            additional_risk_accts: ArrayVec::new(),
            risk_model_config_acct: mpg.risk_model_configuration_acct,
            trader_risk_state_account_len,
        };
        ctx.load_products().await?;
        Ok(ctx)
    }

    pub async fn load_products(&mut self) -> SDKResult {
        let mpg = self.get_market_product_group().await;
        let mut products: Vec<SDKProduct> = Vec::with_capacity(mpg.market_products.len());
        for (_, product) in mpg.active_outrights() {
            let market_state = load_order_book(product.orderbook, &self.client).await?;
            let (market_signer, _) =
                Pubkey::find_program_address(&[product.product_key.as_ref()], &self.dex_program_id);

            products.push(SDKProduct {
                key: product.product_key,
                name: product.name,
                orderbook: product.orderbook,
                bids: Pubkey::new_from_array(market_state.bids),
                asks: Pubkey::new_from_array(market_state.asks),
                market_signer,
                event_queue: Pubkey::new_from_array(market_state.event_queue),
            });
        }

        let mut combo_products: Vec<SDKCombo> = Vec::with_capacity(mpg.active_combos().count());
        for (_, product) in mpg.active_combos() {
            let market_state = load_order_book(product.orderbook, &self.client).await?;
            let (market_signer, _) =
                Pubkey::find_program_address(&[product.product_key.as_ref()], &self.dex_program_id);

            combo_products.push(SDKCombo {
                key: product.product_key,
                name: product.name,
                orderbook: product.orderbook,
                bids: Pubkey::new_from_array(market_state.bids),
                asks: Pubkey::new_from_array(market_state.asks),
                market_signer,
                event_queue: Pubkey::new_from_array(market_state.event_queue),
            });
        }
        self.products = products;
        self.combo_products = combo_products;
        Ok(())
    }

    pub async fn get_market_product_group(&self) -> Box<MarketProductGroup> {
        self.client
            .get_anchor_account(self.market_product_group)
            .await
    }
}

async fn load_order_book(orderbook: Pubkey, client: &SDKClient) -> SDKResult<MarketState> {
    let acct = &mut (orderbook, client.get_account(orderbook).await?);
    let info = solana_sdk::account_info::IntoAccountInfo::into_account_info(acct);
    let market_state = MarketState::get(&info)?;
    Ok(*market_state)
}

impl Default for OptionalBootstrapFields {
    fn default() -> Self {
        Self {
            mint_authority: None,
            mint: None,
            fee_collector_and_wallet: None,
        }
    }
}
