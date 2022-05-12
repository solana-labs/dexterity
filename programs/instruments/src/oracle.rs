use crate::{error::DerivativeError, state::enums::OracleType};
use borsh::BorshDeserialize;
use dex::{error::UtilError, utils::numeric::Fractional};
use dummy_oracle::state::OraclePrice;
use solana_program::{
    account_info::AccountInfo, clock::Clock, entrypoint::ProgramResult, msg,
    program_error::ProgramError, pubkey::Pubkey,
};

pub fn validate_pyth_accounts(
    pyth_product_info: &AccountInfo,
    pyth_price_info: &AccountInfo,
) -> ProgramResult {
    let pyth_product_data = pyth_product_info.try_borrow_data()?;
    let pyth_product = pyth_client::cast::<pyth_client::Product>(&pyth_product_data);
    if pyth_product.magic != pyth_client::MAGIC {
        msg!("Pyth product account provided is not a valid Pyth account");
        return Err(ProgramError::InvalidArgument);
    }
    if pyth_product.atype != pyth_client::AccountType::Product as u32 {
        msg!("Pyth product account provided is not a valid Pyth product account");
        return Err(ProgramError::InvalidArgument);
    }
    if pyth_product.ver != pyth_client::VERSION_2 {
        msg!("Pyth product account provided has a different version than the Pyth client");
        return Err(ProgramError::InvalidArgument);
    }
    if !pyth_product.px_acc.is_valid() {
        msg!("Pyth product price account is invalid");
        return Err(ProgramError::InvalidArgument);
    }
    let pyth_price_pubkey = Pubkey::new(&pyth_product.px_acc.val);
    if &pyth_price_pubkey != pyth_price_info.key {
        msg!("Pyth product price account does not match the Pyth price provided");
        return Err(ProgramError::InvalidArgument);
    }
    Ok(())
}

pub fn get_pyth_price(
    pyth_price_info: &AccountInfo,
    _clock: &Clock,
) -> std::result::Result<Fractional, ProgramError> {
    let pyth_price_data = &pyth_price_info.try_borrow_data()?;
    let pyth_price = pyth_client::cast::<pyth_client::Price>(pyth_price_data);
    if pyth_price.agg.price < 0 {
        msg!("Oracle price cannot be negative");
        return Err(DerivativeError::InvalidOracleConfig.into());
    }
    let price = pyth_price.agg.price;
    let conf = pyth_price.agg.conf;
    if price > 0 {
        let pct = Fractional::new(conf as i64, 0) / Fractional::new(price, 0);
        if pct > Fractional::new(1, 1) {
            msg!("Market is too wide");
            return Err(DerivativeError::InvalidOracleConfig.into());
        }
    }
    Ok(Fractional::new(price, pyth_price.expo.abs() as u64))
}

pub fn get_dummy_price(
    price_info: &AccountInfo,
    _clock: &Clock,
) -> std::result::Result<Fractional, ProgramError> {
    let price_data = OraclePrice::try_from_slice(&price_info.data.borrow_mut())?;
    Ok(Fractional::new(price_data.price, price_data.decimals))
}

pub fn get_oracle_price(
    oracle_type: OracleType,
    price_info: &AccountInfo,
    clock: &Clock,
) -> std::result::Result<Fractional, ProgramError> {
    match oracle_type {
        OracleType::Pyth => get_pyth_price(price_info, clock),
        OracleType::Dummy => get_dummy_price(price_info, clock),
        _ => Err(UtilError::AccountUninitialized.into()),
    }
}
