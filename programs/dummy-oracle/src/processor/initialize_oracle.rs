use crate::{
    state::{AccountTag, OraclePrice},
    utils::{assert_keys_equal, assert_signer, get_rent},
};
use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program::invoke_signed,
    program_error::ProgramError,
    pubkey::Pubkey,
    system_instruction, system_program,
    sysvar::{clock::Clock, rent::Rent, Sysvar},
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
    system_program: &'a AccountInfo<'b>,
    rent: Rent,
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
            system_program: next_account_info(accounts_iter)?,
            rent: Rent::get()?,
            clock: Clock::get()?,
        };
        assert_keys_equal(*a.system_program.key, system_program::id())?;
        assert_signer(a.update_authority)?;
        Ok(a)
    }
}

pub fn process(program_id: &Pubkey, accounts: &[AccountInfo], params: Params) -> ProgramResult {
    let ctx = Context::parse(program_id, accounts)?;
    if ctx.oracle_price.data_is_empty() {
        let seeds_without_bump: &[&[u8]] = &[b"oracle"];
        let (key, bump_seed) = Pubkey::find_program_address(seeds_without_bump, program_id);
        assert_keys_equal(key, *ctx.oracle_price.key)?;
        let seeds = &[seeds_without_bump[0], &[bump_seed]];
        invoke_signed(
            &system_instruction::create_account(
                ctx.update_authority.key,
                ctx.oracle_price.key,
                get_rent(&ctx.rent, OraclePrice::LEN as u64, ctx.oracle_price),
                OraclePrice::LEN,
                program_id,
            ),
            &[
                ctx.update_authority.clone(),
                ctx.oracle_price.clone(),
                ctx.system_program.clone(),
            ],
            &[seeds],
        )?;
    }
    let mut oracle_price = OraclePrice::try_from_slice(&ctx.oracle_price.data.borrow_mut())?;
    if !oracle_price.is_initialized() {
        oracle_price.tag = AccountTag::OraclePrice;
        oracle_price.price = params.price;
        oracle_price.decimals = params.decimals;
        oracle_price.slot = ctx.clock.slot;
        oracle_price.update_authority = *ctx.update_authority.key;
        oracle_price.serialize(&mut *ctx.oracle_price.data.borrow_mut())?;
    }
    Ok(())
}
