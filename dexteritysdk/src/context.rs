// additional methods for context object

use itertools::Itertools;
use solana_program::{pubkey::Pubkey, system_instruction::create_account};
use solana_sdk::signature::{Keypair, Signer};

use constant_fees::initialize_trader_fee_acct_ix;

use crate::{
    common::KeypairD,
    create_token_account, initialize_trader_risk_group,
    processor::{
        clear_expired_orderbook::clear_expired_orderbook_ixs,
        consume_orderbook_events::consume_orderbook_events_ixs,
    },
    SDKContext, SDKResult, SDKTrader,
};

impl SDKContext {
    pub async fn crank_raw(
        &self,
        product_key: Pubkey,
        market_signer: Pubkey,
        orderbook: Pubkey,
        event_queue: Pubkey,
        reward_target: &KeypairD,
        trader_and_risk_accounts: &mut [Pubkey],
        max_iterations: u64,
    ) -> SDKResult {
        trader_and_risk_accounts.sort();
        trader_and_risk_accounts.iter_mut().dedup();
        let ixs = consume_orderbook_events_ixs(
            self.aaob_program_id,
            self.market_product_group,
            product_key,
            market_signer,
            orderbook,
            event_queue,
            reward_target,
            self.fee_model_program_id,
            self.fee_model_config_acct,
            self.risk_model_config_acct,
            self.fee_output_register,
            self.risk_engine_program_id,
            self.out_register_risk_info,
            trader_and_risk_accounts,
            max_iterations,
        );
        self.client
            .sign_send_instructions(ixs, vec![reward_target])
            .await
    }

    pub async fn clear_expired_orderbook(
        &self,
        product_key: Pubkey,
        market_signer: Pubkey,
        orderbook: Pubkey,
        event_queue: Pubkey,
        bids: Pubkey,
        asks: Pubkey,
        n: Option<u64>,
    ) -> SDKResult {
        let ixs = clear_expired_orderbook_ixs(
            self.aaob_program_id,
            self.market_product_group,
            product_key,
            market_signer,
            orderbook,
            event_queue,
            bids,
            asks,
            n,
        );
        self.client.sign_send_instructions(ixs, vec![]).await
    }

    pub async fn create_account(
        &self,
        to_address: &KeypairD,
        owner: &Pubkey,
        size: usize,
    ) -> SDKResult {
        let ix = create_account(
            &self.payer.pubkey(),
            &to_address.pubkey(),
            self.client.rent_exempt(size).max(1),
            size as u64,
            owner,
        );
        self.client
            .sign_send_instructions(vec![ix], vec![to_address])
            .await
    }

    pub async fn create_anchor_account<T>(
        &self,
        to_address: &KeypairD,
        owner: &Pubkey,
    ) -> SDKResult {
        let size = std::mem::size_of::<T>() + 8;
        self.create_account(to_address, owner, size).await
    }

    pub async fn register_trader(&self, keypair: impl Into<KeypairD>) -> SDKResult<SDKTrader> {
        let keypair = keypair.into();
        let risk_state_account = KeypairD::new();
        let trader_risk_group = KeypairD::new();
        let (trader_fee_acct, trader_fee_acct_bump) = Pubkey::find_program_address(
            &[
                b"trader_fee_acct",
                &trader_risk_group.pubkey().to_bytes(),
                &self.market_product_group.to_bytes(),
            ],
            &self.fee_model_program_id,
        );
        let (risk_signer, _) = Pubkey::find_program_address(
            &[self.market_product_group.as_ref()],
            &self.dex_program_id,
        );

        // allocate wallet
        let wallet =
            create_token_account(&self.client, &self.vault_mint, &keypair.pubkey()).await?;

        // initialize trader fee acct
        self.client
            .sign_send_instructions(
                vec![initialize_trader_fee_acct_ix(
                    self.fee_model_program_id,
                    self.payer.pubkey(),
                    self.fee_model_config_acct,
                    trader_fee_acct,
                    self.market_product_group,
                    trader_risk_group.pubkey(),
                    solana_program::system_program::id(),
                )],
                vec![],
            )
            .await?;

        initialize_trader_risk_group(
            &self.client,
            &trader_risk_group,
            self.dex_program_id,
            &keypair,
            self.market_product_group,
            trader_fee_acct,
            &risk_state_account,
            risk_signer,
            self.risk_engine_program_id,
        )
        .await?;

        Ok(SDKTrader {
            keypair,
            account: trader_risk_group.pubkey(),
            fee_acct: trader_fee_acct,
            fee_acct_bump: trader_fee_acct_bump,
            wallet: wallet,
            risk_state_account: risk_state_account.pubkey(),
        })
    }
}
