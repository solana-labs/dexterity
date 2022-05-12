use ::std::cell::Ref;
use anchor_lang::{
    prelude::*,
    solana_program::{
        log::sol_log_compute_units, msg, program::invoke_signed_unchecked,
        program_pack::IsInitialized, pubkey::Pubkey,
    },
};
use borsh::BorshSerialize;

use crate::{
    error::{DexError, DomainOrProgramResult, UtilError},
    state::risk_engine_register::*,
    utils::{
        cpi::risk_check,
        loadable::Loadable,
        numeric::{Fractional, ZERO_FRAC},
        validation::{assert_keys_equal, assert_valid_token_account_owner, check_funds},
    },
    validate_account_health_ix, WithdrawFunds, WithdrawFundsParams,
};

#[repr(C)]
#[derive(AnchorSerialize, AnchorDeserialize, PartialEq, Clone)]
pub struct Params {
    pub quantity: Fractional,
}

fn validate(ctx: &Context<WithdrawFunds>) -> DomainOrProgramResult {
    let accts = &ctx.accounts;
    let trader_risk_group = accts.trader_risk_group.load()?;
    let market_product_group = accts.market_product_group.load()?;

    if !trader_risk_group.is_initialized() {
        msg!("TraderRiskGroup account is not initialized yet!");
        return Err(UtilError::AccountUninitialized.into());
    }

    assert_keys_equal(
        market_product_group.risk_engine_program_id,
        accts.risk_engine_program.key(),
    )?;
    assert_keys_equal(
        trader_risk_group.market_product_group,
        accts.market_product_group.key(),
    )?;
    assert_valid_token_account_owner(accts.user_token_account.as_ref(), &accts.user.key())?;
    assert_keys_equal(accts.user.key(), trader_risk_group.owner)?;
    assert_keys_equal(
        trader_risk_group.risk_state_account,
        accts.trader_risk_state_acct.key(),
    )?;
    assert_keys_equal(
        market_product_group.risk_model_configuration_acct,
        accts.risk_model_configuration_acct.key(),
    )?;
    Ok(())
}

pub fn process<'info>(
    ctx: Context<'_, '_, '_, 'info, WithdrawFunds<'info>>,
    params: WithdrawFundsParams,
) -> DomainOrProgramResult {
    validate(&ctx)?;
    let accts = ctx.accounts;
    let mut trader_risk_group = accts.trader_risk_group.load_mut()?;
    let mut market_product_group = accts.market_product_group.load_mut()?;

    let WithdrawFundsParams { quantity } = params;

    let quantity = quantity.round(market_product_group.decimals as u32)?;

    let vault_seeds = &[
        b"market_vault",
        accts.market_product_group.as_ref().key.as_ref(),
        &[market_product_group.vault_bump as u8],
    ];
    let vault_key = Pubkey::create_program_address(vault_seeds, ctx.program_id)?;

    assert_keys_equal(vault_key, accts.market_product_group_vault.key())?;
    check_funds(quantity)?;
    // TODO: check max amount able to be withdrawn here

    trader_risk_group.apply_all_funding(&mut market_product_group)?;
    let token_transfer_instruction = spl_token::instruction::transfer(
        &accts.token_program.key(),
        &accts.market_product_group_vault.key(),
        &accts.user_token_account.key(),
        &accts.market_product_group_vault.key(),
        &[],
        quantity.m as u64,
    )?;
    invoke_signed_unchecked(
        &token_transfer_instruction,
        &[
            accts.token_program.to_account_info(),
            accts.market_product_group_vault.to_account_info(),
            accts.user_token_account.to_account_info(),
        ],
        &[vault_seeds],
    )?;

    trader_risk_group.cash_balance -= quantity;

    let risk_engine_output = risk_check(
        &accts.risk_engine_program,
        &accts.market_product_group,
        &accts.trader_risk_group,
        &accts.risk_output_register,
        &accts.trader_risk_state_acct,
        &accts.risk_model_configuration_acct,
        &accts.risk_signer,
        ctx.remaining_accounts,
        &OrderInfo {
            operation_type: OperationType::CheckHealth,
            ..Default::default()
        },
        market_product_group.get_validate_account_health_discriminant(),
        market_product_group.risk_and_fee_bump as u8,
    )?;

    let health_info = match risk_engine_output {
        HealthResult::Health { health_info: v } => v,
        HealthResult::Liquidation {
            liquidation_info: _,
        } => return Err(DexError::InvalidAccountHealthError.into()),
    };

    if health_info.action != ActionStatus::Approved {
        return Err(DexError::InvalidAccountHealthError.into());
    }

    trader_risk_group.total_withdrawn += quantity;
    market_product_group.sequence_number += 1;
    msg!("sequence: {}", market_product_group.sequence_number);
    accts.market_product_group.key().log();
    Ok(())
}
