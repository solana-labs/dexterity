import struct

from solana.publickey import PublicKey

from dexterity.codegen.dex.types import Fractional
from dexterity.program_ids import INSTRUMENTS_PROGRAM_ID
from dexterity.codegen.instruments.types import InstrumentType


def get_derivative_key(
    price_oracle: PublicKey,
    market_product_group: PublicKey,
    instrument_type: InstrumentType,
    strike: float,
    full_funding_period: int,
    minimum_funding_period: int,
    initialization_time: int,
    **kwargs,
):
    strike = Fractional.to_decimal(strike)  # type: Fractional
    derivative_metadata, bump_seed = PublicKey.find_program_address(
        seeds=[
            b"derivative",
            bytes(price_oracle),
            bytes(market_product_group),
            struct.pack("<Q", int(instrument_type)),  # fix this to_bytes()
            struct.pack("<q", strike.m),
            struct.pack("<Q", strike.exp),
            struct.pack("<q", initialization_time),
            struct.pack("<q", full_funding_period),
            struct.pack("<q", minimum_funding_period),
        ],
        program_id=INSTRUMENTS_PROGRAM_ID,
    )
    return derivative_metadata, bump_seed
