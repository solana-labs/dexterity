use crate::{
    error::DomainOrProgramResult, utils::validation::assert_keys_equal, ChooseSuccessor,
    ClaimAuthority,
};
use anchor_lang::prelude::*;

pub fn choose_successor(ctx: Context<ChooseSuccessor>) -> DomainOrProgramResult {
    let accts = ctx.accounts;
    let mut market_product_group = accts.market_product_group.load_mut()?;
    assert_keys_equal(market_product_group.authority, *accts.authority.key)?;
    market_product_group.successor = *accts.new_authority.key;
    market_product_group.sequence_number += 1;
    msg!("sequence: {}", market_product_group.sequence_number);
    accts.market_product_group.key().log();
    Ok(())
}

pub fn claim_authority(ctx: Context<ClaimAuthority>) -> DomainOrProgramResult {
    let accts = ctx.accounts;
    let mut market_product_group = accts.market_product_group.load_mut()?;
    assert_keys_equal(market_product_group.successor, *accts.new_authority.key)?;
    market_product_group.authority = *accts.new_authority.key;
    market_product_group.sequence_number += 1;
    msg!("sequence: {}", market_product_group.sequence_number);
    accts.market_product_group.key().log();
    Ok(())
}
