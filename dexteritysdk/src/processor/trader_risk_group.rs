use anchor_lang::{InstructionData, ToAccountMetas};
use solana_program::{
    instruction::Instruction, pubkey::Pubkey, system_instruction::create_account,
};
use solana_program_test::ProgramTestContext;
use solana_sdk::signature::{Keypair, Signer};

use dex::{accounts, instruction, state::trader_risk_group::*};

use crate::{common::utils::*, sdk_client::SDKClient, KeypairD};

pub fn initialize_trader_risk_group_ixs(
    client: &SDKClient,
    dex_program_id: Pubkey,
    owner: &KeypairD,
    market_product_group: Pubkey,
    trader_risk_group: &KeypairD,
    trader_fee_state_acct: Pubkey,
    trader_risk_state_acct: Pubkey,
    risk_signer: Pubkey,
    risk_engine_program: Pubkey,
    override_risk_account_signer: bool,
) -> Vec<Instruction> {
    let size = std::mem::size_of::<TraderRiskGroup>() as u64 + 8;
    let lamports = client.rent_exempt(size as usize).max(1);
    let create_trader_risk_group_ix = create_account(
        &client.payer.pubkey(),
        &trader_risk_group.pubkey(),
        lamports,
        size,
        &dex_program_id,
    );
    let initialize_trader_risk_group_ix = Instruction {
        program_id: dex::ID,
        data: instruction::InitializeTraderRiskGroup.data(),
        accounts: accounts::InitializeTraderRiskGroup {
            owner: owner.pubkey(),
            trader_risk_group: trader_risk_group.pubkey(),
            market_product_group,
            risk_signer,
            trader_risk_state_acct,
            trader_fee_state_acct,
            risk_engine_program,
            system_program: solana_program::system_program::id(),
        }
        .to_account_metas(None),
    };
    vec![create_trader_risk_group_ix, initialize_trader_risk_group_ix]
}

pub async fn initialize_trader_risk_group(
    client: &SDKClient,
    trader_risk_group: &KeypairD,
    dex_program_id: Pubkey,
    owner: &KeypairD,
    market_product_group: Pubkey,
    fee_state_account: Pubkey,
    risk_state_account: &KeypairD,
    risk_signer: Pubkey,
    risk_engine_program: Pubkey,
) -> SDKResult {
    let ixs = initialize_trader_risk_group_ixs(
        client,
        dex_program_id,
        owner,
        market_product_group,
        trader_risk_group,
        fee_state_account,
        risk_state_account.pubkey(),
        risk_signer,
        risk_engine_program,
        true,
    );
    client
        .sign_send_instructions(ixs, vec![&trader_risk_group, owner, risk_state_account])
        .await
}

pub async fn initialize_trader_risk_group_pda(
    client: &SDKClient,
    trader_risk_group: &KeypairD,
    dex_program_id: Pubkey,
    owner: &KeypairD,
    market_product_group: Pubkey,
    fee_state_account: Pubkey,
    risk_state_account: Pubkey,
    risk_signer: Pubkey,
    risk_engine_program: Pubkey,
) -> SDKResult {
    let ixs = initialize_trader_risk_group_ixs(
        client,
        dex_program_id,
        owner,
        market_product_group,
        trader_risk_group,
        fee_state_account,
        risk_state_account,
        risk_signer,
        risk_engine_program,
        false,
    );
    client
        .sign_send_instructions(ixs, vec![&trader_risk_group, owner])
        .await
}
