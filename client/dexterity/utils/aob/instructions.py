import struct

from solana.publickey import PublicKey
from solana.transaction import AccountMeta, TransactionInstruction
from dexterity.program_ids import AOB_PROGRAM_ID


CREATE_MARKET_IX_CODE = 0


def create_market_aob_ix(
    market: PublicKey,
    event_queue: PublicKey,
    bids: PublicKey,
    asks: PublicKey,
    caller_authority: PublicKey,
    callback_info_len: int,  # u64
    callback_id_len: int,  # u64
    min_base_order_size: int,  # u64
    price_bitmask: int = (1 << 64) - 1,
    cranker_reward: int = 1000,
):
    params = [
        CREATE_MARKET_IX_CODE,
        bytes(caller_authority),
        callback_info_len,
        callback_id_len,
        min_base_order_size,
        price_bitmask,
        cranker_reward,
    ]
    return TransactionInstruction(
        keys=[
            AccountMeta(market, is_signer=False, is_writable=True),
            AccountMeta(event_queue, is_signer=False, is_writable=True),
            AccountMeta(bids, is_signer=False, is_writable=True),
            AccountMeta(asks, is_signer=False, is_writable=True),
        ],
        program_id=AOB_PROGRAM_ID,
        data=struct.pack("<B32sQQQQQ", *params),
    )
