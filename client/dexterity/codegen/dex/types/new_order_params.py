# LOCK-BEGIN[imports]: DON'T MODIFY
from dexterity.codegen.dex.types.fractional import Fractional
from dexterity.codegen.dex.types.order_type import OrderType
from dexterity.utils.aob.state.base import (
    SelfTradeBehavior,
    Side,
)
from podite import (
    U64,
    pod,
)

# LOCK-END


# LOCK-BEGIN[class(NewOrderParams)]: DON'T MODIFY
@pod
class NewOrderParams:
    side: Side
    max_base_qty: Fractional
    order_type: "OrderType"
    self_trade_behavior: SelfTradeBehavior
    match_limit: U64
    limit_price: Fractional
    # LOCK-END

    @classmethod
    def to_bytes(cls, obj, **kwargs):
        return cls.pack(obj, converter="bytes", **kwargs)

    @classmethod
    def from_bytes(cls, raw, **kwargs):
        return cls.unpack(raw, converter="bytes", **kwargs)
