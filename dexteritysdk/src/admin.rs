use std::ops::Deref;

use agnostic_orderbook::state::Side;
use solana_program::pubkey::Pubkey;
use solana_sdk::{signature::Keypair, signer::Signer};

use crate::processor::remove_market_product::remove_market_product_ixs;
use dex::{state::constants::NAME_LEN, utils::numeric::Fractional};

use crate::{
    common::utils::SDKError,
    initialize_market_product_ixs,
    instrument::initialize_derivative,
    processor::{combo::initialize_combo_ixs, new_order::new_order_ixs},
    KeypairD, SDKClient, SDKContext, SDKResult, SDKTrader,
};

pub struct DexAdmin {
    pub ctx: SDKContext,
    pub authority: KeypairD,
    pub fee_collector: KeypairD,
    pub fee_collector_wallet: Pubkey,
}

impl DexAdmin {
    pub fn new(
        ctx: SDKContext,
        authority: KeypairD,
        fee_collector: KeypairD,
        fee_collector_wallet: Pubkey,
    ) -> Self {
        Self {
            ctx,
            authority,
            fee_collector,
            fee_collector_wallet,
        }
    }

    pub async fn initialize_market_product(
        &self,
        product: Pubkey,
        orderbook: Pubkey,
        name: [u8; NAME_LEN],
        tick_size: impl Into<Fractional>,
        base_decimals: u64,
        price_offset: impl Into<Fractional>,
    ) -> SDKResult {
        let ixs = initialize_market_product_ixs(
            self.authority.pubkey(),
            self.market_product_group,
            product,
            orderbook,
            name,
            tick_size.into(),
            base_decimals,
            price_offset.into(),
        );
        self.client
            .sign_send_instructions(ixs, vec![&self.authority])
            .await
    }

    pub async fn remove_market_product(
        &self,
        market_product_group: Pubkey,
        product: Pubkey,
        aaob_program_id: Pubkey,
        orderbook: Pubkey,
        market_signer: Pubkey,
        event_queue: Pubkey,
        bids: Pubkey,
        asks: Pubkey,
    ) -> SDKResult {
        let ixs = remove_market_product_ixs(
            self.authority.pubkey(),
            market_product_group,
            product,
            aaob_program_id,
            orderbook,
            market_signer,
            event_queue,
            bids,
            asks,
        );
        self.client
            .sign_send_instructions(ixs, vec![&self.authority])
            .await
    }

    pub async fn initialize_combo(
        &self,
        orderbook: Pubkey,
        name: [u8; NAME_LEN],
        products: &[Pubkey],
        tick_size: Fractional,
        price_offset: Fractional,
        base_decimals: u64,
        ratios: Vec<i8>,
    ) -> SDKResult {
        let ixs = initialize_combo_ixs(
            self.authority.pubkey(),
            self.market_product_group,
            orderbook,
            name,
            products,
            tick_size,
            price_offset,
            base_decimals,
            ratios,
        );
        self.client
            .sign_send_instructions(ixs, vec![&self.authority])
            .await
    }
}

impl Deref for DexAdmin {
    type Target = SDKContext;

    fn deref(&self) -> &Self::Target {
        &self.ctx
    }
}

impl AsMut<SDKContext> for DexAdmin {
    fn as_mut(&mut self) -> &mut SDKContext {
        &mut self.ctx
    }
}
