from solana.publickey import PublicKey
from solana.transaction import AccountMeta, TransactionInstruction
from podite import I64, U64, pod

from .common import InstructionCode
from dexterity.program_ids import ORACLE_PROGRAM_ID


@pod
class Params:
    instr: InstructionCode
    slot: U64
    epoch_start_timestamp: I64
    epoch: U64
    leader_schedule_epoch: U64
    unix_timestamp: I64


def update_clock_ix(
    clock: PublicKey,
    slot: int,
    epoch_start_timestamp: int,
    epoch: int,
    leader_schedule_epoch: int,
    unix_timestamp: int,
):
    keys = [
        AccountMeta(pubkey=clock, is_signer=False, is_writable=True),
    ]
    params = Params(
        instr=InstructionCode.UPDATE_CLOCK,
        slot=slot,
        epoch_start_timestamp=epoch_start_timestamp,
        epoch=epoch,
        leader_schedule_epoch=leader_schedule_epoch,
        unix_timestamp=unix_timestamp,
    )
    return TransactionInstruction(
        keys=keys, program_id=ORACLE_PROGRAM_ID, data=params.to_bytes()
    )
