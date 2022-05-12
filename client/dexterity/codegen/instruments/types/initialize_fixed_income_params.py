# LOCK-BEGIN[imports]: DON'T MODIFY
from podite import (
    FixedLenArray,
    U64,
    pod,
)
from solana.publickey import PublicKey
from solmate.dtypes import UnixTimestamp

# LOCK-END


# LOCK-BEGIN[class(InitializeFixedIncomeParams)]: DON'T MODIFY
@pod
class InitializeFixedIncomeParams:
    face_value: U64
    coupon_rate: U64
    initialization_time: UnixTimestamp
    coupon_dates: FixedLenArray[UnixTimestamp, 32]
    maturity_date: UnixTimestamp
    close_authority: PublicKey
    # LOCK-END

    @classmethod
    def to_bytes(cls, obj, **kwargs):
        return cls.pack(obj, converter="bytes", **kwargs)

    @classmethod
    def from_bytes(cls, raw, **kwargs):
        return cls.unpack(raw, converter="bytes", **kwargs)
