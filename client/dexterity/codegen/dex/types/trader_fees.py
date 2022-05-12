# LOCK-BEGIN[imports]: DON'T MODIFY
from podite import (
    I32,
    pod,
)
from solmate.dtypes import UnixTimestamp

# LOCK-END


# LOCK-BEGIN[class(TraderFees)]: DON'T MODIFY
@pod
class TraderFees:
    valid_until: UnixTimestamp
    maker_fee_bps: I32
    taker_fee_bps: I32
    # LOCK-END

    @classmethod
    def to_bytes(cls, obj, **kwargs):
        return cls.pack(obj, converter="bytes", **kwargs)

    @classmethod
    def from_bytes(cls, raw, **kwargs):
        return cls.unpack(raw, converter="bytes", **kwargs)
