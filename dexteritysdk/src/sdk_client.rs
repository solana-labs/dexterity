use std::{cell::RefCell, sync::Arc};

use anchor_client::{ClientError, Program as AnchorClient};
use anyhow::anyhow;
use async_trait::async_trait;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use solana_client::{
    rpc_client::RpcClient, rpc_config::RpcSendTransactionConfig, thin_client::ThinClient,
};
use solana_program::{
    account_info::AccountInfo, hash::Hash, instruction::Instruction, pubkey::Pubkey, rent::Rent,
    sysvar::SysvarId,
};
use solana_program_test::BanksClient;
use solana_sdk::{
    account::Account,
    client::SyncClient,
    commitment_config::{CommitmentConfig, CommitmentLevel},
    signature::Keypair,
    signer::Signer,
    transaction::Transaction,
    transport::TransportError,
};
use tokio::sync::RwLock;

use crate::{
    common::{
        utils::{SDKError, SDKResult},
        KeypairD,
    },
    state::clone_keypair,
};

#[async_trait]
pub trait ClientSubset {
    async fn process_transaction(&self, mut tx: Transaction, signers: &Vec<&Keypair>) -> SDKResult;
    async fn fetch_latest_blockhash(&self) -> std::result::Result<Hash, SDKError>;
    async fn fetch_account(&self, key: Pubkey) -> std::result::Result<Account, SDKError>;
}

pub trait ClientSubsetSync {
    fn process_transaction(&self, tx: Transaction, signers: &Vec<&Keypair>) -> SDKResult;
    fn fetch_latest_blockhash(&self) -> std::result::Result<Hash, SDKError>;
    fn fetch_account(&self, key: Pubkey) -> std::result::Result<Account, SDKError>;
}

#[derive(Clone)]
pub struct SDKClient {
    pub client: Arc<dyn ClientSubset + 'static + Sync + Send>,
    rent: Rent,
    pub payer: KeypairD,
}

impl SDKClient {
    pub async fn from_banks(
        client: &BanksClient,
        payer: &Keypair,
    ) -> std::result::Result<Self, SDKError> {
        let mut client = client.clone();
        let rent = client.get_rent().await?;
        Ok(Self {
            rent,
            client: Arc::new(RwLock::new(client)),
            payer: payer.into(),
        })
    }

    pub fn from_rpc(rpc: RpcClient, payer: &Keypair) -> std::result::Result<Self, SDKError> {
        let rent_account = rpc
            .get_account_with_commitment(
                &anchor_lang::prelude::Rent::id(),
                CommitmentConfig::confirmed(),
            )?
            .value
            .ok_or(anyhow!("Failed to fetch rent sysvar"))?;
        let rent = bincode::deserialize(&*rent_account.data).map_err(|e| anyhow::Error::from(e))?;
        Ok(Self {
            client: Arc::new(Arc::new(rpc)),
            rent,
            payer: payer.into(),
        })
    }

    pub async fn sign_send_instructions(
        &self,
        instructions: Vec<Instruction>,
        mut signers: Vec<&Keypair>, // todo: use slice
    ) -> std::result::Result<(), SDKError> {
        signers.insert(0, &self.payer);
        self.client
            .process_transaction(
                Transaction::new_with_payer(&instructions, Some(&self.payer.pubkey())),
                &signers,
            )
            .await
    }

    pub async fn get_latest_blockhash(&self) -> std::result::Result<Hash, SDKError> {
        self.client.fetch_latest_blockhash().await
    }

    pub fn rent_exempt(&self, size: usize) -> u64 {
        self.rent.minimum_balance(size) as u64
    }

    pub async fn get_account(&self, key: Pubkey) -> std::result::Result<Account, SDKError> {
        self.client.fetch_account(key).await
    }
}

#[async_trait]
impl ClientSubset for Arc<RpcClient> {
    async fn process_transaction(&self, tx: Transaction, signers: &Vec<&Keypair>) -> SDKResult {
        let client = self.clone();
        let signers_owned = signers
            .into_iter()
            .map(|&i| KeypairD::from(i).0)
            .collect_vec();

        tokio::task::spawn_blocking(move || {
            let signers = signers_owned.iter().collect();
            (*client).process_transaction(tx, &signers)
        })
        .await
        .map_err(|e| SDKError::Other(anyhow::Error::msg(e.to_string())))
        .and_then(|e| e)
    }

    async fn fetch_latest_blockhash(&self) -> std::result::Result<Hash, SDKError> {
        let client = self.clone();
        tokio::task::spawn_blocking(move || (*client).fetch_latest_blockhash())
            .await
            .map_err(|e| SDKError::Other(anyhow::Error::msg(e.to_string())))
            .and_then(|e| e)
    }

    async fn fetch_account(&self, key: Pubkey) -> std::result::Result<Account, SDKError> {
        let client = self.clone();
        tokio::task::spawn_blocking(move || (*client).fetch_account(key))
            .await
            .map_err(|e| SDKError::Other(anyhow::Error::msg(e.to_string())))
            .and_then(|e| e)
    }
}

impl ClientSubsetSync for RpcClient {
    fn process_transaction(&self, mut tx: Transaction, signers: &Vec<&Keypair>) -> SDKResult {
        tx.partial_sign(signers, self.get_latest_blockhash()?);
        self.send_and_confirm_transaction_with_spinner_and_config(
            &tx,
            CommitmentConfig::confirmed(),
            RpcSendTransactionConfig {
                skip_preflight: true,
                preflight_commitment: None,
                encoding: None,
                max_retries: None,
            },
        )?;
        Ok(())
    }

    fn fetch_latest_blockhash(&self) -> std::result::Result<Hash, SDKError> {
        Ok(self
            .get_latest_blockhash_with_commitment(CommitmentConfig::processed())
            .map(|(hash, _)| hash)?)
    }

    fn fetch_account(&self, key: Pubkey) -> std::result::Result<Account, SDKError> {
        Ok(self
            .get_account_with_commitment(&key, CommitmentConfig::processed())?
            .value
            .ok_or(anyhow!("Failed to get account"))?)
    }
}

#[async_trait]
impl ClientSubset for RwLock<BanksClient> {
    async fn process_transaction(&self, mut tx: Transaction, signers: &Vec<&Keypair>) -> SDKResult {
        tx.partial_sign(signers, self.fetch_latest_blockhash().await?);
        self.write()
            .await
            .process_transaction_with_commitment(tx, CommitmentLevel::Confirmed)
            .await?;
        Ok(())
    }

    async fn fetch_latest_blockhash(&self) -> std::result::Result<Hash, SDKError> {
        self.write()
            .await
            .get_latest_blockhash()
            .await
            .map_err(SDKError::from)
    }

    async fn fetch_account(&self, key: Pubkey) -> std::result::Result<Account, SDKError> {
        self.write()
            .await
            .get_account_with_commitment(key, CommitmentLevel::Confirmed)
            .await?
            .ok_or(anyhow!("Failed to get account").into())
    }
}

///////////// Non-interesting impls  ////////////////////
