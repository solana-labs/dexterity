from solana.publickey import PublicKey
from solana.transaction import AccountMeta, TransactionInstruction
from podite import I64, U64, pod
from solana.system_program import SYS_PROGRAM_ID

from .common import InstructionCode
from dexterity.program_ids import ORACLE_PROGRAM_ID


@pod
class Params:
    instr: InstructionCode
    price: I64
    decimals: U64


def update_price_ix(
    oracle_price: PublicKey, update_authority: PublicKey, price: int, decimals: int = 0
):
    keys = [
        AccountMeta(pubkey=oracle_price, is_signer=False, is_writable=True),
        AccountMeta(pubkey=update_authority, is_signer=True, is_writable=False),
        AccountMeta(pubkey=SYS_PROGRAM_ID,is_signer=False,is_writable=False),
    ]
    params = Params(
        instr=InstructionCode.UPDATE_PRICE,
        price=price,
        decimals=decimals,
    )
    return TransactionInstruction(
        keys=keys, program_id=ORACLE_PROGRAM_ID, data=params.to_bytes()
    )
