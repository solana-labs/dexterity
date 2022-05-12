use anchor_lang::{
    prelude::*,
    solana_program::{
        program::invoke_signed, program_error::ProgramError, program_pack::IsInitialized,
        pubkey::Pubkey,
    },
};

use crate::{
    error::{DomainOrProgramResult, UtilError},
    utils::validation::{assert, assert_keys_equal},
    DepositFunds, DepositFundsParams, DomainOrProgramError,
};

fn validate(accts: &DepositFunds) -> DomainOrProgramResult {
    let trader_risk_group = accts.trader_risk_group.load()?;
    assert_keys_equal(accts.token_program.key(), spl_token::ID)?;
    assert(
        trader_risk_group.is_initialized(),
        UtilError::AccountUninitialized,
    )?;
    assert_keys_equal(trader_risk_group.owner, accts.user.key())?;
    assert_keys_equal(
        trader_risk_group.market_product_group,
        accts.market_product_group.key(),
    )?;
    Ok(())
}

pub fn process(ctx: Context<DepositFunds>, params: DepositFundsParams) -> DomainOrProgramResult {
    let accts = ctx.accounts;
    validate(accts)?;
    let DepositFundsParams { quantity } = params;
    let mut trader_risk_group = accts.trader_risk_group.load_mut()?;
    let market_product_group = accts.market_product_group.load()?;
    let vault_seeds = &[
        b"market_vault",
        accts.market_product_group.as_ref().key.as_ref(),
        &[market_product_group.vault_bump as u8],
    ];
    let vault_key =
        Pubkey::create_program_address(vault_seeds, ctx.program_id).map_err(ProgramError::from)?;

    assert_keys_equal(vault_key, accts.market_product_group_vault.key())?;

    let token_quantity = quantity.round(market_product_group.decimals as u32)?;

    // check_funds(token_quantity)?;

    let token_transfer_instruction = spl_token::instruction::transfer(
        accts.token_program.key,
        &accts.user_token_account.key(),
        &accts.market_product_group_vault.key(),
        accts.user.key,
        &[],
        token_quantity.m as u64,
    )?;

    invoke_signed(
        &token_transfer_instruction,
        &[
            accts.token_program.to_account_info(),
            accts.user_token_account.to_account_info(),
            accts.market_product_group_vault.to_account_info(),
            accts.user.to_account_info(),
        ],
        &[vault_seeds],
    )?;
    trader_risk_group.total_deposited = trader_risk_group.total_deposited.checked_add(quantity)?;
    trader_risk_group.cash_balance = trader_risk_group.cash_balance.checked_add(quantity)?;

    Ok(())
}
