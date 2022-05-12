#![allow(unused_imports)]

use agnostic_orderbook::state::{SelfTradeBehavior, Side};
use anchor_lang::{
    prelude::*,
    solana_program::{
        account_info::AccountInfo,
        entrypoint::ProgramResult,
        instruction::{AccountMeta, Instruction},
        pubkey::Pubkey,
        sysvar::{rent::Rent, Sysvar},
    },
};
use anchor_spl::token::{Mint, Token, TokenAccount};
use borsh::{BorshDeserialize, BorshSerialize};

use crate::{
    error::{DomainOrProgramError, UtilError},
    state::{
        constants::NAME_LEN,
        enums::OrderType,
        fee_model::TraderFeeParams,
        market_product_group::MarketProductGroup,
        risk_engine_register::{OperationType, OrderInfo, RiskOutputRegister},
        trader_risk_group::TraderRiskGroup,
    },
    utils::numeric::Fractional,
    UtilError::SerializeError,
};

pub mod error;
/// Handlers for each instruction
pub mod processor;
/// Describes the data structures the program uses to encode state
pub mod state;
/// Helper functions
pub mod utils;

declare_id!("Dex1111111111111111111111111111111111111111");

#[program]
pub mod dex {
    use super::*;

    pub fn initialize_market_product_group(
        ctx: Context<InitializeMarketProductGroup>,
        params: InitializeMarketProductGroupParams,
    ) -> ProgramResult {
        processor::initialize_market_product_group::process(ctx, params).map_err(log_errors)
    }

    pub fn initialize_market_product(
        ctx: Context<InitializeMarketProduct>,
        params: InitializeMarketProductParams,
    ) -> ProgramResult {
        processor::initialize_market_product::process(ctx, params).map_err(log_errors)
    }

    pub fn remove_market_product(ctx: Context<RemoveMarketProduct>) -> ProgramResult {
        processor::remove_market_product::process(ctx).map_err(log_errors)
    }

    pub fn initialize_trader_risk_group<'a, 'b, 'c, 'info>(
        ctx: Context<'a, 'b, 'c, 'info, InitializeTraderRiskGroup<'info>>,
    ) -> ProgramResult {
        processor::initialize_trader_risk_group::process(ctx).map_err(log_errors)
    }

    pub fn new_order<'info>(
        ctx: Context<'_, '_, '_, 'info, NewOrder<'info>>,
        params: NewOrderParams,
    ) -> ProgramResult {
        processor::new_order::process(ctx, params).map_err(log_errors)
    }

    pub fn consume_orderbook_events<'a, 'b, 'c, 'info>(
        ctx: Context<'a, 'b, 'c, 'info, ConsumeOrderbookEvents<'info>>,
        params: ConsumeOrderbookEventsParams,
    ) -> ProgramResult {
        processor::consume_orderbook_events::process(ctx, params).map_err(log_errors)
    }

    pub fn cancel_order<'info>(
        ctx: Context<'_, '_, '_, 'info, CancelOrder<'info>>,
        params: CancelOrderParams,
    ) -> ProgramResult {
        processor::cancel_order::process(ctx, params).map_err(log_errors)
    }

    pub fn deposit_funds(ctx: Context<DepositFunds>, params: DepositFundsParams) -> ProgramResult {
        processor::deposit_funds::process(ctx, params).map_err(log_errors)
    }

    pub fn withdraw_funds<'info>(
        ctx: Context<'_, '_, '_, 'info, WithdrawFunds<'info>>,
        params: WithdrawFundsParams,
    ) -> ProgramResult {
        processor::withdraw_funds::process(ctx, params).map_err(log_errors)
    }

    pub fn update_product_funding(
        ctx: Context<UpdateProductFunding>,
        params: UpdateProductFundingParams,
    ) -> ProgramResult {
        processor::update_product_funding::process(ctx, params).map_err(log_errors)
    }

    pub fn transfer_full_position<'info>(
        ctx: Context<'_, '_, '_, 'info, TransferFullPosition<'info>>,
    ) -> ProgramResult {
        // msg!("Dex Instr: Transfer full position");
        processor::transfer_full_position::process(ctx).map_err(log_errors)
    }

    pub fn initialize_combo(
        ctx: Context<InitializeCombo>,
        params: InitializeComboParams,
    ) -> ProgramResult {
        processor::initialize_combo::process(ctx, params).map_err(log_errors)
    }

    pub fn update_trader_funding(ctx: Context<UpdateTraderFunding>) -> ProgramResult {
        processor::update_trader_funding::process(ctx).map_err(log_errors)
    }

    pub fn clear_expired_orderbook(
        ctx: Context<ClearExpiredOrderbook>,
        params: ClearExpiredOrderbookParams,
    ) -> ProgramResult {
        processor::clear_expired_orderbook::process(ctx, params).map_err(log_errors)
    }

    pub fn sweep_fees(ctx: Context<SweepFees>) -> ProgramResult {
        processor::sweep_fees::process(ctx).map_err(log_errors)
    }

    pub fn choose_successor(ctx: Context<ChooseSuccessor>) -> ProgramResult {
        processor::change_authority::choose_successor(ctx).map_err(log_errors)
    }

    pub fn claim_authority(ctx: Context<ClaimAuthority>) -> ProgramResult {
        processor::change_authority::claim_authority(ctx).map_err(log_errors)
    }
}

