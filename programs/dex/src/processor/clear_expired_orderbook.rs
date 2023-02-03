use anchor_lang::{
    prelude::*,
    solana_program::{msg, program::invoke_signed, program_pack::IsInitialized},
};
use bonfida_utils::InstructionsAccount;

use agnostic_orderbook::state::{critbit::Slab, AccountTag};

use crate::{
    error::{DexError, DomainOrProgramResult, UtilError},
    state::{callback_info::CallBackInfoDex, products::Product},
    utils::{
        orderbook::load_orderbook,
        validation::{assert, assert_keys_equal},
    },
    ClearExpiredOrderbook, ClearExpiredOrderbookParams, DomainOrProgramError,
};

fn validate(accts: &ClearExpiredOrderbook) -> std::result::Result<Product, DomainOrProgramError> {
    let market_product_group = accts.market_product_group.load()?;
    assert(
        market_product_group.is_initialized(),
        UtilError::AccountUninitialized,
    )?;
    // TODO validate aaob programId

    let (_, product) = market_product_group.find_product_index(&accts.product.key())?;

    assert_keys_equal(product.orderbook, accts.orderbook.key())?;
    assert(
        market_product_group.is_expired(product),
        DexError::ContractIsNotExpired,
    )?;
    Ok(*product)
}

pub fn process(
    ctx: Context<ClearExpiredOrderbook>,
    params: ClearExpiredOrderbookParams,
) -> DomainOrProgramResult {
    let accts = ctx.accounts;
    let ClearExpiredOrderbookParams {
        num_orders_to_cancel,
    } = params;
    validate(accts)?;

    let mut num_orders_cancelled: u8 = 0;
    while num_orders_to_cancel > num_orders_cancelled {
        let mut bids = accts.bids.data.borrow_mut();
        let mut asks = accts.asks.data.borrow_mut();
        let bids = Slab::<CallBackInfoDex>::from_buffer(&mut bids, AccountTag::Bids)?;
        let asks = Slab::<CallBackInfoDex>::from_buffer(&mut asks, AccountTag::Asks)?;

        let (book, handle) = if bids.root().is_some() {
            (&bids, bids.find_max())
        } else {
            (&asks, asks.find_min())
        };
        match handle {
            Some(nh) => {
                // msg!("Attempting to cancel order: {}", nh);
                let leaf_node = book.leaf_nodes[nh as usize];
                let order_id = leaf_node.key;
                let cancel_order_accounts =
                    agnostic_orderbook::instruction::cancel_order::Accounts {
                        market: &accts.orderbook,
                        event_queue: &accts.event_queue,
                        bids: &accts.bids,
                        asks: &accts.asks,
                    };

                let cancel_order_params =
                    agnostic_orderbook::instruction::cancel_order::Params { order_id };

                let _order_summary =
                    match agnostic_orderbook::instruction::cancel_order::process::<CallBackInfoDex>(
                        &ctx.program_id,
                        cancel_order_accounts,
                        cancel_order_params,
                    ) {
                        Err(error) => {
                            return Err(DomainOrProgramError::ProgramErr(error));
                        }
                        Ok(s) => s,
                    };

                //     .get_instruction(
                //         accts.aaob_program.key(),
                //         agnostic_orderbook::instruction::AgnosticOrderbookInstruction::CancelOrder
                //             as u8,
                //         agnostic_orderbook::instruction::cancel_order::Params { order_id },
                //     );
                // let cancel_order_instruction = agnostic_orderbook::instruction::cancel_order(
                //     cancel_order_accounts,
                //     accts.register.key(),
                //     agnostic_orderbook::instruction::cancel_order::Params { order_id },
                // );
                // invoke_signed(
                //     &cancel_order_instruction,
                //     &[
                //         accts.aaob_program.clone(),
                //         accts.orderbook.clone(),
                //         accts.market_signer.clone(),
                //         accts.event_queue.clone(),
                //         accts.bids.clone(),
                //         accts.asks.clone(),
                //     ],
                //     &[&[accts.product.key.as_ref(), &[product.bump as u8]]],
                // )?;
            }
            None => break,
        }
        num_orders_cancelled += 1;
    }
    if num_orders_cancelled == 0 {
        return Err(DexError::OrderbookIsEmptyError.into());
    }
    Ok(())
}
