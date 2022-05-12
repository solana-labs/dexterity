use anchor_lang::{InstructionData, ToAccountMetas};
use solana_program::{
    instruction::Instruction, program_error::ProgramError, pubkey::Pubkey,
    system_instruction::create_account, system_program, sysvar,
};
use solana_program_test::ProgramTestContext;
use solana_sdk::signature::{Keypair, Signer};

use crate::{
    common::utils::*,
    sdk_client::{ClientSubset, SDKClient},
    KeypairD,
};
use dex::state::{
    constants::*, fee_model::TraderFees, market_product_group::*,
    risk_engine_register::RiskOutputRegister,
};

pub fn initialize_market_product_group_ixs(
    client: &SDKClient,
    dex_program_id: Pubkey,
    vault_mint: Pubkey,
    market_product_group_vault: Pubkey,
    auth: &Keypair,
    fee_collector: Pubkey,
    fee_model_program_id: Pubkey,
    fee_model_configuration_acct: Pubkey,
    risk_model_configuration_acct: Pubkey,
    risk_engine_program_id: Pubkey,
    name: [u8; NAME_LEN],
    market_product_group: &Keypair,
    risk_output_register_keypair: &Keypair,
    fee_output_register_keypair: &Keypair,
    validate_account_discriminant_len: u64,
    find_fees_discriminant_len: u64,
    validate_account_health_discriminant: [u8; 8],
    validate_account_liquidation_discriminant: [u8; 8],
    create_risk_state_account_discriminant: [u8; 8],
    find_fees_discriminant: [u8; 8],
) -> Vec<Instruction> {
    let size = std::mem::size_of::<MarketProductGroup>() + 8;
    let lamports = client.rent_exempt(size).max(1);
    let create_market_product_group_ix = create_account(
        &client.payer.pubkey(),
        &market_product_group.pubkey(),
        lamports,
        size as u64,
        &dex_program_id,
    );
    dbg!(find_fees_discriminant_len);
    let initialize_market_product_group_ix = Instruction {
        program_id: dex::ID,
        data: dex::instruction::InitializeMarketProductGroup {
            params: dex::InitializeMarketProductGroupParams {
                name,
                validate_account_discriminant_len,
                find_fees_discriminant_len,
                validate_account_health_discriminant,
                validate_account_liquidation_discriminant,
                create_risk_state_account_discriminant,
                find_fees_discriminant,
                max_maker_fee_bps: 1000,
                min_maker_fee_bps: -100,
                max_taker_fee_bps: 1000,
                min_taker_fee_bps: -100,
            },
        }
        .data(),
        accounts: dex::accounts::InitializeMarketProductGroup {
            authority: auth.pubkey(),
            market_product_group: market_product_group.pubkey(),
            market_product_group_vault,
            vault_mint,
            fee_collector,
            fee_model_program: fee_model_program_id,
            fee_model_configuration_acct,
            risk_model_configuration_acct,
            risk_engine_program: risk_engine_program_id,
            sysvar_rent: sysvar::rent::id(),
            system_program: system_program::id(),
            token_program: spl_token::id(),
            fee_output_register: fee_output_register_keypair.pubkey(),
            risk_output_register: risk_output_register_keypair.pubkey(),
        }
        .to_account_metas(Some(true)),
    };

    let size = std::mem::size_of::<RiskOutputRegister>() as u64 + 8;
    let lamports = client.rent_exempt(size as usize).max(1);
    let create_risk_register_ix = create_account(
        &client.payer.pubkey(),
        &risk_output_register_keypair.pubkey(),
        lamports,
        size as u64,
        &risk_engine_program_id,
    );

    let size = std::mem::size_of::<TraderFees>() as u64;
    let lamports = client.rent_exempt(size as usize).max(1);
    let create_fee_register_ix = create_account(
        &client.payer.pubkey(),
        &fee_output_register_keypair.pubkey(),
        lamports,
        size,
        &fee_model_program_id,
    );
    vec![
        create_market_product_group_ix,
        initialize_market_product_group_ix,
        create_risk_register_ix,
        create_fee_register_ix,
    ]
}

pub async fn initialize_market_product_group(
    client: &SDKClient,
    market_product_group: &KeypairD,
    risk_output_register_keypair: &KeypairD,
    fee_output_register_keypair: &KeypairD,
    dex_program_id: Pubkey,
    vault_mint: Pubkey,
    vault: Pubkey,
    auth: &KeypairD,
    fee_collector: Pubkey,
    fee_model_program_id: Pubkey,
    fee_model_configuration_acct: Pubkey,
    risk_model_configuration_acct: Pubkey,
    risk_engine_program_id: Pubkey,
    name: [u8; NAME_LEN],
    validate_account_discriminant_len: u64,
    find_fees_discriminant_len: u64,
    validate_account_health_discriminant: [u8; 8],
    validate_account_liquidation_discriminant: [u8; 8],
    create_risk_state_account_discriminant: [u8; 8],
    find_fees_discriminant: [u8; 8],
) -> std::result::Result<Pubkey, SDKError> {
    let ixs = initialize_market_product_group_ixs(
        client,
        dex_program_id,
        vault_mint,
        vault,
        auth,
        fee_collector,
        fee_model_program_id,
        fee_model_configuration_acct,
        risk_model_configuration_acct,
        risk_engine_program_id,
        name,
        &market_product_group,
        &risk_output_register_keypair,
        &fee_output_register_keypair,
        validate_account_discriminant_len,
        find_fees_discriminant_len,
        validate_account_health_discriminant,
        validate_account_liquidation_discriminant,
        create_risk_state_account_discriminant,
        find_fees_discriminant,
    );
    client
        .sign_send_instructions(
            ixs,
            vec![
                &market_product_group,
                auth,
                &risk_output_register_keypair,
                &fee_output_register_keypair,
            ],
        )
        .await?;
    Ok(market_product_group.pubkey())
}
