use bincode;
use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program_error::ProgramError,
    pubkey::Pubkey,
    sysvar::clock::Clock,
};

#[repr(C)]
#[derive(BorshSerialize, BorshDeserialize, PartialEq, Debug, Clone)]
pub struct Params {
    pub slot: u64,
    pub epoch_start_timestamp: i64,
    pub epoch: u64,
    pub leader_schedule_epoch: u64,
    pub unix_timestamp: i64,
}

struct Context<'a, 'b: 'a> {
    clock: &'a AccountInfo<'b>,
}

impl<'a, 'b: 'a> Context<'a, 'b> {
    pub fn parse(
        _program_id: &Pubkey,
        accounts: &'a [AccountInfo<'b>],
    ) -> std::result::Result<Self, ProgramError> {
        let accounts_iter = &mut accounts.iter();
        let a = Self {
            clock: next_account_info(accounts_iter)?,
        };
        Ok(a)
    }
}

pub fn process(program_id: &Pubkey, accounts: &[AccountInfo], params: Params) -> ProgramResult {
    let ctx = Context::parse(program_id, accounts)?;
    let mut clock: Clock = bincode::deserialize(&ctx.clock.data.borrow()).ok().unwrap();
    clock.slot = params.slot;
    clock.epoch_start_timestamp = params.epoch_start_timestamp;
    clock.epoch = params.epoch;
    clock.leader_schedule_epoch = params.leader_schedule_epoch;
    clock.unix_timestamp = params.unix_timestamp;
    bincode::serialize_into(&mut *ctx.clock.data.borrow_mut(), &clock).ok();
    Ok(())
}
