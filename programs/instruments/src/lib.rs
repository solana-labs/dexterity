pub mod error;
pub mod oracle;
pub mod processor;
pub mod state;

use crate::state::{
    derivative_metadata::DerivativeMetadata,
    enums::{InstrumentType, OracleType},
};
use anchor_lang::prelude::*;
use bytemuck::{Pod, Zeroable};
use dex::utils::numeric::Fractional;
use solana_program::{
    account_info::AccountInfo,
    clock::UnixTimestamp,
    entrypoint::ProgramResult,
    pubkey::Pubkey,
    sysvar::{rent::Rent, Sysvar},
};

declare_id!("instruments11111111111111111111111111111111");

#[program]
pub mod instruments {
    use super::*;
    pub fn initialize_derivative(
        ctx: Context<InitializeDerivative>,
        params: InitializeDerivativeParams,
    ) -> ProgramResult {
        processor::initialize_derivative::process(ctx, params)
    }

    pub fn settle_derivative(ctx: Context<SettleDerivative>) -> ProgramResult {
        processor::settle_derivative::process(ctx)
    }

    pub fn close_derivative_account(ctx: Context<CloseDerivativeAccount>) -> ProgramResult {
        processor::close_derivative_account::process(ctx)
    }
}

#[derive(Accounts)]
pub struct SettleDerivative<'info> {
    #[account(mut)]
    pub market_product_group: AccountInfo<'info>,
    #[account(
        mut,
        seeds=[
            b"derivative",
            price_oracle.key.to_bytes().as_ref(),
            market_product_group.key.to_bytes().as_ref(),
            (derivative_metadata.load()?.instrument_type as u64).to_le_bytes().as_ref(),
            derivative_metadata.load()?.strike.m.to_le_bytes().as_ref(),
            derivative_metadata.load()?.strike.exp.to_le_bytes().as_ref(),
            derivative_metadata.load()?.initialization_time.to_le_bytes().as_ref(),
            derivative_metadata.load()?.full_funding_period.to_le_bytes().as_ref(),
            derivative_metadata.load()?.minimum_funding_period.to_le_bytes().as_ref(),
        ],
        bump,
    )]
    pub derivative_metadata: AccountLoader<'info, DerivativeMetadata>,
    pub price_oracle: AccountInfo<'info>,
    pub dex_program: AccountInfo<'info>,
    pub clock: AccountInfo<'info>,
}

#[repr(C)]
#[derive(AnchorSerialize, AnchorDeserialize, Pod, Zeroable, PartialEq, Debug, Clone, Copy)]
pub struct InitializeDerivativeParams {
    /// CALL or PUT (perpetuals are just calls with 0 strike price)
    pub instrument_type: InstrumentType,
    /// Strike price of an option, 0 for for perpetual swaps and futures
    pub strike: Fractional,
    /// Number of seconds for a 100% interest payment
    pub full_funding_period: UnixTimestamp,
    /// Number of seconds for a minimum funding period (< 100%)
    pub minimum_funding_period: UnixTimestamp,
    pub initialization_time: UnixTimestamp,
    pub close_authority: Pubkey,
    // Oracle type
    pub oracle_type: OracleType,
}

#[derive(Accounts)]
#[instruction(params: InitializeDerivativeParams)]
pub struct InitializeDerivative<'info> {
    #[account(
        init,
        seeds=[
            b"derivative",
            price_oracle.key.to_bytes().as_ref(),
            market_product_group.key.to_bytes().as_ref(),
            (params.instrument_type as u64).to_le_bytes().as_ref(),
            params.strike.m.to_le_bytes().as_ref(),
            params.strike.exp.to_le_bytes().as_ref(),
            params.initialization_time.to_le_bytes().as_ref(),
            params.full_funding_period.to_le_bytes().as_ref(),
            params.minimum_funding_period.to_le_bytes().as_ref(),
        ],
        bump,
        payer = payer,
        space = 8 + std::mem::size_of::<DerivativeMetadata>()
    )]
    pub derivative_metadata: AccountLoader<'info, DerivativeMetadata>,
    pub price_oracle: AccountInfo<'info>,
    pub market_product_group: AccountInfo<'info>,
    #[account(mut)]
    pub payer: Signer<'info>,
    pub system_program: Program<'info, System>,
    pub clock: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct CloseDerivativeAccount<'info> {
    #[account(
        mut,
        seeds=[
            b"derivative",
            derivative_metadata.load()?.price_oracle.to_bytes().as_ref(),
            derivative_metadata.load()?.market_product_group.to_bytes().as_ref(),
            (derivative_metadata.load()?.instrument_type as u64).to_le_bytes().as_ref(),
            derivative_metadata.load()?.strike.m.to_le_bytes().as_ref(),
            derivative_metadata.load()?.strike.exp.to_le_bytes().as_ref(),
            derivative_metadata.load()?.initialization_time.to_le_bytes().as_ref(),
            derivative_metadata.load()?.full_funding_period.to_le_bytes().as_ref(),
            derivative_metadata.load()?.minimum_funding_period.to_le_bytes().as_ref(),
        ],
        bump,
    )]
    derivative_metadata: AccountLoader<'info, DerivativeMetadata>,
    close_authority: Signer<'info>,
    destination: AccountInfo<'info>,
}
