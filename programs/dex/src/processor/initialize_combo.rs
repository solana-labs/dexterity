use anchor_lang::{
    prelude::*,
    solana_program::{
        entrypoint::ProgramResult,
        msg,
        program_error::ProgramError,
        program_pack::IsInitialized,
        pubkey::Pubkey,
        sysvar::{clock::Clock, Sysvar},
    },
};
use num::Integer;

use crate::{
    error::{DomainOrProgramResult, UtilError},
    state::{
        constants::MAX_LEGS,
        market_product_group::*,
        products::{Combo, Leg, Product, ProductMetadata},
    },
    utils::{
        numeric::ZERO_FRAC,
        orderbook::load_orderbook,
        validation::{assert, assert_keys_equal},
    },
    InitializeCombo, InitializeComboParams,
};

fn validate(
    ctx: &Context<InitializeCombo>,
    params: &InitializeComboParams,
) -> DomainOrProgramResult {
    let accts = &ctx.accounts;
    let market_product_group = accts.market_product_group.load()?;
    if !market_product_group.is_initialized() {
        msg!("MarketProductGroup account is not initialized");
        return Err(UtilError::AccountUninitialized.into());
    }
    assert_keys_equal(accts.authority.key(), market_product_group.authority)?;
    // Checks that the list of products is in strict lexicographic order by public key
    assert(
        ctx.remaining_accounts
            .windows(2)
            .all(|w| *w[0].key < *w[1].key),
        ProgramError::InvalidAccountData,
    )?;
    assert_valid_ratios(&params.ratios)?;
    if params.ratios.len() != ctx.remaining_accounts.len() {
        msg!("Ratios and products have different lengths");
        return Err(ProgramError::InvalidAccountData.into());
    }
    Ok(())
}

pub fn process(
    ctx: Context<InitializeCombo>,
    params: InitializeComboParams,
) -> DomainOrProgramResult {
    validate(&ctx, &params)?;
    let accts = ctx.accounts;

    let mut market_product_group = accts.market_product_group.load_mut()?;
    let mut legs = [Leg::default(); MAX_LEGS];
    let mut seeds = Vec::with_capacity(params.ratios.len() * 34);
    for (i, (ratio, product)) in params.ratios.iter().zip(ctx.remaining_accounts).enumerate() {
        let (product_index, _) = market_product_group.find_outright(product.key)?;
        legs[i] = Leg {
            product_index,
            product_key: *product.key,
            ratio: *ratio as i64,
        };
        seeds.extend(product.key.to_bytes().iter());
    }

    for ratio in params.ratios.iter() {
        seeds.extend(ratio.to_le_bytes().iter());
    }
    // Format of the seeds is [product_key_1, ..., product_key_N, [ratio_1, ..., ratio_N]]
    let (product_key, _) =
        Pubkey::find_program_address(&seeds.chunks(32).collect::<Vec<&[u8]>>(), ctx.program_id);
    let (market_authority_key, bump) =
        Pubkey::find_program_address(&[product_key.as_ref()], ctx.program_id);
    let _orderbook = load_orderbook(accts.orderbook.as_ref(), &market_authority_key)?;

    let mut product = Product::Combo {
        combo: Combo {
            metadata: ProductMetadata {
                bump: bump as u64,
                product_key,
                name: params.name,
                orderbook: *accts.orderbook.key,
                tick_size: params.tick_size,
                base_decimals: params.base_decimals,
                price_offset: params.price_offset,
                contract_volume: ZERO_FRAC,
                prices: PriceEwma::default(),
            },
            num_legs: params.ratios.len(),
            legs,
        },
    };
    product.prices.initialize(Clock::get()?.slot);
    market_product_group.add_product(product)?;
    market_product_group.sequence_number += 1;
    msg!("sequence: {}", market_product_group.sequence_number);
    accts.market_product_group.key().log();
    Ok(())
}

fn assert_valid_ratios(ratios: &Vec<i8>) -> ProgramResult {
    if ratios.len() < 2 {
        msg!("Combo must have at least 2 legs");
        return Err(ProgramError::InvalidAccountData);
    }
    let mut gcd: i8 = -1;
    for item in ratios.iter() {
        if gcd != -1 {
            gcd = item.gcd(&gcd);
        } else {
            gcd = *item;
        }
    }
    if gcd != 1 {
        msg!("Leg ratios have not been fully reduced");
        return Err(ProgramError::InvalidAccountData);
    }
    Ok(())
}
