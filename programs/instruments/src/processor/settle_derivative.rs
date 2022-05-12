use crate::{
    error::DerivativeError,
    oracle::get_oracle_price,
    state::{
        derivative_metadata::DerivativeMetadata,
        enums::{ExpirationStatus, InstrumentType, OracleType},
    },
    SettleDerivative,
};
use anchor_lang::prelude::*;
use dex::{
    error::{DomainOrProgramError, DomainOrProgramResult, UtilError},
    state::{
        constants::{NO_ASK_PRICE, NO_BID_PRICE},
        market_product_group::MarketProductGroup,
        products::Product,
    },
    utils::{
        numeric::{Fractional, ZERO_FRAC},
        validation::{assert, assert_keys_equal},
    },
};
use solana_program::{
    entrypoint::ProgramResult, program_error::ProgramError, sysvar, sysvar::clock::Clock,
};
use std::cell::Ref;

pub fn process(ctx: Context<SettleDerivative>) -> ProgramResult {
    validate(&ctx)?;
    let accts = ctx.accounts;
    let funding_amount = get_funding_amount(accts)?;
    msg!("About to update dex funding");
    update_product_funding_in_dex(accts, funding_amount)
}

fn validate(ctx: &Context<SettleDerivative>) -> DomainOrProgramResult {
    let accts = &ctx.accounts;
    let derivative_metadata = accts.derivative_metadata.load_mut()?;
    assert(
        derivative_metadata.is_initialized(),
        DerivativeError::UninitializedAccount,
    )?;
    assert_keys_equal(derivative_metadata.clock, *accts.clock.key)?;

    assert(
        !derivative_metadata.expired(),
        DerivativeError::ContractIsExpired,
    )?;
    assert_keys_equal(
        derivative_metadata.get_key(ctx.program_id)?,
        accts.derivative_metadata.key(),
    )?;
    assert_keys_equal(derivative_metadata.price_oracle, *accts.price_oracle.key)?;
    Ok(())
}

fn get_funding_amount(
    accts: &mut SettleDerivative,
) -> std::result::Result<Fractional, ProgramError> {
    let clock: Clock = bincode::deserialize(&accts.clock.data.borrow()).map_err(|e| {
        msg!("Failed to deserialize clock {}", e);
        ProgramError::InvalidArgument
    })?;
    let mut derivative_metadata = accts.derivative_metadata.load_mut()?;
    match derivative_metadata.oracle_type {
        OracleType::Pyth => assert_keys_equal(accts.clock.key(), sysvar::clock::ID)?,
        _ => {}
    }
    let loader = AccountLoader::try_from(&accts.market_product_group)?;
    let market_product_group: Ref<MarketProductGroup> = loader.load()?;

    let index_price =
        get_oracle_price(derivative_metadata.oracle_type, &accts.price_oracle, &clock)?;
    let payoff = get_payoff(&derivative_metadata, index_price)?;

    assert(
        clock.unix_timestamp > derivative_metadata.initialization_time,
        ProgramError::from(DerivativeError::InvalidSettlementTime),
    )?;
    let elapsed = clock
        .unix_timestamp
        .saturating_sub(derivative_metadata.last_funding_time);
    if elapsed >= derivative_metadata.minimum_funding_period {
        derivative_metadata.last_funding_time = clock.unix_timestamp;
    } else {
        msg!("Contract has not reached its next funding time");
        msg!(
            "last_funding_time: {}, current time: {}, time_remaining: {}",
            derivative_metadata.last_funding_time,
            clock.unix_timestamp,
            derivative_metadata.minimum_funding_period
                - (clock.unix_timestamp - derivative_metadata.last_funding_time)
        );
        return Err(DerivativeError::InvalidSettlementTime.into());
    }
    let (_, market_product) =
        market_product_group.find_product_index(&accts.derivative_metadata.key())?;

    let funding_amount = if derivative_metadata.instrument_type.is_recurring()? {
        // Handle Everlasting Options and Perpetual Swaps
        // If mark price dips below payoff, longs get paid
        let mark_price = get_mark_price(market_product, &clock)?;
        let offset = payoff - mark_price;
        // Compute fraction of offset you should be paying
        let num = Fractional::from(elapsed);
        let denom = Fractional::from(derivative_metadata.full_funding_period);
        // Cap out the funding payment at 100%
        let mut pct = num.checked_div(denom)?.min(1.into());
        if pct.exp > 2 {
            pct = pct.round_sf(2);
        }
        let res = (offset * pct).round_sf(market_product_group.decimals as u32);
        msg!(
            "mark_price: {} index: {} offset: {} num: {} denom: {} pct: {} std::result::Result: {}",
            mark_price,
            payoff,
            offset,
            num,
            denom,
            pct,
            res
        );
        res
    } else {
        // Handle Vanilla Options and Futures
        derivative_metadata.expired = ExpirationStatus::Expired;
        msg!("payoff: {}", payoff);
        payoff.round_sf(market_product_group.decimals as u32)
    };
    Ok(funding_amount)
}

