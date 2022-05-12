use crate::{error::DerivativeError, state::enums::AccountTag, CloseDerivativeAccount};
use anchor_lang::prelude::*;
use dex::{
    error::UtilError,
    utils::validation::{assert, assert_keys_equal},
};
use solana_program::entrypoint::ProgramResult;

pub fn process(ctx: Context<CloseDerivativeAccount>) -> ProgramResult {
    let accts = ctx.accounts;
    let mut derivative_metadata = accts.derivative_metadata.load_mut()?;
    assert_keys_equal(*accts.derivative_metadata.as_ref().owner, *ctx.program_id)?;
    assert(
        derivative_metadata.is_initialized(),
        UtilError::AccountUninitialized,
    )?;
    assert_keys_equal(
        *accts.close_authority.key,
        derivative_metadata.close_authority,
    )?;
    assert_keys_equal(
        derivative_metadata.get_key(ctx.program_id)?,
        accts.derivative_metadata.key(),
    )?;
    assert(
        derivative_metadata.expired(),
        DerivativeError::CannotBeDeleted,
    )?;
    let dest_starting_lamports = accts.destination.lamports();
    **accts.destination.lamports.borrow_mut() = dest_starting_lamports
        .checked_add(accts.derivative_metadata.as_ref().lamports())
        .ok_or(DerivativeError::NumericalOverflow)?;
    **accts.derivative_metadata.as_ref().lamports.borrow_mut() = 0;
    derivative_metadata.tag = AccountTag::Uninitialized;
    Ok(())
}
