# LOCK-BEGIN[imports]: DON'T MODIFY
from podite import (
    I64,
    pod,
)
from solana.publickey import PublicKey
from solmate.dtypes import Usize

# LOCK-END


# LOCK-BEGIN[class(Leg)]: DON'T MODIFY
@pod
class Leg:
    product_index: Usize
    product_key: PublicKey
    ratio: I64
    # LOCK-END

    @classmethod
    def to_bytes(cls, obj, **kwargs):
        return cls.pack(obj, converter="bytes", **kwargs)

    @classmethod
    def from_bytes(cls, raw, **kwargs):
        return cls.unpack(raw, converter="bytes", **kwargs)
