# LOCK-BEGIN[imports]: DON'T MODIFY
from dexterity.codegen.dex.types.fractional import Fractional
from podite import (
    FixedLenArray,
    U64,
    U8,
    pod,
)

# LOCK-END


# LOCK-BEGIN[class(InitializeMarketProductParams)]: DON'T MODIFY
@pod
class InitializeMarketProductParams:
    name: FixedLenArray[U8, 16]
    tick_size: Fractional
    base_decimals: U64
    price_offset: Fractional
    # LOCK-END

    @classmethod
    def to_bytes(cls, obj, **kwargs):
        return cls.pack(obj, converter="bytes", **kwargs)

    @classmethod
    def from_bytes(cls, raw, **kwargs):
        return cls.unpack(raw, converter="bytes", **kwargs)
