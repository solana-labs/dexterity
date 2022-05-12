use crate::{
    error::{DomainOrProgramError, DomainOrProgramResult},
    state::{
        constants::{NO_ASK_PRICE, NO_BID_PRICE},
        market_product_group::PriceEwma,
    },
    utils::numeric::Fractional,
};
use agnostic_orderbook::{
    critbit::{NodeHandle, Slab},
    state::{MarketState, Side},
};
use anchor_lang::solana_program::{
    account_info::AccountInfo, clock::Clock, msg, program_error::ProgramError, pubkey::Pubkey,
};

pub const EWMA_ROUND: u32 = 2;

pub fn load_orderbook(
    account: &AccountInfo,
    market_signer: &Pubkey,
) -> std::result::Result<MarketState, DomainOrProgramError> {
    let orderbook_state = MarketState::get(account)?;
    if orderbook_state.tag != agnostic_orderbook::state::AccountTag::Market as u64 {
        msg!("Invalid orderbook");
        return Err(ProgramError::InvalidArgument.into());
    }
    if &orderbook_state.caller_authority != &market_signer.to_bytes() {
        msg!("The provided orderbook isn't owned by the market signer.");
        return Err(ProgramError::InvalidArgument.into());
    }
    Ok(*orderbook_state)
}

pub fn get_bbo(
    node: Option<NodeHandle>,
    book: &Slab,
    side: Side,
    tick_size: Fractional,
    price_offset: Fractional,
) -> std::result::Result<Fractional, DomainOrProgramError> {
    match node {
        Some(nh) => {
            let leaf_node = book.get_node(nh).unwrap().as_leaf().unwrap().to_owned();
            let price_aob = leaf_node.price();
            let price_dex = Fractional::new((price_aob >> 32) as i64, 0)
                .checked_mul(tick_size)?
                .checked_sub(price_offset)?;
            Ok(price_dex)
        }
        None => match side {
            Side::Bid => Ok(NO_BID_PRICE),
            Side::Ask => Ok(NO_ASK_PRICE),
        },
    }
}

pub fn update_prices(
    clock: &Clock,
    prices: &mut PriceEwma,
    bid_price: Fractional,
    ask_price: Fractional,
    windows: &[u64],
) -> DomainOrProgramResult {
    let curr_slot = clock.slot;
    let prev_slot = prices.slot;
    let slots_elapsed = Fractional::from((curr_slot - prev_slot) as i64).round(4)?;
    if curr_slot > prev_slot {
        apply_ewma_transform(&mut prices.ewma_bid, windows, prices.bid, slots_elapsed)?;
        apply_ewma_transform(&mut prices.ewma_ask, windows, prices.ask, slots_elapsed)?;
        prices.prev_bid = prices.bid;
        prices.prev_ask = prices.ask;
    } else {
        if prices.bid == NO_BID_PRICE {
            apply_ewma_transform(&mut prices.ewma_bid, windows, bid_price, slots_elapsed)?;
        }
        if prices.ask == NO_ASK_PRICE {
            apply_ewma_transform(&mut prices.ewma_ask, windows, ask_price, slots_elapsed)?;
        }
    }
    prices.bid = bid_price;
    prices.ask = ask_price;
    prices.slot = curr_slot;
    Ok(())
}

fn apply_ewma_transform(
    ewma: &mut [Fractional],
    windows: &[u64],
    curr_price: Fractional,
    slots_elapsed: Fractional,
) -> DomainOrProgramResult {
    if curr_price == NO_BID_PRICE || curr_price == NO_ASK_PRICE {
        return Ok(());
    }
    for i in 0..windows.len() {
        if ewma[i] == NO_BID_PRICE || ewma[i] == NO_ASK_PRICE {
            ewma[i] = curr_price;
            continue;
        }
        let window = windows[i];
        let x = -slots_elapsed
            .checked_div(Fractional::new(window as i64, 0))?
            .round_sf(EWMA_ROUND);
        let weight = x.exp()?.round_sf(EWMA_ROUND);
        let prev = weight.saturating_mul(ewma[i]);
        let curr = (Fractional::new(1, 0).checked_sub(weight)?).saturating_mul(curr_price);
        ewma[i] = prev.saturating_add(curr);
    }
    Ok(())
}