fn log_errors(e: DomainOrProgramError) -> ProgramError {
    msg!("Error: {}", e);
    e.into()
}

#[repr(C)]
#[derive(AnchorSerialize, AnchorDeserialize, PartialEq, Debug, Clone)]
pub struct InitializeMarketProductGroupParams {
    pub name: [u8; NAME_LEN],
    pub validate_account_discriminant_len: u64,
    pub find_fees_discriminant_len: u64,
    pub validate_account_health_discriminant: [u8; 8],
    pub validate_account_liquidation_discriminant: [u8; 8],
    pub create_risk_state_account_discriminant: [u8; 8],
    pub find_fees_discriminant: [u8; 8],
    pub max_maker_fee_bps: i16,
    pub min_maker_fee_bps: i16,
    pub max_taker_fee_bps: i16,
    pub min_taker_fee_bps: i16,
}

#[derive(Accounts)]
pub struct InitializeMarketProductGroup<'info> {
    authority: Signer<'info>,
    #[account(zero)]
    market_product_group: AccountLoader<'info, MarketProductGroup>,
    #[account(mut)]
    market_product_group_vault: AccountInfo<'info>,
    vault_mint: Account<'info, Mint>,
    fee_collector: AccountInfo<'info>,
    #[account(executable)]
    fee_model_program: AccountInfo<'info>,
    fee_model_configuration_acct: AccountInfo<'info>,
    risk_model_configuration_acct: AccountInfo<'info>,
    #[account(executable)]
    risk_engine_program: AccountInfo<'info>,
    sysvar_rent: AccountInfo<'info>,
    system_program: Program<'info, System>,
    token_program: Program<'info, Token>,
    fee_output_register: AccountInfo<'info>,
    risk_output_register: AccountInfo<'info>,
}

#[repr(C)]
#[derive(AnchorSerialize, AnchorDeserialize, PartialEq, Debug, Clone)]
pub struct InitializeMarketProductParams {
    pub name: [u8; NAME_LEN],
    pub tick_size: Fractional,
    pub base_decimals: u64,
    pub price_offset: Fractional, // Allows for negative prices in ticks up to -price_offset
}

#[derive(Accounts)]
pub struct InitializeMarketProduct<'info> {
    authority: Signer<'info>,
    #[account(mut)]
    market_product_group: AccountLoader<'info, MarketProductGroup>,
    product: AccountInfo<'info>,
    orderbook: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct RemoveMarketProduct<'info> {
    authority: Signer<'info>,
    #[account(mut)]
    market_product_group: AccountLoader<'info, MarketProductGroup>,
    product: AccountInfo<'info>,
    #[account(executable)]
    aaob_program: AccountInfo<'info>,
    #[account(mut)]
    orderbook: AccountInfo<'info>,
    market_signer: AccountInfo<'info>,
    #[account(mut)]
    event_queue: AccountInfo<'info>,
    #[account(mut)]
    bids: AccountInfo<'info>,
    #[account(mut)]
    asks: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct InitializeTraderRiskGroup<'info> {
    #[account(mut)]
    owner: Signer<'info>,
    #[account(zero)]
    trader_risk_group: AccountLoader<'info, TraderRiskGroup>,
    market_product_group: AccountLoader<'info, MarketProductGroup>,
    risk_signer: AccountInfo<'info>,
    #[account(mut)]
    trader_risk_state_acct: Signer<'info>,
    trader_fee_state_acct: AccountInfo<'info>,
    risk_engine_program: AccountInfo<'info>,
    system_program: Program<'info, System>,
}

