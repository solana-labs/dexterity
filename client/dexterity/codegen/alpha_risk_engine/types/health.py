# LOCK-BEGIN[imports]: DON'T MODIFY
from dexterity.codegen.dex.types.fractional import Fractional
from podite import (
    Vec,
    pod,
)

# LOCK-END


# LOCK-BEGIN[class(Health)]: DON'T MODIFY
@pod
class Health:
    margin_req: Fractional
    portfolio_value: Fractional
    total_abs_dollar_position: Fractional
    abs_dollar_position: Vec[Fractional]
    # LOCK-END

    @classmethod
    def to_bytes(cls, obj, **kwargs):
        return cls.pack(obj, converter="bytes", **kwargs)

    @classmethod
    def from_bytes(cls, raw, **kwargs):
        return cls.unpack(raw, converter="bytes", **kwargs)
