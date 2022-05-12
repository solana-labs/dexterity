# LOCK-BEGIN[imports]: DON'T MODIFY
from dexterity.codegen.dex.types.fractional import Fractional
from dexterity.codegen.instruments.types.account_tag import AccountTag
from dexterity.codegen.instruments.types.expiration_status import ExpirationStatus
from dexterity.codegen.instruments.types.instrument_type import InstrumentType
from dexterity.codegen.instruments.types.oracle_type import OracleType
from podite import (
    U64,
    pod,
)
from solana.publickey import PublicKey
from solmate.dtypes import UnixTimestamp

# LOCK-END


# LOCK-BEGIN[class(DerivativeMetadata)]: DON'T MODIFY
@pod
class DerivativeMetadata:
    tag: AccountTag
    expired: ExpirationStatus
    oracle_type: OracleType
    instrument_type: InstrumentType
    bump: U64
    strike: Fractional
    initialization_time: UnixTimestamp
    full_funding_period: UnixTimestamp
    minimum_funding_period: UnixTimestamp
    price_oracle: PublicKey
    market_product_group: PublicKey
    close_authority: PublicKey
    clock: PublicKey
    last_funding_time: UnixTimestamp
    # LOCK-END

    @classmethod
    def to_bytes(cls, obj, **kwargs):
        return cls.pack(obj, converter="bytes", **kwargs)

    @classmethod
    def from_bytes(cls, raw, **kwargs):
        return cls.unpack(raw, converter="bytes", **kwargs)