#[repr(C)]
#[derive(AnchorSerialize, AnchorDeserialize, PartialEq, Clone)]
pub struct NewOrderParams {
    /// The order's side (Bid or Ask)
    pub side: Side,
    /// The max quantity of base token to match and post
    pub max_base_qty: Fractional,
    /// The order type (supported types include Limit, FOK, IOC and PostOnly)
    pub order_type: OrderType,
    /// Configures what happens when this order is at least partially matched against an order belonging to the same user account
    pub self_trade_behavior: SelfTradeBehavior,
    /// The maximum number of orders to be matched against.
    /// Setting this number too high can sometimes lead to excessive resource consumption which can cause a failure.
    pub match_limit: u64,
    /// The order's limit price in ticks
    pub limit_price: Fractional,
}

#[derive(Accounts)]
pub struct NewOrder<'info> {
    #[account(mut, signer)]
    user: AccountInfo<'info>,
    #[account(mut)]
    trader_risk_group: AccountLoader<'info, TraderRiskGroup>,
    #[account(mut)]
    market_product_group: AccountLoader<'info, MarketProductGroup>,
    product: AccountInfo<'info>,
    #[account(executable)]
    aaob_program: AccountInfo<'info>,
    #[account(mut)]
    orderbook: AccountInfo<'info>,
    market_signer: AccountInfo<'info>,
    #[account(mut)]
    event_queue: AccountInfo<'info>,
    #[account(mut)]
    bids: AccountInfo<'info>,
    #[account(mut)]
    asks: AccountInfo<'info>,
    system_program: Program<'info, System>,
    #[account(executable)]
    fee_model_program: AccountInfo<'info>,
    fee_model_configuration_acct: AccountInfo<'info>,
    #[account(mut)]
    trader_fee_state_acct: AccountInfo<'info>,
    #[account(mut)]
    fee_output_register: AccountInfo<'info>,
    #[account(executable)]
    risk_engine_program: AccountInfo<'info>,
    risk_model_configuration_acct: AccountInfo<'info>,
    #[account(mut)]
    risk_output_register: AccountInfo<'info>,
    #[account(mut)]
    trader_risk_state_acct: AccountInfo<'info>,
    risk_and_fee_signer: AccountInfo<'info>,
}

#[repr(C)]
#[derive(AnchorSerialize, AnchorDeserialize, PartialEq, Clone)]
pub struct ConsumeOrderbookEventsParams {
    /// The maximum number of events to consume
    pub max_iterations: u64,
}

#[derive(Accounts)]
pub struct ConsumeOrderbookEvents<'info> {
    #[account(executable)]
    aaob_program: AccountInfo<'info>,
    #[account(mut)]
    market_product_group: AccountLoader<'info, MarketProductGroup>,
    product: AccountInfo<'info>,
    market_signer: AccountInfo<'info>,
    #[account(mut)]
    orderbook: AccountInfo<'info>,
    #[account(mut)]
    event_queue: AccountInfo<'info>,
    #[account(mut, signer)]
    reward_target: AccountInfo<'info>,
    #[account(executable)]
    fee_model_program: AccountInfo<'info>,
    fee_model_configuration_acct: AccountInfo<'info>,
    #[account(mut)]
    fee_output_register: AccountInfo<'info>,
    risk_and_fee_signer: AccountInfo<'info>,
    // Remaining accounts are for risk engine
}
#[repr(C)]
#[derive(AnchorSerialize, AnchorDeserialize, PartialEq, Clone)]
pub struct CancelOrderParams {
    /// The order_id of the order to cancel. Redundancy is used here to avoid having to iterate over all
    /// open orders on chain.
    pub order_id: u128,
}

#[derive(Accounts)]
pub struct CancelOrder<'info> {
    user: Signer<'info>,
    #[account(mut)]
    trader_risk_group: AccountLoader<'info, TraderRiskGroup>,
    #[account(mut)]
    market_product_group: AccountLoader<'info, MarketProductGroup>,
    product: AccountInfo<'info>,
    #[account(executable)]
    aaob_program: AccountInfo<'info>,
    #[account(mut)]
    orderbook: AccountInfo<'info>,
    market_signer: AccountInfo<'info>,
    #[account(mut)]
    event_queue: AccountInfo<'info>,
    #[account(mut)]
    bids: AccountInfo<'info>,
    #[account(mut)]
    asks: AccountInfo<'info>,
    system_program: Program<'info, System>,
    #[account(executable)]
    risk_engine_program: AccountInfo<'info>,
    risk_model_configuration_acct: AccountInfo<'info>,
    #[account(mut)]
    risk_output_register: AccountInfo<'info>,
    #[account(mut)]
    trader_risk_state_acct: AccountInfo<'info>,
    risk_signer: AccountInfo<'info>,
    // Remaining accounts are for risk engine
}

#[repr(C)]
#[derive(AnchorSerialize, AnchorDeserialize, PartialEq, Clone)]
pub struct DepositFundsParams {
    pub quantity: Fractional,
}

