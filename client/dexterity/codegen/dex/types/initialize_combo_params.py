# LOCK-BEGIN[imports]: DON'T MODIFY
from dexterity.codegen.dex.types.fractional import Fractional
from podite import (
    FixedLenArray,
    I8,
    U64,
    U8,
    Vec,
    pod,
)

# LOCK-END


# LOCK-BEGIN[class(InitializeComboParams)]: DON'T MODIFY
@pod
class InitializeComboParams:
    name: FixedLenArray[U8, 16]
    tick_size: Fractional
    price_offset: Fractional
    base_decimals: U64
    ratios: Vec[I8]
    # LOCK-END

    @classmethod
    def to_bytes(cls, obj, **kwargs):
        return cls.pack(obj, converter="bytes", **kwargs)

    @classmethod
    def from_bytes(cls, raw, **kwargs):
        return cls.unpack(raw, converter="bytes", **kwargs)
