use crate::{
    error::{DomainOrProgramError, DomainOrProgramResult},
    state::callback_info::CallBackInfoDex,
    utils::validation::assert_keys_equal,
    CreateMarketAccounts,
};
use agnostic_orderbook::error::AoError;
use anchor_lang::prelude::*;

fn validate(accts: &CreateMarketAccounts) -> DomainOrProgramResult {
    let market_product_group = accts.market_product_group.load().map_err(|e| {
        msg!("Failed to deserialize market product group");
        e
    })?;
    assert_keys_equal(accts.authority.key(), market_product_group.authority)?;

    Ok(())
}

pub fn process(
    ctx: Context<CreateMarketAccounts>,
    params: agnostic_orderbook::instruction::create_market::Params,
) -> DomainOrProgramResult {
    let accts = &ctx.accounts;
    validate(accts)?;

    let invoke_accounts = agnostic_orderbook::instruction::create_market::Accounts {
        market: &ctx.accounts.market,
        event_queue: &ctx.accounts.event_queue,
        bids: &ctx.accounts.bids,
        asks: &ctx.accounts.asks,
    };

    if let Err(error) = agnostic_orderbook::instruction::create_market::process::<CallBackInfoDex>(
        ctx.program_id,
        invoke_accounts,
        params,
    ) {
        return Err(DomainOrProgramError::ProgramErr(error));
    }

    return Ok(());
}
