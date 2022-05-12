use anchor_lang::{InstructionData, ToAccountMetas};
use anyhow::anyhow;
use solana_program::{instruction::Instruction, pubkey::Pubkey};
use solana_sdk::signer::Signer;

use constant_fees::update_fees_ix;
use dex::{accounts, instruction};

use crate::{admin::DexAdmin, common::utils::SDKResult, SDKContext, SDKError};

pub fn sweep_fees_ix(
    market_product_group: Pubkey,
    fee_collector: Pubkey,
    fee_collector_token_account: Pubkey,
    market_product_group_vault: Pubkey,
) -> Vec<Instruction> {
    let accts = accounts::SweepFees {
        market_product_group,
        fee_collector,
        fee_collector_token_account,
        market_product_group_vault,
        token_program: spl_token::ID,
    };
    vec![Instruction {
        program_id: dex::ID,
        accounts: accts.to_account_metas(None),
        data: instruction::SweepFees {}.data(),
    }]
}

impl DexAdmin {
    pub async fn sweep_fees(&self) -> SDKResult {
        self.client
            .sign_send_instructions(
                sweep_fees_ix(
                    self.market_product_group,
                    self.fee_collector.pubkey(),
                    self.fee_collector_wallet,
                    self.vault,
                ),
                vec![],
            )
            .await?;
        Ok(())
    }

    pub async fn update_fees(&self, maker_fee_bps: i32, taker_fee_bps: i32) -> SDKResult {
        self.client
            .sign_send_instructions(
                vec![update_fees_ix(
                    self.fee_model_program_id,
                    self.payer.pubkey(),
                    self.fee_model_config_acct,
                    self.market_product_group,
                    solana_program::system_program::id(),
                    constant_fees::UpdateFeesParams {
                        maker_fee_bps,
                        taker_fee_bps,
                    },
                )],
                vec![&self.authority],
            )
            .await
    }
}
