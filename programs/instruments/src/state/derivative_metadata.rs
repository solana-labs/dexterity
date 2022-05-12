use crate::state::enums::{AccountTag, ExpirationStatus, InstrumentType, OracleType};
use anchor_lang::prelude::*;
use dex::utils::numeric::Fractional;
use solana_program::{clock::UnixTimestamp, program_error::ProgramError, pubkey::Pubkey};

#[account(zero_copy)]
pub struct DerivativeMetadata {
    pub tag: AccountTag,
    pub expired: ExpirationStatus,
    pub oracle_type: OracleType,
    pub instrument_type: InstrumentType,
    pub bump: u64,
    pub strike: Fractional,
    pub initialization_time: UnixTimestamp,
    pub full_funding_period: UnixTimestamp,
    pub minimum_funding_period: UnixTimestamp,
    pub price_oracle: Pubkey,
    pub market_product_group: Pubkey,
    pub close_authority: Pubkey,
    pub clock: Pubkey,
    pub last_funding_time: UnixTimestamp,
}

impl DerivativeMetadata {
    pub fn get_key(&self, program_id: &Pubkey) -> std::result::Result<Pubkey, ProgramError> {
        let seeds = &[
            b"derivative",
            self.price_oracle.as_ref(),
            self.market_product_group.as_ref(),
            &(self.instrument_type as u64).to_le_bytes(),
            &self.strike.m.to_le_bytes(),
            &self.strike.exp.to_le_bytes(),
            &self.initialization_time.to_le_bytes(),
            &self.full_funding_period.to_le_bytes(),
            &self.minimum_funding_period.to_le_bytes(),
            &[self.bump as u8],
        ];
        Ok(Pubkey::create_program_address(seeds, program_id)?)
    }

    pub fn is_initialized(&self) -> bool {
        self.tag == AccountTag::DerivativeMetadata && self.expired == ExpirationStatus::Active
    }

    pub fn expired(&self) -> bool {
        self.expired == ExpirationStatus::Expired
    }
}
