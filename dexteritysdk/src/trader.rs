use agnostic_orderbook::state::{SelfTradeBehavior, Side};
use anchor_lang::Key;
use anyhow::anyhow;
use dex::{
    state::{constants::SENTINEL, enums::OrderType, trader_risk_group::TraderRiskGroup},
    utils::numeric::Fractional,
};
use futures::future::join_all;
use solana_program::{instruction::Instruction, pubkey::Pubkey};
use solana_sdk::{
    signature::{Keypair, Signature, SignerError},
    signer::Signer,
};

use crate::{
    common::{utils::SDKResult, KeypairD},
    processor::{
        cancel_order::cancel_order_ixs,
        consume_orderbook_events::consume_orderbook_events_ixs,
        deposit_funds::{deposit_funds, deposit_funds_ixs},
        new_order::{new_order, new_order_ixs},
        transfer_full_position::transfer_full_position_ixs,
        update_trader_funding::update_trader_funding,
    },
    state::{Order, SDKProduct},
    SDKClient, SDKCombo, SDKContext, SDKError,
};

#[derive(Debug, Clone)]
pub struct SDKTrader {
    pub keypair: KeypairD,
    pub account: Pubkey, // trader risk group key
    pub fee_acct: Pubkey,
    pub fee_acct_bump: u8,
    pub wallet: Pubkey,
    pub risk_state_account: Pubkey,
}

impl SDKTrader {
    pub async fn connect(
        ctx: &SDKContext,
        account: Pubkey,
        keypair: Keypair,
        wallet: Pubkey,
    ) -> std::result::Result<Self, SDKError> {
        let trg = ctx
            .client
            .get_anchor_account::<TraderRiskGroup>(account)
            .await;
        if trg.owner != keypair.pubkey() {
            return Err(SDKError::PublicKeyMismatch);
        }
        if trg.market_product_group != ctx.market_product_group {
            return Err(SDKError::PublicKeyMismatch);
        }

        let (trader_fee_acct, trader_fee_acct_bump) = Pubkey::find_program_address(
            &[
                b"trader_fee_acct",
                &account.to_bytes(),
                &trg.market_product_group.to_bytes(),
            ],
            &ctx.fee_model_program_id,
        );
        if trader_fee_acct != trg.fee_state_account {
            return Err(SDKError::PublicKeyMismatch);
        }

        Ok(Self {
            keypair: KeypairD(keypair),
            account,
            fee_acct: trader_fee_acct,
            fee_acct_bump: trader_fee_acct_bump,
            wallet: wallet,
            risk_state_account: trg.risk_state_account,
        })
    }

    pub async fn get_trader_risk_group(&self, client: &SDKClient) -> Box<TraderRiskGroup> {
        client.get_anchor_account(self.account).await
    }

    pub async fn deposit(&self, ctx: &SDKContext, qty: impl Into<Fractional>) -> SDKResult {
        let ixs = deposit_funds_ixs(
            self.keypair.pubkey(),
            self.wallet,
            self.account,
            ctx.market_product_group,
            ctx.vault,
            qty.into(),
        );
        ctx.client
            .sign_send_instructions(ixs, vec![&self.keypair])
            .await
    }

    pub async fn apply_funding(&self, ctx: &SDKContext, market_product_group: Pubkey) -> SDKResult {
        update_trader_funding(&ctx.client, self.account, market_product_group).await
    }

    pub async fn transfer_funds(
        &self,
        ctx: &SDKContext,
        trader_wallet: Pubkey,
        trader: &SDKTrader,
        market_product_group: Pubkey,
        market_product_group_vault: Pubkey,
        quantity: Fractional,
    ) -> SDKResult {
        deposit_funds(
            &ctx.client,
            ctx.dex_program_id,
            spl_token::ID,
            &trader.keypair,
            trader_wallet,
            trader.account,
            market_product_group,
            market_product_group_vault,
            quantity,
        )
        .await
    }

    pub async fn transfer_position(
        &self,
        ctx: &SDKContext,
        market_product_group: Pubkey,
        liquidatee_risk_group: Pubkey,
        liquidatee_risk_state_account: Pubkey,
    ) -> SDKResult {
        let ixs = transfer_full_position_ixs(
            self.keypair.pubkey(),
            liquidatee_risk_group,
            self.account,
            market_product_group,
            ctx.risk_engine_program_id,
            ctx.out_register_risk_info,
            self.risk_state_account,
            liquidatee_risk_state_account,
            ctx.risk_model_config_acct,
        );
        ctx.client
            .sign_send_instructions(ixs, vec![&self.keypair])
            .await
    }

