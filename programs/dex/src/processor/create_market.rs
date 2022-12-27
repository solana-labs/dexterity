use crate::{error::DomainOrProgramResult, CreateMarketAccounts};
use anchor_lang::prelude::*;

pub fn process(
    ctx: Context<CreateMarketAccounts>,
    params: agnostic_orderbook::instruction::create_market::Params,
) -> DomainOrProgramResult {
    
    return Ok(());
}