fn update_product_funding_in_dex(accts: &SettleDerivative, amount: Fractional) -> ProgramResult {
    msg!("Updating funding");
    let derivative_metadata = accts.derivative_metadata.load()?;
    let seeds: &[&[u8]] = &[
        b"derivative",
        &accts.price_oracle.key.to_bytes(),
        &accts.market_product_group.key.to_bytes(),
        &(derivative_metadata.instrument_type as u64).to_le_bytes(),
        &derivative_metadata.strike.m.to_le_bytes(),
        &derivative_metadata.strike.exp.to_le_bytes(),
        &derivative_metadata.initialization_time.to_le_bytes(),
        &derivative_metadata.full_funding_period.to_le_bytes(),
        &derivative_metadata.minimum_funding_period.to_le_bytes(),
        &[derivative_metadata.bump as u8],
    ];
    let expired = derivative_metadata.expired();
    let cpi_program = accts.dex_program.clone();
    let cpi_accounts = dex::cpi::accounts::UpdateProductFunding {
        market_product_group: accts.market_product_group.clone(),
        product: accts.derivative_metadata.to_account_info(),
    };
    dex::cpi::update_product_funding(
        CpiContext::new_with_signer(cpi_program, cpi_accounts, &[seeds]),
        dex::UpdateProductFundingParams { amount, expired },
    )?;
    msg!("Updated funding");

    Ok(())
}

fn get_payoff(
    derivative_metadata: &DerivativeMetadata,
    index_price: Fractional,
) -> std::result::Result<Fractional, DomainOrProgramError> {
    let raw_payoff = match derivative_metadata.instrument_type {
        InstrumentType::RecurringCall | InstrumentType::ExpiringCall => {
            index_price - derivative_metadata.strike
        }
        InstrumentType::RecurringPut | InstrumentType::ExpiringPut => {
            derivative_metadata.strike - index_price
        }
        _ => {
            return Err(UtilError::AccountUninitialized.into());
        }
    };
    Ok(if raw_payoff.is_negative() {
        ZERO_FRAC
    } else {
        raw_payoff
    })
}

fn get_mark_price(
    market_product: &Product,
    clock: &Clock,
) -> std::result::Result<Fractional, DomainOrProgramError> {
    // TODO: This calculation can improved to be more robust
    let best_bid = market_product.get_prev_best_bid(clock.slot);
    let best_ask = market_product.get_prev_best_ask(clock.slot);
    if best_ask == NO_ASK_PRICE || best_bid == NO_BID_PRICE {
        msg!("best_bid: {}", best_bid);
        msg!("best_ask: {}", best_ask);
        msg!("Bid or ask is empty");
        return Err(ProgramError::InvalidAccountData.into());
    }
    (best_bid + best_ask)
        .checked_div(Fractional::new(2, 0))
        .map_err(DomainOrProgramError::from)
}
