use std::{
    cell::RefCell,
    rc::Rc,
    sync::{Arc, LockResult, PoisonError},
};

use anchor_client::ClientError;
use anchor_lang::Key;
use anyhow::{anyhow, Error};
use solana_program::{
    instruction::Instruction, message::Message, program_error::ProgramError,
    program_option::COption, program_pack::Pack, pubkey::Pubkey, sysvar::rent::Rent,
};
use solana_program_test::{BanksClientError, ProgramTest, ProgramTestContext};
use solana_sdk::{
    account::{Account, ReadableAccount},
    client::{Client, SyncClient},
    signature::{Keypair, Signer},
    system_instruction,
    transaction::Transaction,
    transport::TransportError,
};
use spl_associated_token_account::{create_associated_token_account, get_associated_token_address};
use spl_token::{instruction, state::Mint};
use thiserror::Error;
use tokio::task::spawn_blocking;

use constant_fees::update_fees_ix;
use dex::{
    error::UtilError,
    state::{market_product_group::MarketProductGroup, trader_risk_group::TraderRiskGroup},
    utils::{loadable::Loadable, numeric::Fractional},
};

use crate::{
    sdk_client::{ClientSubset, SDKClient},
    state::clone_keypair,
    KeypairD, SDKContext,
};

pub type MintInfo = (Pubkey, Mint);
pub const NUM_TOKENS: u64 = 1_000_000_000_000_000; // decimal 6

pub type SDKResult<T = ()> = std::result::Result<T, SDKError>;

