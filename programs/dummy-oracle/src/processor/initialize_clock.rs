use crate::utils::{assert_keys_equal, assert_signer, get_rent};
use bincode;
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
pub struct Params {}

struct Context<'a, 'b: 'a> {
    clock: &'a AccountInfo<'b>,
    update_authority: &'a AccountInfo<'b>,
    system_program: &'a AccountInfo<'b>,
    rent: Rent,
}

impl<'a, 'b: 'a> Context<'a, 'b> {
    pub fn parse(
        _program_id: &Pubkey,
        accounts: &'a [AccountInfo<'b>],
    ) -> std::result::Result<Self, ProgramError> {
        let accounts_iter = &mut accounts.iter();
        let a = Self {
            clock: next_account_info(accounts_iter)?,
            update_authority: next_account_info(accounts_iter)?,
            system_program: next_account_info(accounts_iter)?,
            rent: Rent::get()?,
        };
        assert_keys_equal(*a.system_program.key, system_program::id())?;
        assert_signer(a.update_authority)?;
        Ok(a)
    }
}

pub fn process(program_id: &Pubkey, accounts: &[AccountInfo], _params: Params) -> ProgramResult {
    let ctx = Context::parse(program_id, accounts)?;
    if ctx.clock.data_is_empty() {
        let seeds_without_bump: &[&[u8]] = &[b"clock"];
        let (_key, bump_seed) = Pubkey::find_program_address(seeds_without_bump, program_id);
        let seeds = &[seeds_without_bump[0], &[bump_seed]];
        invoke_signed(
            &system_instruction::create_account(
                ctx.update_authority.key,
                ctx.clock.key,
                get_rent(&ctx.rent, Clock::size_of() as u64, ctx.clock),
                Clock::size_of() as u64,
                program_id,
            ),
            &[
                ctx.update_authority.clone(),
                ctx.clock.clone(),
                ctx.system_program.clone(),
            ],
            &[seeds],
        )?;
    }
    let mut clock: Clock = bincode::deserialize(&ctx.clock.data.borrow()).ok().unwrap();

    clock.slot = 0;
    clock.epoch_start_timestamp = 0;
    clock.epoch = 0;
    clock.leader_schedule_epoch = 0;
    clock.unix_timestamp = 0;

    bincode::serialize_into(&mut *ctx.clock.data.borrow_mut(), &clock).ok();
    Ok(())
}