#[derive(Accounts)]
pub struct DepositFunds<'info> {
    token_program: Program<'info, Token>,
    user: Signer<'info>,
    #[account(mut)]
    user_token_account: Account<'info, TokenAccount>,
    #[account(mut)]
    trader_risk_group: AccountLoader<'info, TraderRiskGroup>,
    market_product_group: AccountLoader<'info, MarketProductGroup>,
    #[account(mut)]
    market_product_group_vault: Account<'info, TokenAccount>,
}

#[repr(C)]
#[derive(AnchorSerialize, AnchorDeserialize, PartialEq, Clone)]
pub struct WithdrawFundsParams {
    pub quantity: Fractional,
}

#[derive(Accounts)]
pub struct WithdrawFunds<'info> {
    token_program: Program<'info, Token>,
    user: Signer<'info>,
    #[account(mut)]
    user_token_account: Account<'info, TokenAccount>,
    #[account(mut)]
    trader_risk_group: AccountLoader<'info, TraderRiskGroup>,
    market_product_group: AccountLoader<'info, MarketProductGroup>,
    #[account(mut)]
    market_product_group_vault: Account<'info, TokenAccount>,
    #[account(executable)]
    risk_engine_program: AccountInfo<'info>,
    risk_model_configuration_acct: AccountInfo<'info>,
    #[account(mut)]
    risk_output_register: AccountInfo<'info>,
    #[account(mut)]
    trader_risk_state_acct: AccountInfo<'info>,
    risk_signer: AccountInfo<'info>,
    // Remaining accounts are for risk engine
}

#[repr(C)]
#[derive(AnchorSerialize, AnchorDeserialize, PartialEq, Clone)]
pub struct UpdateProductFundingParams {
    pub amount: Fractional,
    pub expired: bool,
}

#[derive(Accounts)]
pub struct UpdateProductFunding<'info> {
    #[account(mut)]
    market_product_group: AccountLoader<'info, MarketProductGroup>,
    product: Signer<'info>,
}

#[derive(Accounts)]
pub struct TransferFullPosition<'info> {
    liquidator: Signer<'info>,
    #[account(mut)]
    market_product_group: AccountLoader<'info, MarketProductGroup>,
    #[account(mut)]
    liquidatee_risk_group: AccountLoader<'info, TraderRiskGroup>,
    #[account(mut)]
    liquidator_risk_group: AccountLoader<'info, TraderRiskGroup>,
    #[account(executable)]
    risk_engine_program: AccountInfo<'info>,
    risk_model_configuration_acct: AccountInfo<'info>,
    #[account(mut)]
    risk_output_register: AccountInfo<'info>,
    #[account(mut)]
    liquidator_risk_state_account_info: AccountInfo<'info>,
    #[account(mut)]
    liquidatee_risk_state_account_info: AccountInfo<'info>,
    risk_signer: AccountInfo<'info>,
    // Remaining accounts are for risk engine
}

#[repr(C)]
#[derive(AnchorSerialize, AnchorDeserialize, PartialEq, Debug, Clone)]
pub struct InitializeComboParams {
    pub name: [u8; NAME_LEN],
    // Fixed point number (32 integer bits, 32 fractional bits)
    pub tick_size: Fractional,
    pub price_offset: Fractional,
    pub base_decimals: u64,
    pub ratios: Vec<i8>,
}

#[derive(Accounts)]
pub struct InitializeCombo<'info> {
    authority: Signer<'info>,
    #[account(mut)]
    market_product_group: AccountLoader<'info, MarketProductGroup>,
    orderbook: AccountInfo<'info>,
    // Remaining accounts are for products
}

#[derive(Accounts)]
pub struct UpdateTraderFunding<'info> {
    #[account(mut)]
    market_product_group: AccountLoader<'info, MarketProductGroup>,
    #[account(mut)]
    trader_risk_group: AccountLoader<'info, TraderRiskGroup>,
}

#[repr(C)]
#[derive(AnchorSerialize, AnchorDeserialize, PartialEq, Clone)]
pub struct ClearExpiredOrderbookParams {
    pub num_orders_to_cancel: u8,
}

