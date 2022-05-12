use std::ops::{Deref, DerefMut};

use anchor_lang::{
    prelude::*,
    solana_program::{
        entrypoint::ProgramResult, program_error::ProgramError, program_pack::IsInitialized,
        pubkey::Pubkey,
    },
};
use bytemuck::{Pod, Zeroable};
use itertools::Itertools;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_big_array::BigArray;

use crate::{
    error::{DexError, DomainOrProgramError, DomainOrProgramResult, UtilError},
    state::{
        constants::*,
        enums::*,
        products::{Combo, Outright, Product, ProductMetadata},
    },
    utils::{
        bitset::Bitset,
        loadable::Loadable,
        numeric::{Fractional, ZERO_FRAC},
        validation::assert,
        TwoIterators,
    },
};

/// The highest level organizational unit of the Dex.
/// Market product groups exist independently of each other.
/// i.e. each trader, product etc, corresponds to exactly one market product group.
#[account(zero_copy)]
#[derive(AnchorSerialize, Deserialize, Serialize)] // serde
pub struct MarketProductGroup {
    // TODO: add aaob program id
    pub tag: AccountTag,
    pub name: [u8; NAME_LEN],
    pub authority: Pubkey,
    // The future authority of the MarketProductGroup
    pub successor: Pubkey,
    pub vault_mint: Pubkey,
    pub collected_fees: Fractional,
    pub fee_collector: Pubkey,
    pub decimals: u64,
    pub risk_engine_program_id: Pubkey,
    pub fee_model_program_id: Pubkey,
    pub fee_model_configuration_acct: Pubkey,
    pub risk_model_configuration_acct: Pubkey,
    pub active_flags_products: Bitset,
    pub ewma_windows: [u64; 4],
    pub market_products: ProductArray,
    pub vault_bump: u16,
    pub risk_and_fee_bump: u16,
    pub find_fees_discriminant_len: u16,
    pub validate_account_discriminant_len: u16,
    pub find_fees_discriminant: [u8; 8],
    pub validate_account_health_discriminant: [u8; 8],
    pub validate_account_liquidation_discriminant: [u8; 8],
    pub create_risk_state_account_discriminant: [u8; 8],
    pub max_maker_fee_bps: i16,
    pub min_maker_fee_bps: i16,
    pub max_taker_fee_bps: i16,
    pub min_taker_fee_bps: i16,
    pub fee_output_register: Pubkey,
    pub risk_output_register: Pubkey,
    pub sequence_number: u128,
}

impl Default for MarketProductGroup {
    fn default() -> Self {
        unsafe { std::mem::zeroed() }
    }
}

impl IsInitialized for MarketProductGroup {
    fn is_initialized(&self) -> bool {
        match self.tag {
            AccountTag::MarketProductGroup | AccountTag::MarketProductGroupWithCombos => true,
            _ => false,
        }
    }
}

impl MarketProductGroup {
    pub fn is_expired(&self, product: &Product) -> bool {
        match product {
            Product::Outright { outright: o } => o.is_expired(),
            Product::Combo { combo: c } => c.legs().iter().any(|l| {
                self.market_products[l.product_index]
                    .try_to_outright()
                    .unwrap()
                    .is_expired()
            }),
        }
    }

    // Finds index corresponding to product key
    pub fn find_product_index(
        &self,
        product_key: &Pubkey,
    ) -> DomainOrProgramResult<(usize, &Product)> {
        self.active_products()
            .find(|(_, prod)| &prod.product_key == product_key)
            .ok_or(DexError::MissingMarketProduct.into())
    }

    pub fn find_outright(&self, product_key: &Pubkey) -> DomainOrProgramResult<(usize, &Outright)> {
        let (idx, p) = self.find_product_index(product_key)?;
        Ok((idx, p.try_to_outright()?))
    }

    pub fn find_combo(&self, product_key: &Pubkey) -> DomainOrProgramResult<(usize, &Combo)> {
        let (idx, p) = self.find_product_index(product_key)?;
        Ok((idx, p.try_to_combo()?))
    }

    pub fn active_products(&self) -> impl Iterator<Item = (usize, &Product)> {
        self.market_products
            .iter()
            .enumerate()
            .filter(|(idx, _)| self.active_flags_products.contains(*idx))
    }

    pub fn active_outrights(&self) -> impl Iterator<Item = (usize, &Outright)> {
        self.active_products()
            .filter_map(|(idx, prod)| Some((idx, prod.try_to_outright().ok()?)))
    }

    pub fn active_combos(&self) -> impl Iterator<Item = (usize, &Combo)> {
        self.active_products()
            .filter_map(|(idx, prod)| Some((idx, prod.try_to_combo().ok()?)))
    }

    pub fn deactivate_product(&mut self, key: Pubkey) -> DomainOrProgramResult {
        // todo: handle if Outright has Combos that reference it
        let (index, _) = self.find_product_index(&key)?;
        self.active_flags_products.remove(index)?;
        self.market_products[index] = Default::default();
        Ok(())
    }

    pub fn add_product(&mut self, product: Product) -> DomainOrProgramResult {
        assert(
            self.active_products().all(|(_, p)| p.name != product.name),
            DexError::DuplicateProductNameError,
        )?;
        let idx = self
            .active_flags_products
            .find_idx_and_insert()
            .map_err(|_| DexError::FullMarketProductGroup)?;
        self.market_products[idx] = product;
        Ok(())
    }

    pub fn get_prices(&mut self, product_idx: usize) -> &mut PriceEwma {
        &mut self.market_products[product_idx].prices
    }

    pub fn get_find_fees_discriminant(&self) -> Vec<u8> {
        self.find_fees_discriminant[..self.find_fees_discriminant_len as usize].to_vec()
    }

    pub fn get_validate_account_health_discriminant(&self) -> Vec<u8> {
        self.validate_account_health_discriminant[..self.validate_account_discriminant_len as usize]
            .to_vec()
    }

    pub fn get_validate_account_liquidation_discriminant(&self) -> Vec<u8> {
        self.validate_account_liquidation_discriminant
            [..self.validate_account_discriminant_len as usize]
            .to_vec()
    }
}

#[zero_copy]
#[derive(
    Pod, Default, Debug, Eq, PartialEq, AnchorSerialize, AnchorDeserialize, Serialize, Deserialize,
)]
pub struct PriceEwma {
    pub ewma_bid: [Fractional; 4],
    pub ewma_ask: [Fractional; 4],
    pub bid: Fractional,
    pub ask: Fractional,
    pub slot: u64,
    pub prev_bid: Fractional,
    pub prev_ask: Fractional,
}

unsafe impl Zeroable for PriceEwma {}

impl PriceEwma {
    pub fn initialize(&mut self, slot: u64) {
        self.slot = slot;
        for ewma in self.ewma_bid.iter_mut() {
            *ewma = NO_BID_PRICE;
        }
        for ewma in self.ewma_ask.iter_mut() {
            *ewma = NO_ASK_PRICE;
        }
        self.bid = NO_BID_PRICE;
        self.ask = NO_ASK_PRICE;
        self.prev_bid = NO_BID_PRICE;
        self.prev_ask = NO_ASK_PRICE;
    }
}

#[account(zero_copy)]
#[derive(AnchorSerialize, Serialize, Deserialize)]
#[repr(transparent)]
pub struct ProductArray {
    #[serde(with = "BigArray")]
    pub array: [Product; 256],
}

impl Deref for ProductArray {
    type Target = [Product; 256];

    fn deref(&self) -> &Self::Target {
        &self.array
    }
}

impl DerefMut for ProductArray {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.array
    }
}
