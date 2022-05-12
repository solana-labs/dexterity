use anchor_lang::{
    prelude::*,
    solana_program::{
        log::sol_log_compute_units, program::invoke_signed, program_pack::IsInitialized,
    },
};

use crate::{
    error::{DomainOrProgramResult, UtilError},
    utils::{
        numeric::Fractional,
        validation::{assert, assert_keys_equal, assert_valid_token_account_owner},
    },
    SweepFees,
};

fn validate(ctx: &Context<SweepFees>) -> DomainOrProgramResult {
    let market_product_group = ctx.accounts.market_product_group.load()?;
    assert(
        market_product_group.is_initialized(),
        UtilError::AccountUninitialized,
    )?;
    assert_keys_equal(
        market_product_group.fee_collector,
        ctx.accounts.fee_collector.key(),
    )?;
    assert_valid_token_account_owner(
        ctx.accounts.fee_collector_token_account.as_ref(),
        &ctx.accounts.fee_collector.key(),
    )?;
    Ok(())
}

pub fn process(ctx: Context<SweepFees>) -> DomainOrProgramResult {
    validate(&ctx)?;
    let accts = ctx.accounts;
    let mut market_product_group = accts.market_product_group.load_mut()?;

    let fees_to_sweep = market_product_group
        .collected_fees
        .round_unchecked(market_product_group.decimals as u32)?;
    market_product_group.collected_fees = market_product_group
        .collected_fees
        .checked_sub(fees_to_sweep)?;

    let vault_seeds = &[
        b"market_vault",
        accts.market_product_group.as_ref().key.as_ref(),
        &[market_product_group.vault_bump as u8],
    ];
    let vault_key = Pubkey::create_program_address(vault_seeds, ctx.program_id)?;
    assert_keys_equal(vault_key, accts.market_product_group_vault.key())?;

    let token_transfer_instruction = spl_token::instruction::transfer(
        &accts.token_program.key(),
        &accts.market_product_group_vault.key(),
        &accts.fee_collector_token_account.key(),
        &accts.market_product_group_vault.key(),
        &[],
        fees_to_sweep.m as u64,
    )?;
    invoke_signed(
        &token_transfer_instruction,
        &[
            accts.token_program.to_account_info(),
            accts.market_product_group_vault.to_account_info(),
            accts.fee_collector_token_account.to_account_info(),
        ],
        &[vault_seeds],
    )?;
    market_product_group.sequence_number += 1;
    msg!("sequence: {}", market_product_group.sequence_number);
    accts.market_product_group.key().log();
    Ok(())
}
