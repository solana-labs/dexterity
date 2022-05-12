use crate::{
    error::{DomainOrProgramResult, UtilError},
    state::{
        constants::{HEALTH_BUFFER_LEN, MAX_OUTRIGHTS},
        enums::AccountTag,
    },
    utils::{cpi::create_risk_state_account, numeric::ZERO_FRAC, validation::assert_keys_equal},
    InitializeTraderRiskGroup,
};
use anchor_lang::{
    prelude::*,
    solana_program::{msg, pubkey::Pubkey},
};

pub fn process<'a, 'b, 'c, 'info>(
    ctx: Context<'a, 'b, 'c, 'info, InitializeTraderRiskGroup<'info>>,
) -> DomainOrProgramResult {
    let accts = ctx.accounts;
    let mut trader_risk_group = accts.trader_risk_group.load_init()?;
    let market_product_group = accts.market_product_group.load()?;
    if trader_risk_group.tag != AccountTag::Uninitialized {
        msg!("TraderRiskGroup account is already initialized");
        return Err(UtilError::AccountAlreadyInitialized.into());
    }
    assert_keys_equal(
        *accts.trader_fee_state_acct.owner,
        market_product_group.fee_model_program_id,
    )?;
    // Risk account should not be initialized
    assert_keys_equal(
        *accts.trader_risk_state_acct.owner,
        accts.system_program.key(),
    )?;
    trader_risk_group.fee_state_account = accts.trader_fee_state_acct.key();
    trader_risk_group.tag = AccountTag::TraderRiskGroup;
    trader_risk_group.owner = accts.owner.key();
    trader_risk_group.market_product_group = accts.market_product_group.key();
    trader_risk_group.cash_balance = ZERO_FRAC;
    trader_risk_group.pending_cash_balance = ZERO_FRAC;
    // Initialize fees as 0
    trader_risk_group.valid_until = 0;
    trader_risk_group.maker_fee_bps = 0;
    trader_risk_group.taker_fee_bps = 0;
    trader_risk_group.active_products = [u8::MAX; MAX_OUTRIGHTS];
    trader_risk_group.risk_state_account = accts.trader_risk_state_acct.key();
    trader_risk_group.client_order_id = 0;
    trader_risk_group.open_orders.initialize();
    trader_risk_group.pending_fees = ZERO_FRAC;

    let risk_program_id = accts.risk_engine_program.key();
    create_risk_state_account(
        &accts.risk_engine_program,
        &accts.owner,
        &accts.risk_signer,
        &accts.trader_risk_state_acct,
        &accts.market_product_group,
        &accts.system_program,
        &ctx.remaining_accounts,
        market_product_group
            .create_risk_state_account_discriminant
            .to_vec(),
        market_product_group.risk_and_fee_bump as u8,
    )?;

    // Risk state account should be initialized and assigned to the risk engine program
    assert_keys_equal(*accts.trader_risk_state_acct.owner, risk_program_id)?;

    Ok(())
}
