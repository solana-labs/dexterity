use std::{borrow::Borrow, cell::RefCell, marker::PhantomData, sync::Arc};

use agnostic_orderbook::state::SelfTradeBehavior;
use anchor_lang::Key;
use async_trait::async_trait;
use solana_client::{rpc_client::RpcClient, thin_client::ThinClient};
use solana_program::{hash::Hash, instruction::Instruction, message::Message, pubkey::Pubkey};
use solana_program_test::BanksClient;
use solana_sdk::{
    client::{Client, SyncClient},
    commitment_config::CommitmentLevel,
    signature::{Keypair, Signature, Signer},
    signer::SignerError,
    signers::Signers,
    transaction::Transaction,
    transport::TransportError,
};

use dex::{
    state::{constants::*, enums::*, products::Leg},
    utils::numeric::Fractional,
};

use crate::{
    common::utils::*,
    processor::{
        cancel_order::*, combo::*, consume_orderbook_events::*, deposit_funds::*, new_order::*,
        orderbook::*, transfer_full_position::*,
    },
    sdk_client::SDKClient,
    trader::SDKTrader,
};

use crate::common::Side;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Order {
    pub side: Side,
    pub size: Fractional,
    pub price: Fractional,
}

impl Order {
    pub fn new(side: Side, size: Fractional, price: Fractional) -> Self {
        Order { side, size, price }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct SDKProduct {
    pub key: Pubkey,
    pub name: [u8; NAME_LEN],
    pub orderbook: Pubkey,
    pub bids: Pubkey,
    pub asks: Pubkey,
    pub market_signer: Pubkey,
    pub event_queue: Pubkey,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct SDKCombo {
    pub key: Pubkey,
    pub name: [u8; NAME_LEN],
    pub orderbook: Pubkey,
    pub bids: Pubkey,
    pub asks: Pubkey,
    pub market_signer: Pubkey,
    pub event_queue: Pubkey,
}

impl Key for SDKProduct {
    fn key(&self) -> Pubkey {
        self.key
    }
}

impl Key for SDKCombo {
    fn key(&self) -> Pubkey {
        self.key
    }
}

pub fn clone_keypair(keypair: &Keypair) -> Keypair {
    Keypair::from_bytes(&keypair.to_bytes()).unwrap()
}

impl SDKProduct {
    pub fn market_signer(product_key: Pubkey, dex_program_id: Pubkey) -> (Pubkey, u8) {
        Pubkey::find_program_address(&[product_key.as_ref()], &dex_program_id)
    }
}