#[derive(Accounts)]
pub struct ClearExpiredOrderbook<'info> {
    market_product_group: AccountLoader<'info, MarketProductGroup>,
    product: AccountInfo<'info>,
    #[account(executable)]
    aaob_program: AccountInfo<'info>,
    #[account(mut)]
    orderbook: AccountInfo<'info>,
    market_signer: AccountInfo<'info>,
    #[account(mut)]
    event_queue: AccountInfo<'info>,
    #[account(mut)]
    bids: AccountInfo<'info>,
    #[account(mut)]
    asks: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct SweepFees<'info> {
    #[account(mut)]
    market_product_group: AccountLoader<'info, MarketProductGroup>,
    fee_collector: AccountInfo<'info>,
    #[account(mut)]
    market_product_group_vault: Account<'info, TokenAccount>,
    #[account(mut)]
    fee_collector_token_account: Account<'info, TokenAccount>,
    token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct ChooseSuccessor<'info> {
    #[account(mut)]
    market_product_group: AccountLoader<'info, MarketProductGroup>,
    authority: Signer<'info>,
    new_authority: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct ClaimAuthority<'info> {
    #[account(mut)]
    market_product_group: AccountLoader<'info, MarketProductGroup>,
    new_authority: Signer<'info>,
}

#[derive(Accounts)]
pub struct UpdateHealthState<'info> {
    authority: Signer<'info>,
    #[account(mut)]
    trader_risk_group: AccountLoader<'info, TraderRiskGroup>,
    #[account(mut)]
    market_product_group: AccountLoader<'info, MarketProductGroup>,
    #[account(executable)]
    risk_engine_program: AccountInfo<'info>,
    #[account(mut)]
    risk_output_register: AccountInfo<'info>,
    #[account(mut)]
    trader_risk_state_acct: AccountInfo<'info>,
}

pub fn validate_account_health_ix(
    program_id: Pubkey,
    market_product_group: Pubkey,
    trader_risk_group: Pubkey,
    out_register_risk: Pubkey,
    trader_risk_state_acct: Pubkey,
    risk_model_configuration: Pubkey,
    risk_signer: Pubkey,
    risk_engine_accounts: Vec<Pubkey>,
    mut discriminant: Vec<u8>,
    order_info: &OrderInfo,
) -> std::result::Result<Instruction, DomainOrProgramError> {
    let mut accounts = vec![
        AccountMeta::new_readonly(market_product_group, false),
        AccountMeta::new_readonly(trader_risk_group, false),
        AccountMeta::new(out_register_risk, false),
        AccountMeta::new(trader_risk_state_acct, false),
        AccountMeta::new_readonly(risk_model_configuration, false),
        AccountMeta::new_readonly(risk_signer, true),
    ];
    for key in risk_engine_accounts.into_iter() {
        accounts.push(AccountMeta::new(key, false));
    }
    BorshSerialize::serialize(order_info, &mut discriminant)
        .map_err(|_| UtilError::SerializeError)?;
    Ok(Instruction {
        program_id,
        accounts,
        data: discriminant,
    })
}

pub fn find_fees_ix(
    program_id: Pubkey,
    market_product_group: Pubkey,
    trader_risk_group: Pubkey,
    trader_fee_state_acct: Pubkey,
    fee_model_configuration: Pubkey,
    fee_output_register: Pubkey,
    fee_signer: Pubkey,
    fee_params: &TraderFeeParams,
    mut discriminant: Vec<u8>,
) -> std::result::Result<Instruction, DomainOrProgramError> {
    let accounts = vec![
        AccountMeta::new_readonly(market_product_group, false),
        AccountMeta::new_readonly(trader_risk_group, false),
        AccountMeta::new(trader_fee_state_acct, false),
        AccountMeta::new_readonly(fee_model_configuration, false),
        AccountMeta::new(fee_output_register, false),
        AccountMeta::new_readonly(fee_signer, true),
    ];
    BorshSerialize::serialize(fee_params, &mut discriminant)
        .map_err(|_| UtilError::SerializeError)?;
    Ok(Instruction {
        program_id,
        accounts,
        data: discriminant,
    })
}

pub fn create_trader_risk_state_acct_ix(
    program_id: Pubkey,
    authority: Pubkey,
    risk_signer: Pubkey,
    trader_risk_state_acct: &AccountInfo,
    market_product_group: Pubkey,
    system_program: Pubkey,
    risk_engine_accounts: Vec<Pubkey>,
    discriminant: Vec<u8>,
) -> Instruction {
    let mut accounts = vec![
        AccountMeta::new(authority, true),
        AccountMeta::new_readonly(risk_signer, true),
        AccountMeta::new(
            trader_risk_state_acct.key(),
            trader_risk_state_acct.is_signer,
        ),
        AccountMeta::new_readonly(market_product_group, false),
        AccountMeta::new_readonly(system_program, false),
    ];
    for key in risk_engine_accounts.into_iter() {
        accounts.push(AccountMeta::new(key, false));
    }
    Instruction {
        program_id,
        accounts,
        data: discriminant,
    }
}
