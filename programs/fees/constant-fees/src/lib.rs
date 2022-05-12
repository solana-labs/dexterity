use anchor_lang::prelude::*;
use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint,
    entrypoint::ProgramResult,
    msg,
    program_error::ProgramError,
    pubkey::Pubkey,
    sysvar::Sysvar,
};

use bytemuck::{Pod, Zeroable};
use dex::{
    error::DomainOrProgramResult,
    state::fee_model::{TraderFeeParams, TraderFees},
    utils::{
        param::{WithAcct, WithKey},
        validation::{assert_keys_equal, assert_signer, get_rent},
    },
};
use solana_program::{
    instruction::{AccountMeta, Instruction},
    program::invoke_signed,
    rent::Rent,
};

entrypoint!(process_instruction);
fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    process(program_id, accounts, instruction_data)
}

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
#[repr(u8)]
enum ConstantFeeModelInstruction {
    // This instruction is invoked by the DEX contract
    FindFees { params: TraderFeeParams },
    // These instructions are not exposed to the DEX
    InitializeTraderAcct,
    UpdateFees(UpdateFeesParams),
}

#[repr(C)]
#[derive(Debug, BorshSerialize, BorshDeserialize, PartialEq, Clone)]
pub struct UpdateFeesParams {
    pub maker_fee_bps: i32,
    pub taker_fee_bps: i32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Zeroable, Pod)]
pub struct TraderFeeState {
    pub bump: u64,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Zeroable, Pod)]
struct FeeConfig {
    maker_fee_bps: i32,
    taker_fee_bps: i32,
}

fn print_ix_name(ix: impl std::fmt::Debug) {
    msg!("Fee Ix: {:?}", ix);
}

fn process(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    let ix = ConstantFeeModelInstruction::try_from_slice(instruction_data).map_err(|e| {
        msg!("Error: {}", e);
        ProgramError::InvalidInstructionData
    })?;

    print_ix_name(&ix);
    match ix {
        ConstantFeeModelInstruction::FindFees { params } => {
            process_find_fees(program_id, accounts, &params)
        }
        ConstantFeeModelInstruction::UpdateFees(params) => {
            process_update_fees(program_id, accounts, params)
        }
        ConstantFeeModelInstruction::InitializeTraderAcct => {
            process_initialize_trader_acct(program_id, accounts)
        }
    }
    .map_err(|e| {
        msg!("Error: {}", &e);
        e.into()
    })
}

fn process_find_fees(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    _params: &TraderFeeParams,
) -> DomainOrProgramResult {
    let accounts_iter = &mut accounts.iter();
    let market_product_group = next_account_info(accounts_iter)?;
    let trader_risk_group = next_account_info(accounts_iter)?;
    // This account is just a placeholder for the constant_fees program
    let trader_fee_state = WithKey::<TraderFeeState>::load(next_account_info(accounts_iter)?)?;
    let fee_model_configuration_acct =
        WithKey::<FeeConfig>::load(next_account_info(accounts_iter)?)?;
    let mut fee_output_register =
        WithKey::<TraderFees>::load_mut(next_account_info(accounts_iter)?)?;

    let fee_signer = next_account_info(accounts_iter)?;

    let (fee_signer_key, _) =
        Pubkey::find_program_address(&[market_product_group.key.as_ref()], &dex::ID);
    assert_keys_equal(fee_signer_key, *fee_signer.key)?;
    assert_signer(fee_signer)?;

    let trader_state_key = Pubkey::create_program_address(
        &[
            b"trader_fee_acct",
            &trader_risk_group.key().to_bytes(),
            market_product_group.key.as_ref(),
            &[trader_fee_state.bump as u8],
        ],
        program_id,
    )?;
    assert_keys_equal(trader_state_key, *trader_fee_state.key)?;

    fee_output_register.valid_until = solana_program::clock::Clock::get()?.unix_timestamp + 1; // add an offset to allow skipping fee model calculations
    fee_output_register.set_taker_fee_bps(fee_model_configuration_acct.taker_fee_bps);
    fee_output_register.set_maker_fee_bps(fee_model_configuration_acct.maker_fee_bps);

    Ok(())
}

