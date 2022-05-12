use anchor_lang::{
    prelude::*,
    solana_program::{msg, program_pack::IsInitialized},
};

use crate::{
    error::{DexError, DomainOrProgramResult, UtilError},
    state::enums::ProductStatus,
    utils::validation::assert,
    UpdateProductFunding, UpdateProductFundingParams,
};

pub fn process(
    ctx: Context<UpdateProductFunding>,
    params: UpdateProductFundingParams,
) -> DomainOrProgramResult {
    let accts = ctx.accounts;
    let mut market_product_group = accts.market_product_group.load_mut()?;
    assert(
        market_product_group.is_initialized(),
        UtilError::AccountUninitialized,
    )?;

    let (idx, _) = market_product_group.find_product_index(&accts.product.key())?;
    let cash_decimals = market_product_group.decimals;
    let product = market_product_group.market_products[idx].try_to_outright_mut()?;
    product.apply_new_funding(params.amount, cash_decimals)?;

    if params.expired {
        product.product_status = ProductStatus::Expired;
    }
    market_product_group.sequence_number += 1;
    msg!("sequence: {}", market_product_group.sequence_number);
    accts.market_product_group.key().log();
    Ok(())
}
