# LOCK-BEGIN[imports]: DON'T MODIFY
from dexterity.codegen.instruments.types.account_tag import AccountTag
from dexterity.codegen.instruments.types.expiration_status import ExpirationStatus
from podite import (
    FixedLenArray,
    U64,
    pod,
)
from solana.publickey import PublicKey
from solmate.dtypes import UnixTimestamp

# LOCK-END


# LOCK-BEGIN[class(FixedIncomeMetadata)]: DON'T MODIFY
@pod
class FixedIncomeMetadata:
    tag: AccountTag
    bump: U64
    face_value: U64
    coupon_rate: U64
    initialization_time: UnixTimestamp
    coupon_dates: FixedLenArray[UnixTimestamp, 32]
    maturity_date: UnixTimestamp
    market_product_group: PublicKey
    close_authority: PublicKey
    last_funding_time: UnixTimestamp
    expired: ExpirationStatus
    # LOCK-END

    @classmethod
    def to_bytes(cls, obj, **kwargs):
        return cls.pack(obj, converter="bytes", **kwargs)

    @classmethod
    def from_bytes(cls, raw, **kwargs):
        return cls.unpack(raw, converter="bytes", **kwargs)
