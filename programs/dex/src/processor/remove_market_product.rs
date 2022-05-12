use anchor_lang::{
    prelude::*,
    solana_program::{msg, program::invoke_signed, program_pack::IsInitialized},
};
use bonfida_utils::InstructionsAccount;

use crate::{
    error::{DexError, DomainOrProgramResult, UtilError},
    state::products::Product,
    utils::validation::{assert, assert_keys_equal},
    DomainOrProgramError, RemoveMarketProduct,
};

fn validate(ctx: &Context<RemoveMarketProduct>) -> std::result::Result<u8, DomainOrProgramError> {
    let accts = &ctx.accounts;
    let market_product_group = accts.market_product_group.load()?;
    if !market_product_group.is_initialized() {
        msg!("MarketProductGroup account is not initialized");
        return Err(UtilError::AccountUninitialized.into());
    }
    assert_keys_equal(accts.authority.key(), market_product_group.authority)?;
    let (_, product) = market_product_group.find_product_index(&accts.product.key())?;

    // Validation
    assert_keys_equal(product.orderbook, accts.orderbook.key())?;
    assert(
        market_product_group.is_expired(product),
        DexError::ContractIsNotExpired,
    )?;
    match product {
        Product::Outright { outright: o } => {
            assert(o.is_removable(), DexError::ContractIsNotExpired)?;
            assert(
                !market_product_group
                    .active_combos()
                    .any(|(_, combo)| combo.has_leg(accts.product.key())),
                DexError::CombosNotRemoved,
            )?;
        }
        Product::Combo { combo: _ } => {}
    }
    Ok(product.bump as u8)
}

pub fn process(ctx: Context<RemoveMarketProduct>) -> DomainOrProgramResult {
    let product_bump = validate(&ctx)?;
    let accts = ctx.accounts;

    let close_market_instruction = agnostic_orderbook::instruction::close_market::Accounts {
        market: accts.orderbook.key,
        event_queue: accts.event_queue.key,
        bids: accts.bids.key,
        asks: accts.asks.key,
        authority: accts.market_signer.key,
        lamports_target_account: accts.authority.key,
    }
    .get_instruction(
        accts.aaob_program.key(),
        agnostic_orderbook::instruction::AgnosticOrderbookInstruction::CloseMarket as u8,
        agnostic_orderbook::instruction::close_market::Params {},
    );
    invoke_signed(
        &close_market_instruction,
        &[
            accts.aaob_program.to_account_info(),
            accts.orderbook.to_account_info(),
            accts.market_signer.to_account_info(),
            accts.event_queue.to_account_info(),
            accts.bids.to_account_info(),
            accts.asks.to_account_info(),
            accts.authority.to_account_info(),
        ],
        &[&[accts.product.key.as_ref(), &[product_bump]]],
    )?;
    let mut market_product_group = accts.market_product_group.load_mut()?;
    market_product_group.deactivate_product(accts.product.key())?;
    market_product_group.sequence_number += 1;
    msg!("sequence: {}", market_product_group.sequence_number);
    accts.market_product_group.key().log();
    Ok(())
}
