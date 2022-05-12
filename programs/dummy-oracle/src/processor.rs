#![allow(unused_imports)]
use crate::instruction::DummyInstruction;
use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::AccountInfo,
    entrypoint::ProgramResult,
    instruction::{AccountMeta, Instruction},
    msg,
    pubkey::Pubkey,
};

#[allow(missing_docs)]
pub mod initialize_clock;

#[allow(missing_docs)]
pub mod initialize_oracle;

#[allow(missing_docs)]
pub mod update_clock;

#[allow(missing_docs)]
pub mod update_price;

pub struct Processor;
impl Processor {
    pub fn process(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        instruction_data: &[u8],
    ) -> ProgramResult {
        let instruction = DummyInstruction::try_from_slice(instruction_data)?;
        match instruction {
            DummyInstruction::InitializeClock(params) => {
                msg!("Instruction: InitializeClock");
                initialize_clock::process(program_id, accounts, params)
            }
            DummyInstruction::InitializeOracle(params) => {
                msg!("Instruction: InitializeOracle");
                initialize_oracle::process(program_id, accounts, params)
            }
            DummyInstruction::UpdateClock(params) => {
                msg!("Instruction: UpdateClock");
                update_clock::process(program_id, accounts, params)
            }
            DummyInstruction::UpdatePrice(params) => {
                msg!("Instruction: UpdatePrice");
                update_price::process(program_id, accounts, params)
            }
        }
    }
}

pub fn initialize_clock_ix(
    program_id: Pubkey,
    clock: Pubkey,
    update_authority: Pubkey,
    system_program: Pubkey,
    params: initialize_clock::Params,
) -> Instruction {
    let data = DummyInstruction::InitializeClock(params)
        .try_to_vec()
        .unwrap();
    let accounts = vec![
        AccountMeta::new(clock, false),
        AccountMeta::new(update_authority, true),
        AccountMeta::new_readonly(system_program, false),
    ];
    Instruction {
        program_id,
        accounts,
        data,
    }
}

pub fn initialize_oracle_ix(
    program_id: Pubkey,
    oracle_price: Pubkey,
    update_authority: Pubkey,
    system_program: Pubkey,
    params: initialize_oracle::Params,
) -> Instruction {
    let data = DummyInstruction::InitializeOracle(params)
        .try_to_vec()
        .unwrap();
    let accounts = vec![
        AccountMeta::new(oracle_price, false),
        AccountMeta::new(update_authority, true),
        AccountMeta::new_readonly(system_program, false),
    ];
    Instruction {
        program_id,
        accounts,
        data,
    }
}

pub fn update_clock_ix(
    program_id: Pubkey,
    oracle_price: Pubkey,
    update_authority: Pubkey,
    system_program: Pubkey,
    params: update_clock::Params,
) -> Instruction {
    let data = DummyInstruction::UpdateClock(params).try_to_vec().unwrap();
    let accounts = vec![
        AccountMeta::new(oracle_price, false),
        AccountMeta::new(update_authority, true),
        AccountMeta::new_readonly(system_program, false),
    ];
    Instruction {
        program_id,
        accounts,
        data,
    }
}

pub fn update_price_ix(
    program_id: Pubkey,
    oracle_price: Pubkey,
    update_authority: Pubkey,
    system_program: Pubkey,
    params: update_price::Params,
) -> Instruction {
    let data = DummyInstruction::UpdatePrice(params).try_to_vec().unwrap();
    let accounts = vec![
        AccountMeta::new(oracle_price, false),
        AccountMeta::new(update_authority, true),
        AccountMeta::new_readonly(system_program, false),
    ];
    Instruction {
        program_id,
        accounts,
        data,
    }
}