fn process_update_fees(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    params: UpdateFeesParams,
) -> DomainOrProgramResult {
    let accounts_iter = &mut accounts.iter();
    let payer = next_account_info(accounts_iter)?;
    let fee_model_config_acct = next_account_info(accounts_iter)?;
    let market_product_group = next_account_info(accounts_iter)?;
    let system_program = next_account_info(accounts_iter)?;

    if fee_model_config_acct.data_is_empty() {
        let label_seed = b"fee_model_config_acct";
        let (config_acct, bump_seed) = Pubkey::find_program_address(
            &[label_seed, market_product_group.key.as_ref()],
            program_id,
        );
        let seeds = &[label_seed, market_product_group.key.as_ref(), &[bump_seed]];
        assert_keys_equal(config_acct, *fee_model_config_acct.key)?;
        let size = std::mem::size_of::<FeeConfig>();
        msg!("{}", size);
        invoke_signed(
            &solana_program::system_instruction::create_account(
                payer.key,
                fee_model_config_acct.key,
                get_rent(&Rent::get()?, size as u64, fee_model_config_acct),
                size as u64,
                program_id,
            ),
            &[
                payer.clone(),
                fee_model_config_acct.clone(),
                system_program.clone(),
            ],
            &[seeds],
        )?;
    }
    let mut fee_model_configuration_acct = WithAcct::<FeeConfig>::load_mut(fee_model_config_acct)?;
    fee_model_configuration_acct.maker_fee_bps = params.maker_fee_bps;
    fee_model_configuration_acct.taker_fee_bps = params.taker_fee_bps;

    Ok(())
}

fn process_initialize_trader_acct(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
) -> DomainOrProgramResult {
    let accounts_iter = &mut accounts.iter();
    let payer = next_account_info(accounts_iter)?;
    let _fee_model_config_acct = WithAcct::<FeeConfig>::load(next_account_info(accounts_iter)?)?;
    // This account is just a placeholder for the constant_fees program
    let trader_fee_acct = next_account_info(accounts_iter)?;
    let market_product_group = next_account_info(accounts_iter)?;
    let trader_risk_group = next_account_info(accounts_iter)?;
    let system_program = next_account_info(accounts_iter)?;

    if !trader_fee_acct.data_is_empty() {
        msg!("TraderFeeAcct already initialized");
        return Err(ProgramError::InvalidArgument.into());
    }

    let label_seed = b"trader_fee_acct";
    let (trader_fee_acct_key, bump) = Pubkey::find_program_address(
        &[
            label_seed,
            &trader_risk_group.key.to_bytes(),
            &market_product_group.key.to_bytes(),
        ],
        program_id,
    );
    assert_keys_equal(*trader_fee_acct.key, trader_fee_acct_key)?;
    let size = std::mem::size_of::<TraderFeeState>();
    invoke_signed(
        &solana_program::system_instruction::create_account(
            payer.key,
            &trader_fee_acct_key,
            get_rent(&Rent::get()?, size as u64, trader_fee_acct),
            size as u64,
            program_id,
        ),
        &[
            payer.clone(),
            trader_fee_acct.clone(),
            system_program.clone(),
        ],
        &[&[
            label_seed,
            &trader_risk_group.key.to_bytes(),
            &market_product_group.key.to_bytes(),
            &[bump],
        ]],
    )?;
    let mut trader_fee_state = WithKey::<TraderFeeState>::load_mut(trader_fee_acct)?;
    trader_fee_state.bump = bump as u64;
    Ok(())
}

pub fn initialize_trader_fee_acct_ix(
    program_id: Pubkey,
    payer: Pubkey,
    fee_model_config_acct: Pubkey,
    trader_fee_acct: Pubkey,
    market_product_group: Pubkey,
    trader_risk_group: Pubkey,
    system_program: Pubkey,
) -> Instruction {
    let data = ConstantFeeModelInstruction::InitializeTraderAcct
        .try_to_vec()
        .unwrap();
    let accounts = vec![
        AccountMeta::new_readonly(payer, true),
        AccountMeta::new_readonly(fee_model_config_acct, false),
        AccountMeta::new(trader_fee_acct, false),
        AccountMeta::new_readonly(market_product_group, false),
        AccountMeta::new_readonly(trader_risk_group, false),
        AccountMeta::new_readonly(system_program, false),
    ];
    Instruction {
        program_id,
        accounts,
        data,
    }
}

pub fn update_fees_ix(
    program_id: Pubkey,
    payer: Pubkey,
    fee_model_config_acct: Pubkey,
    market_product_group: Pubkey,
    system_program: Pubkey,
    params: UpdateFeesParams,
) -> Instruction {
    let data = ConstantFeeModelInstruction::UpdateFees(params)
        .try_to_vec()
        .unwrap();

    let accounts = vec![
        AccountMeta::new_readonly(payer, true),
        AccountMeta::new(fee_model_config_acct, false),
        AccountMeta::new_readonly(market_product_group, false),
        AccountMeta::new_readonly(system_program, false),
    ];
    Instruction {
        program_id,
        accounts,
        data,
    }
}
