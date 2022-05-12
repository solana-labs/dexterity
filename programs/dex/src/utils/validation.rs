use crate::{
    error::{DexError, DomainOrProgramResult, UtilError},
    utils::numeric::Fractional,
    DomainOrProgramError,
};
use anchor_lang::solana_program::{
    account_info::AccountInfo,
    msg,
    program_error::ProgramError,
    program_pack::{IsInitialized, Pack},
    pubkey::Pubkey,
    sysvar::rent::Rent,
};
use spl_associated_token_account::get_associated_token_address;
use spl_token::state::Account;

#[inline(always)]
pub fn get_rent(rent: &Rent, size: u64, account_info: &AccountInfo) -> u64 {
    rent.minimum_balance(size as usize)
        .max(1)
        .saturating_sub(account_info.lamports())
}

#[inline(always)]
pub fn assert_is_ata(ata: &AccountInfo, wallet: &Pubkey, mint: &Pubkey) -> DomainOrProgramResult {
    assert_owned_by(ata, &spl_token::id())?;
    let ata_account: Account = assert_initialized(ata)?;
    assert_keys_equal(ata_account.owner, *wallet)?;
    assert_keys_equal(get_associated_token_address(wallet, mint), *ata.key)?;
    Ok(())
}

#[inline(always)]
pub fn assert_signer(account_info: &AccountInfo) -> DomainOrProgramResult {
    if !account_info.is_signer {
        msg!("Account must be signer.");
        Err(ProgramError::MissingRequiredSignature.into())
    } else {
        Ok(())
    }
}

#[track_caller]
#[inline(always)]
pub fn assert_rent_exempt(rent: &Rent, account_info: &AccountInfo) -> DomainOrProgramResult {
    if !rent.is_exempt(account_info.lamports(), account_info.data_len()) {
        let caller = std::panic::Location::caller();
        msg!("Account must rent exempt. \n{}", caller);
        Err(UtilError::NotRentExempt.into())
    } else {
        Ok(())
    }
}

#[track_caller]
#[inline(always)]
pub fn assert(v: bool, err: impl Into<DomainOrProgramError>) -> DomainOrProgramResult {
    assert_with_msg(v, err, "Assertion failed.")
}

#[track_caller]
#[inline(always)]
pub fn assert_with_msg(
    v: bool,
    err: impl Into<DomainOrProgramError>,
    msg: &str,
) -> DomainOrProgramResult {
    if !v {
        let caller = std::panic::Location::caller();
        msg!("{}. \n{}", msg, caller);
        Err(err.into())
    } else {
        Ok(())
    }
}

#[track_caller]
#[inline(always)]
pub fn assert_equal<T: PartialEq>(
    v1: T,
    v2: T,
    err: impl Into<DomainOrProgramError>,
) -> DomainOrProgramResult {
    assert_with_msg(v1 == v2, err, "Assertion failed.")
}

#[track_caller]
#[inline(always)]
pub fn assert_keys_equal(key1: Pubkey, key2: Pubkey) -> DomainOrProgramResult {
    if key1 != key2 {
        let caller = std::panic::Location::caller();
        msg!("Public Keys do not match {} {}. \n{}", key1, key2, caller,);
        Err(UtilError::PublicKeyMismatch.into())
    } else {
        Ok(())
    }
}

#[inline(always)]
pub fn check_funds(funds: Fractional) -> DomainOrProgramResult {
    if (funds.round(0)? != funds) || (funds.m < 0) {
        return Err(DexError::FundsError.into());
    }
    Ok(())
}

#[track_caller]
#[inline(always)]
pub fn assert_keys_unequal(key1: Pubkey, key2: Pubkey) -> DomainOrProgramResult {
    if key1 == key2 {
        let caller = std::panic::Location::caller();
        msg!(
            "Public Keys are eqaual when they should not be {} {}. \n{}",
            key1,
            key2,
            caller,
        );
        Err(UtilError::PublicKeysShouldBeUnique.into())
    } else {
        Ok(())
    }
}

#[track_caller]
#[inline(always)]
pub fn assert_initialized<T: Pack + IsInitialized>(
    account_info: &AccountInfo,
) -> std::result::Result<T, ProgramError> {
    let account: T = T::unpack_unchecked(&account_info.data.borrow())?;
    if !account.is_initialized() {
        let caller = std::panic::Location::caller();
        msg!(
            "Account {} is not initialized. \n{}",
            account_info.key,
            caller
        );
        Err(UtilError::AccountUninitialized.into())
    } else {
        Ok(account)
    }
}

#[inline(always)]
pub fn assert_valid_token_account_owner(
    account_info: &AccountInfo,
    owner: &Pubkey,
) -> DomainOrProgramResult {
    let account: spl_token::state::Account =
        spl_token::state::Account::unpack_unchecked(&account_info.data.borrow())?;
    if account.owner != *owner {
        msg!("Wallet account is not owned by the user");
        Err(UtilError::IncorrectOwner.into())
    } else {
        Ok(())
    }
}

#[inline(always)]
pub fn assert_owned_by(account: &AccountInfo, owner: &Pubkey) -> DomainOrProgramResult {
    if account.owner != owner {
        let caller = std::panic::Location::caller();
        msg!(
            "Account owner does not match expected: {} actual: {}. \n{}",
            account.owner,
            owner,
            caller,
        );
        Err(UtilError::IncorrectOwner.into())
    } else {
        Ok(())
    }
}
