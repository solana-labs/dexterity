use anchor_lang::{
    prelude::*,
    solana_program::{
        entrypoint::ProgramResult,
        msg,
        program::{invoke_signed_unchecked, invoke_unchecked},
        program_error::ProgramError,
        program_pack::IsInitialized,
        pubkey::Pubkey,
        system_instruction,
        sysvar::{clock::Clock, Sysvar},
    },
};
use borsh::BorshSerialize;
use std::{
    borrow::BorrowMut,
    cell::{Ref, RefMut},
};

use crate::{
    create_trader_risk_state_acct_ix,
    error::{DexError, DomainOrProgramResult, UtilError},
    find_fees_ix,
    state::{
        fee_model::TraderFeeParams,
        risk_engine_register::{HealthResult, OrderInfo, RiskOutputRegister},
    },
    utils::{
        loadable::Loadable,
        logs::DexOrderSummary,
        numeric::{fp32_mul, u64_to_quote, Fractional, ZERO_FRAC},
        orderbook::{get_bbo, update_prices},
        param::WithAcct,
        validation::{assert, assert_keys_equal},
    },
    validate_account_health_ix, DomainOrProgramError, MarketProductGroup, NewOrder, NewOrderParams,
    TraderRiskGroup,
};

pub fn find_fees<'a>(
    fee_model_program: &AccountInfo<'a>,
    market_product_group: &AccountInfo<'a>,
    trader_risk_group: &AccountLoader<'a, TraderRiskGroup>,
    trader_fee_state: &AccountInfo<'a>,
    fee_model_configuration_acct: &AccountInfo<'a>,
    fee_output_register: &AccountInfo<'a>,
    fee_signer: &AccountInfo<'a>,
    discriminant: Vec<u8>,
    fee_params: &TraderFeeParams,
    fee_bump: u8,
) -> ProgramResult {
    invoke_signed_unchecked(
        &find_fees_ix(
            fee_model_program.key(),
            market_product_group.key(),
            trader_risk_group.key(),
            trader_fee_state.key(),
            fee_model_configuration_acct.key(),
            fee_output_register.key(),
            fee_signer.key(),
            fee_params,
            discriminant,
        )?,
        &[
            fee_model_program.clone(),
            market_product_group.to_account_info(),
            trader_risk_group.to_account_info(),
            trader_fee_state.clone(),
            fee_model_configuration_acct.clone(),
            fee_output_register.clone(),
            fee_signer.clone(),
        ],
        &[&[market_product_group.key().as_ref(), &[fee_bump]]],
    )
}

pub fn risk_check<'a, 'c>(
    risk_engine_program: &AccountInfo<'a>,
    market_product_group: &AccountLoader<'a, MarketProductGroup>,
    trader_risk_group: &AccountLoader<'a, TraderRiskGroup>,
    risk_output_register: &AccountInfo<'a>,
    risk_state_account: &AccountInfo<'a>,
    risk_model_configuration_acct: &AccountInfo<'a>,
    risk_and_fee_signer: &AccountInfo<'a>,
    remaining_risk_accounts: &'c [AccountInfo<'a>],
    order_info: &OrderInfo,
    discriminant: Vec<u8>,
    risk_bump: u8,
) -> DomainOrProgramResult<HealthResult> {
    let mut risk_accounts = vec![];
    risk_accounts.extend_from_slice(&[
        risk_engine_program.clone(),
        market_product_group.to_account_info(),
        trader_risk_group.to_account_info(),
        risk_output_register.to_account_info(),
        risk_state_account.to_account_info(),
        risk_model_configuration_acct.to_account_info(),
        risk_and_fee_signer.to_account_info(),
    ]);
    risk_accounts.extend(remaining_risk_accounts.iter().cloned());
    let account_health_ix = validate_account_health_ix(
        risk_engine_program.key(),
        market_product_group.key(),
        trader_risk_group.key(),
        risk_output_register.key(),
        risk_state_account.key(),
        risk_model_configuration_acct.key(),
        risk_and_fee_signer.key(),
        remaining_risk_accounts.iter().map(Key::key).collect(),
        discriminant,
        order_info,
    )?;

    invoke_signed_unchecked(
        &account_health_ix,
        risk_accounts.as_slice(),
        &[&[market_product_group.key().as_ref(), &[risk_bump]]],
    )?;
    Ok(RiskOutputRegister::load(risk_output_register)?.risk_engine_output)
}

/// This CPI will create a risk state account for each user
pub fn create_risk_state_account<'a, 'c>(
    risk_engine_program: &AccountInfo<'a>,
    authority: &AccountInfo<'a>,
    risk_signer: &AccountInfo<'a>,
    risk_state_account: &AccountInfo<'a>,
    market_product_group: &AccountLoader<'a, MarketProductGroup>,
    system_program_account: &Program<'a, System>,
    remaining_risk_accounts: &'c [AccountInfo<'a>],
    discriminant: Vec<u8>,
    risk_bump: u8,
) -> DomainOrProgramResult {
    let mut risk_accounts = vec![];
    risk_accounts.extend_from_slice(&[
        risk_engine_program.to_account_info(),
        authority.to_account_info(),
        risk_signer.to_account_info(),
        risk_state_account.to_account_info(),
        market_product_group.to_account_info(),
        system_program_account.to_account_info(),
    ]);
    risk_accounts.extend(remaining_risk_accounts.iter().cloned());
    let risk_state_ix = create_trader_risk_state_acct_ix(
        risk_engine_program.key(),
        authority.key(),
        risk_signer.key(),
        &risk_state_account.to_account_info(),
        market_product_group.key(),
        system_program_account.key(),
        remaining_risk_accounts.iter().map(Key::key).collect(),
        discriminant,
    );

    invoke_signed_unchecked(
        &risk_state_ix,
        risk_accounts.as_slice(),
        &[&[market_product_group.key().as_ref(), &[risk_bump]]],
    )?;
    Ok(())
}
