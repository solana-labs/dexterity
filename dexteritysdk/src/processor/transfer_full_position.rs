use crate::{common::utils::*, sdk_client::SDKClient};
use anchor_lang::{InstructionData, ToAccountMetas};
use dex::{accounts, instruction};
use solana_program::{instruction::Instruction, pubkey::Pubkey};
use solana_program_test::ProgramTestContext;
use solana_sdk::signature::{Keypair, Signer};

pub fn transfer_full_position_ixs(
    user: Pubkey,
    liquidatee_risk_group: Pubkey,
    liquidator_risk_group: Pubkey,
    market_product_group: Pubkey,
    risk_engine_program: Pubkey,
    risk_output_register: Pubkey,
    liquidator_risk_state_account_info: Pubkey,
    liquidatee_risk_state_account_info: Pubkey,
    risk_model_configuration_acct: Pubkey,
) -> Vec<Instruction> {
    let (risk_signer, _) = Pubkey::find_program_address(&[market_product_group.as_ref()], &dex::ID);
    let account_metas = accounts::TransferFullPosition {
        liquidator: user,
        market_product_group,
        liquidatee_risk_group,
        liquidator_risk_group,
        risk_engine_program,
        risk_model_configuration_acct,
        risk_output_register,
        liquidator_risk_state_account_info,
        liquidatee_risk_state_account_info,
        risk_signer,
    }
    .to_account_metas(Some(true));

    vec![Instruction {
        program_id: dex::ID,
        data: instruction::TransferFullPosition {}.data(),
        accounts: account_metas,
    }]
}