    pub async fn place_order_with_self_trade_behavior(
        &self,
        ctx: &SDKContext,
        product: &SDKProduct,
        side: Side,
        size: impl Into<Fractional>,
        price: impl Into<Fractional>,
        self_trade_behavior: SelfTradeBehavior,
        risk_accounts: &[Pubkey],
        order_type: OrderType,
    ) -> SDKResult {
        let ixs = new_order_ixs(
            ctx.aaob_program_id,
            self.keypair.pubkey(),
            self.account,
            ctx.market_product_group,
            product.key(),
            product.market_signer,
            product.orderbook,
            product.event_queue,
            product.bids,
            product.asks,
            ctx.fee_model_program_id,
            ctx.fee_model_config_acct,
            self.fee_acct,
            ctx.fee_output_register,
            ctx.risk_engine_program_id,
            ctx.risk_model_config_acct,
            risk_accounts,
            side,
            size.into(),
            order_type,
            self_trade_behavior,
            50,
            price.into(),
            ctx.out_register_risk_info,
            self.risk_state_account,
        );
        ctx.client
            .sign_send_instructions(ixs, vec![&self.keypair])
            .await
    }

    pub async fn place_order(
        &self,
        ctx: &SDKContext,
        product: &SDKProduct,
        side: Side,
        size: impl Into<Fractional>,
        price: impl Into<Fractional>,
    ) -> SDKResult {
        self.place_order_with_self_trade_behavior(
            ctx,
            product,
            side,
            size,
            price,
            SelfTradeBehavior::DecrementTake,
            &[],
            OrderType::Limit,
        )
        .await
    }

    pub async fn place_ioc_order(
        &self,
        ctx: &SDKContext,
        product: &SDKProduct,
        side: Side,
        size: impl Into<Fractional>,
        price: impl Into<Fractional>,
    ) -> SDKResult {
        self.place_order_with_self_trade_behavior(
            ctx,
            product,
            side,
            size,
            price,
            SelfTradeBehavior::DecrementTake,
            &[],
            OrderType::ImmediateOrCancel,
        )
        .await
    }

    pub async fn place_order_with_risk_accts(
        &self,
        ctx: &SDKContext,
        product: &SDKProduct,
        side: Side,
        size: impl Into<Fractional>,
        price: impl Into<Fractional>,
        risk_accounts: &[Pubkey],
    ) -> SDKResult {
        self.place_order_with_self_trade_behavior(
            ctx,
            product,
            side,
            size,
            price,
            SelfTradeBehavior::DecrementTake,
            &risk_accounts,
            OrderType::Limit,
        )
        .await
    }

    pub async fn place_ioc_order_with_risk_accts(
        &self,
        ctx: &SDKContext,
        product: &SDKProduct,
        side: Side,
        size: impl Into<Fractional>,
        price: impl Into<Fractional>,
        risk_accounts: &[Pubkey],
    ) -> SDKResult {
        self.place_order_with_self_trade_behavior(
            ctx,
            product,
            side,
            size,
            price,
            SelfTradeBehavior::DecrementTake,
            &risk_accounts,
            OrderType::ImmediateOrCancel,
        )
        .await
    }

    pub async fn place_combo_order(
        &self,
        ctx: &SDKContext,
        combo: &SDKCombo,
        side: Side,
        size: impl Into<Fractional>,
        price: impl Into<Fractional>,
    ) -> SDKResult {
        new_order(
            &ctx.client,
            ctx.dex_program_id,
            ctx.aaob_program_id,
            &self.keypair,
            self.account,
            ctx.market_product_group,
            combo.key(),
            combo.market_signer,
            combo.orderbook,
            combo.event_queue,
            combo.bids,
            combo.asks,
            ctx.fee_model_program_id,
            ctx.fee_model_config_acct,
            self.fee_acct,
            ctx.fee_output_register,
            ctx.risk_engine_program_id,
            ctx.risk_model_config_acct,
            &[],
            side,
            size.into(),
            OrderType::Limit,
            SelfTradeBehavior::DecrementTake,
            50,
            price.into(),
            ctx.out_register_risk_info,
            self.risk_state_account,
        )
        .await
    }

    pub async fn place_orders(
        &self,
        ctx: &SDKContext,
        product: &SDKProduct,
        orders: Vec<Order>,
    ) -> SDKResult {
        let mut ixs: Vec<Instruction> = vec![];
        for order in orders.into_iter() {
            ixs.append(&mut new_order_ixs(
                ctx.aaob_program_id,
                self.keypair.pubkey(),
                self.account,
                ctx.market_product_group,
                product.key(),
                product.market_signer,
                product.orderbook,
                product.event_queue,
                product.bids,
                product.asks,
                ctx.fee_model_program_id,
                ctx.fee_model_config_acct,
                self.fee_acct,
                ctx.fee_output_register,
                ctx.risk_engine_program_id,
                ctx.risk_model_config_acct,
                &[],
                *order.side,
                order.size,
                OrderType::Limit,
                SelfTradeBehavior::DecrementTake,
                50,
                order.price,
                ctx.out_register_risk_info,
                self.risk_state_account,
            ));
        }
        ctx.client
            .sign_send_instructions(ixs, vec![&self.keypair])
            .await
    }