#[derive(Error, Debug)]
pub enum SDKError {
    #[error("Public keys expected to match but do not")]
    PublicKeyMismatch,
    #[error("Action requires admin key")]
    RequiresAdmin,
    #[error("Solana client error")]
    SolanaClient(#[from] solana_client::client_error::ClientError),
    #[error("Some other error")]
    Other(#[from] anyhow::Error),
    #[error("Transaction Failed")]
    TransactionFailed,
    #[error("Transport Error")]
    TransportError(#[from] TransportError),
    #[error("Program Error")]
    ProgramError(#[from] ProgramError),
    #[error("UtilError")]
    UtilError(#[from] UtilError),
    #[error("Anchor client error")]
    AnchorClient(#[from] anchor_client::ClientError),
}

impl From<Box<dyn std::error::Error>> for SDKError {
    fn from(e: Box<dyn std::error::Error>) -> Self {
        SDKError::Other(anyhow::Error::msg(e.to_string()))
    }
}

impl<T> From<PoisonError<T>> for SDKError {
    fn from(e: PoisonError<T>) -> Self {
        SDKError::Other(anyhow::Error::msg(e.to_string()))
    }
}

impl From<BanksClientError> for SDKError {
    fn from(e: BanksClientError) -> Self {
        SDKError::Other(anyhow::Error::msg(e.to_string()))
    }
}

impl From<std::io::Error> for SDKError {
    fn from(e: std::io::Error) -> Self {
        SDKError::TransportError(TransportError::from(e))
    }
}

pub fn log_disable() {
    solana_logger::setup_with_default(
        "solana_rbpf::vm=error,\
         solana_program_runtime=error,\
         solana_program_runtime::message_processor=error,\
         solana_program_test=error",
    )
}

pub fn log_enable() {
    solana_logger::setup_with_default(
        "solana_rbpf::vm=error,\
         solana_program_runtime=debug,\
         solana_runtime::system_instruction_processor=debug,\
         solana_program_test=error",
    )
}

pub fn log_default() {
    solana_logger::setup_with_default(
        "solana_rbpf::vm=warn,\
         solana_program_runtime::message_processor=info,\
         solana_runtime::system_instruction_processor=error,\
         solana_program_test=info",
    );
}

impl SDKClient {
    pub async fn load_account<T>(&self, key: Pubkey) -> Box<T>
    where
        T: Loadable,
    {
        let account = self.get_account(key).await.unwrap();
        Box::new(*T::load_from_bytes(account.data.as_slice()).unwrap())
    }

    pub async fn get_anchor_account<T>(&self, key: Pubkey) -> Box<T>
    where
        T: Loadable,
    {
        let account = self.get_account(key).await.unwrap();
        Box::new(*T::load_from_bytes(&account.data[8..]).unwrap())
    }
}

pub async fn create_fee_program_config_acct(
    client: &SDKClient,
    fee_acct_global_config: &Pubkey,
    fee_acct_program_id: &Pubkey,
    market_product_group: Pubkey,
) -> std::result::Result<(), SDKError> {
    client
        .sign_send_instructions(
            vec![update_fees_ix(
                *fee_acct_program_id,
                client.payer.pubkey(),
                *fee_acct_global_config,
                market_product_group,
                solana_program::system_program::id(),
                constant_fees::UpdateFeesParams {
                    maker_fee_bps: 0,
                    taker_fee_bps: 0,
                },
            )],
            vec![],
        )
        .await
}

pub async fn create_mint2(
    client: &SDKClient,
    pool_mint: &KeypairD,
    mint_authority: &KeypairD,
    decimals: u8,
) -> std::result::Result<(), SDKError> {
    let instructions = &[
        system_instruction::create_account(
            &client.payer.pubkey(),
            &pool_mint.pubkey(),
            client.rent_exempt(Mint::LEN) as u64,
            Mint::LEN as u64,
            &spl_token::ID,
        ),
        instruction::initialize_mint(
            &spl_token::ID,
            &pool_mint.pubkey(),
            &mint_authority.pubkey(),
            None,
            decimals,
        )
        .unwrap(),
    ];
    client
        .sign_send_instructions(instructions.iter().cloned().collect(), vec![pool_mint])
        .await?;
    Ok(())
}

pub async fn mint_to(
    client: &SDKClient,
    mint: &Pubkey,
    account: &Pubkey,
    mint_authority: &KeypairD,
    amount: u64,
) -> std::result::Result<(), SDKError> {
    client
        .sign_send_instructions(
            vec![instruction::mint_to(
                &spl_token::ID,
                mint,
                account,
                &mint_authority.pubkey(),
                &[],
                amount,
            )
            .unwrap()],
            vec![&client.payer, mint_authority],
        )
        .await
}

pub async fn create_token_account(
    client: &SDKClient,
    pool_mint: &Pubkey,
    owner: &Pubkey,
) -> std::result::Result<Pubkey, SDKError> {
    let account_rent = client.rent_exempt(spl_token::state::Account::LEN);
    let ix = create_associated_token_account(&client.payer.pubkey(), &owner, pool_mint);
    let ata = get_associated_token_address(&owner, &pool_mint);
    match client.get_account(ata).await {
        Ok(x) => {
            if x.data().len() == 0 {
                client
                    .sign_send_instructions(vec![ix], vec![&client.payer])
                    .await?;
            }
        }
        Err(_) => {
            client
                .sign_send_instructions(vec![ix], vec![&client.payer])
                .await?;
        }
    }
    Ok(ata)
}

pub async fn transfer(
    prg_test_ctx: &mut ProgramTestContext,
    source: &Pubkey,
    destination: &Pubkey,
    authority: &KeypairD,
    amount: u64,
) -> std::result::Result<(), TransportError> {
    let payer = &Keypair::from_bytes(&prg_test_ctx.payer.to_bytes()).unwrap();
    let banks_client = &mut prg_test_ctx.banks_client;
    let recent_blockhash = prg_test_ctx.last_blockhash;

    let transaction = Transaction::new_signed_with_payer(
        &[instruction::transfer(
            &spl_token::ID,
            source,
            destination,
            &authority.pubkey(),
            &[],
            amount,
        )
        .unwrap()],
        Some(&payer.pubkey()),
        &[payer, authority],
        recent_blockhash,
    );
    banks_client.process_transaction(transaction).await?;
    Ok(())
}

#[inline]
pub fn assert_eq_frac(a: impl Into<Fractional>, b: impl Into<Fractional>) {
    assert_eq!(a.into().get_reduced_form(), b.into().get_reduced_form());
}
