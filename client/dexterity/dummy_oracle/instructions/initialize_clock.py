from solana.publickey import PublicKey
from solana.transaction import AccountMeta, TransactionInstruction
from solana.system_program import SYS_PROGRAM_ID
from podite import I64, U64, pod

from .common import InstructionCode
from dexterity.program_ids import ORACLE_PROGRAM_ID


@pod
class Params:
    instr: InstructionCode


def initialize_clock_ix(clock: PublicKey, update_authority: PublicKey):
    keys = [
        AccountMeta(pubkey=clock, is_signer=False, is_writable=True),
        AccountMeta(pubkey=update_authority, is_signer=True, is_writable=False),
        AccountMeta(pubkey=SYS_PROGRAM_ID, is_signer=False, is_writable=False),
    ]
    params = Params(instr=InstructionCode.INITIALIZE_CLOCK)
    return TransactionInstruction(
        keys=keys, program_id=ORACLE_PROGRAM_ID, data=params.to_bytes()
    )