    pub async fn crank(
        &self,
        ctx: &SDKContext,
        product: &SDKProduct,
        other_traders: &[&SDKTrader],
    ) -> SDKResult {
        let mut accts = Vec::with_capacity(3 + other_traders.len() * 3);
        accts.extend_from_slice(&[self.account, self.fee_acct, self.risk_state_account]);
        accts.extend(
            other_traders
                .iter()
                .flat_map(|t| [t.account, t.fee_acct, t.risk_state_account].into_iter()),
        );
        ctx.crank_raw(
            product.key(),
            product.market_signer,
            product.orderbook,
            product.event_queue,
            &self.keypair,
            accts.as_mut_slice(),
            4,
        )
        .await
    }

    pub async fn cancel_all_orders(
        &self,
        ctx: &SDKContext,
        product_indices: &[usize],
    ) -> SDKResult {
        let trader_risk_group = self.get_trader_risk_group(&ctx.client).await;
        for &n in product_indices.iter() {
            let mut order_ids: Vec<u128> = vec![];
            let mut ptr = trader_risk_group.open_orders.products[n].head_index as usize;
            let order = trader_risk_group.open_orders.orders[ptr];
            assert_eq!(order.prev, SENTINEL);
            while ptr != SENTINEL {
                let order = trader_risk_group.open_orders.orders[ptr];
                assert_ne!(order.id, 0);
                order_ids.push(order.id);
                ptr = order.next;
            }
            if !order_ids.is_empty() {
                match self
                    .cancel_orders(ctx, &ctx.products[n], order_ids.clone())
                    .await
                {
                    Ok(_) => {}
                    Err(_) => {
                        println!("Atomic cancel failed, attempting to cancel indiivdually");
                        let mut cancels = vec![];
                        for order_id in order_ids.into_iter() {
                            cancels.push(self.cancel_order(ctx, &ctx.products[n], order_id));
                        }
                        join_all(cancels).await;
                    }
                };
            }
        }

        Ok(())
    }
    pub async fn cancel(
        &self,
        ctx: &SDKContext,
        product: &SDKProduct,
        order_id: u128,
    ) -> SDKResult {
        self.cancel_underwater(ctx, product, order_id, self).await
    }

    pub async fn cancel_underwater(
        &self,
        ctx: &SDKContext,
        product: &SDKProduct,
        order_id: u128,
        under_water_trader: &SDKTrader,
    ) -> SDKResult {
        ctx.client
            .sign_send_instructions(
                cancel_order_ixs(
                    ctx.aaob_program_id,
                    self.keypair.pubkey(),
                    under_water_trader.account,
                    ctx.market_product_group,
                    product.key(),
                    product.market_signer,
                    product.orderbook,
                    product.event_queue,
                    product.bids,
                    product.asks,
                    ctx.risk_engine_program_id,
                    vec![],
                    order_id,
                    ctx.out_register_risk_info,
                    under_water_trader.risk_state_account,
                    ctx.risk_model_config_acct,
                ),
                vec![&self.keypair],
            )
            .await
    }

    pub async fn cancel_order(
        &self,
        ctx: &SDKContext,
        product: &SDKProduct,
        order: u128,
    ) -> SDKResult {
        let mut ixs: Vec<Instruction> = vec![];
        ixs.append(&mut cancel_order_ixs(
            ctx.aaob_program_id,
            self.keypair.pubkey(),
            self.account,
            ctx.market_product_group,
            product.key(),
            product.market_signer,
            product.orderbook,
            product.event_queue,
            product.bids,
            product.asks,
            ctx.risk_engine_program_id,
            vec![],
            order,
            ctx.out_register_risk_info,
            self.risk_state_account,
            ctx.risk_model_config_acct,
        ));
        ctx.client
            .sign_send_instructions(ixs, vec![&self.keypair])
            .await
    }

    pub async fn cancel_orders(
        &self,
        ctx: &SDKContext,
        product: &SDKProduct,
        orders: Vec<u128>,
    ) -> SDKResult {
        let mut ixs: Vec<Instruction> = vec![];
        for order_id in orders.into_iter() {
            ixs.append(&mut cancel_order_ixs(
                ctx.aaob_program_id,
                self.keypair.pubkey(),
                self.account,
                ctx.market_product_group,
                product.key(),
                product.market_signer,
                product.orderbook,
                product.event_queue,
                product.bids,
                product.asks,
                ctx.risk_engine_program_id,
                vec![],
                order_id,
                ctx.out_register_risk_info,
                self.risk_state_account,
                ctx.risk_model_config_acct,
            ));
        }
        ctx.client
            .sign_send_instructions(ixs, vec![&self.keypair])
            .await
    }
}

impl Key for SDKTrader {
    fn key(&self) -> Pubkey {
        self.keypair.pubkey()
    }
}
