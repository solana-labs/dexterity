import struct

from solana.publickey import PublicKey

from dexterity.program_ids import INSTRUMENTS_PROGRAM_ID

MAX_DATES = 32


def get_fixed_income_key(
    market_product_group: PublicKey,
    initialization_time: int,
    coupon_rate: int,
    maturity_date: int,
):
    fixed_income_metadata, bump_seed = PublicKey.find_program_address(
        seeds=[
            b"fixed_income",
            bytes(market_product_group),
            struct.pack("<Q", initialization_time),
            struct.pack("<q", coupon_rate),
            struct.pack("<Q", maturity_date),  # Todo: Add bump seed
        ],
        program_id=INSTRUMENTS_PROGRAM_ID,
    )

    return fixed_income_metadata, bump_seed
