use crate::{
    error::{DomainOrProgramResult, UtilError},
    utils::validation::{assert, assert_keys_equal},
    UpdateTraderFunding,
};
use anchor_lang::{prelude::*, solana_program::program_pack::IsInitialized};

pub fn process(ctx: Context<UpdateTraderFunding>) -> DomainOrProgramResult {
    let accts = ctx.accounts;
    let mut trader_risk_group = accts.trader_risk_group.load_mut()?;
    let mut market_product_group = accts.market_product_group.load_mut()?;

    // validate
    {
        assert_keys_equal(
            trader_risk_group.market_product_group,
            accts.market_product_group.key(),
        )?;
        assert(
            market_product_group.is_initialized(),
            UtilError::AccountUninitialized,
        )?;
        assert(
            trader_risk_group.is_initialized(),
            UtilError::AccountUninitialized,
        )?;
    }

    trader_risk_group.apply_all_funding(&mut market_product_group)?;
    market_product_group.sequence_number += 1;
    msg!("sequence: {}", market_product_group.sequence_number);
    accts.market_product_group.key().log();
    Ok(())
}
