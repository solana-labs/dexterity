use crate::{state::OraclePrice, utils::assert_signer};
use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program_error::ProgramError,
    pubkey::Pubkey,
    sysvar::{clock::Clock, Sysvar},
};

#[repr(C)]
#[derive(BorshSerialize, BorshDeserialize, PartialEq, Debug, Clone)]
pub struct Params {
    pub price: i64,
    pub decimals: u64,
}

struct Context<'a, 'b: 'a> {
    oracle_price: &'a AccountInfo<'b>,
    update_authority: &'a AccountInfo<'b>,
    clock: Clock,
}

impl<'a, 'b: 'a> Context<'a, 'b> {
    pub fn parse(
        _program_id: &Pubkey,
        accounts: &'a [AccountInfo<'b>],
    ) -> std::result::Result<Self, ProgramError> {
        let accounts_iter = &mut accounts.iter();
        let a = Self {
            oracle_price: next_account_info(accounts_iter)?,
            update_authority: next_account_info(accounts_iter)?,
            clock: Clock::get()?,
        };
        assert_signer(a.update_authority)?;
        Ok(a)
    }
}

pub fn process(program_id: &Pubkey, accounts: &[AccountInfo], params: Params) -> ProgramResult {
    let ctx = Context::parse(program_id, accounts)?;
    let mut oracle_price = OraclePrice::try_from_slice(&ctx.oracle_price.data.borrow_mut())?;
    if !oracle_price.is_initialized() {
        msg!("Oracle Price account is not initialized");
        return Err(ProgramError::InvalidAccountData);
    }

    if *ctx.update_authority.key != oracle_price.update_authority {
        msg!("Update Authorities do not match");
        return Err(ProgramError::InvalidAccountData);
    }

    oracle_price.price = params.price;
    oracle_price.decimals = params.decimals;
    oracle_price.slot = ctx.clock.slot;

    oracle_price.serialize(&mut *ctx.oracle_price.data.borrow_mut())?;
    Ok(())
}
