use crate::{
    error::{DexError, DomainOrProgramResult, UtilError},
    state::{
        constants::MAX_OUTRIGHTS,
        enums::ProductStatus,
        market_product_group::*,
        products::{Outright, Product, ProductMetadata},
    },
    utils::{
        numeric::{Fractional, ZERO_FRAC},
        orderbook::load_orderbook,
        validation::{assert_keys_equal, assert_with_msg},
    },
    DomainOrProgramError, InitializeMarketProduct, InitializeMarketProductParams,
};
use anchor_lang::{
    prelude::*,
    solana_program::{
        msg,
        program_pack::IsInitialized,
        pubkey::Pubkey,
        sysvar::{clock::Clock, Sysvar},
    },
};

fn validate(
    ctx: &Context<InitializeMarketProduct>,
    params: &InitializeMarketProductParams,
) -> std::result::Result<u8, DomainOrProgramError> {
    let accts = &ctx.accounts;
    let (market_authority_key, bump) =
        Pubkey::find_program_address(&[accts.product.key().as_ref()], ctx.program_id);
    msg!("seeds: {:?}", &[accts.product.as_ref()]);
    let orderbook = load_orderbook(&accts.orderbook, &market_authority_key)?;
    let market_product_group = accts.market_product_group.load()?;

    Fractional::from(orderbook.min_base_order_size as i64)
        .checked_mul(params.tick_size)?
        .round(market_product_group.decimals as u32)
        .map_err(
            |_| {
                msg!("Orderbook minimum size and product tick size are incompatible with market decimals");
                DexError::ProductDecimalPrecisionError
            }
        )?;
    assert_with_msg(
        market_product_group.active_outrights().count() < MAX_OUTRIGHTS,
        ProgramError::InvalidArgument,
        "MarketProductGroup is full",
    )?;
    if !market_product_group.is_initialized() {
        msg!("MarketProductGroup account is not initialized");
        return Err(UtilError::AccountUninitialized.into());
    }
    assert_keys_equal(accts.authority.key(), market_product_group.authority)?;
    match market_product_group.find_product_index(&accts.product.key()) {
        Ok(_) => return Err(UtilError::DuplicateProductKey.into()),
        Err(_) => {}
    }
    Ok(bump)
}

pub fn process(
    ctx: Context<InitializeMarketProduct>,
    params: InitializeMarketProductParams,
) -> DomainOrProgramResult {
    let bump = validate(&ctx, &params)?;
    let accts = ctx.accounts;
    let mut market_product_group = accts.market_product_group.load_mut()?;
    let mut market_product = Outright {
        metadata: ProductMetadata {
            bump: bump as u64,
            product_key: accts.product.key(),
            name: params.name,
            orderbook: *accts.orderbook.key,
            contract_volume: ZERO_FRAC,
            // Negative+Fractional Price
            tick_size: params.tick_size,
            base_decimals: params.base_decimals,
            price_offset: params.price_offset,
            prices: PriceEwma::default(),
        },
        product_status: ProductStatus::Initialized,
        num_queue_events: 0,
        dust: ZERO_FRAC,
        cum_funding_per_share: ZERO_FRAC,
        cum_social_loss_per_share: ZERO_FRAC,
        open_long_interest: ZERO_FRAC,
        open_short_interest: ZERO_FRAC,
        padding: Default::default(),
    };
    market_product.prices.initialize(Clock::get()?.slot);
    market_product_group.add_product(Product::Outright {
        outright: market_product,
    })?;
    market_product_group.sequence_number += 1;
    msg!("sequence: {}", market_product_group.sequence_number);
    accts.market_product_group.key().log();
    Ok(())
}
