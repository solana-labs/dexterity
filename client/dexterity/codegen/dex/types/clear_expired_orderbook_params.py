# LOCK-BEGIN[imports]: DON'T MODIFY
from podite import (
    U8,
    pod,
)

# LOCK-END


# LOCK-BEGIN[class(ClearExpiredOrderbookParams)]: DON'T MODIFY
@pod
class ClearExpiredOrderbookParams:
    num_orders_to_cancel: U8
    # LOCK-END

    @classmethod
    def to_bytes(cls, obj, **kwargs):
        return cls.pack(obj, converter="bytes", **kwargs)

    @classmethod
    def from_bytes(cls, raw, **kwargs):
        return cls.unpack(raw, converter="bytes", **kwargs)
