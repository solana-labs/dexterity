# LOCK-BEGIN[imports]: DON'T MODIFY
from dexterity.codegen.dex.types.open_orders_metadata import OpenOrdersMetadata
from dexterity.codegen.dex.types.open_orders_node import OpenOrdersNode
from podite import (
    FixedLenArray,
    U64,
    pod,
)
from solmate.dtypes import Usize

# LOCK-END


# LOCK-BEGIN[class(OpenOrders)]: DON'T MODIFY
@pod
class OpenOrders:
    free_list_head: Usize
    total_open_orders: U64
    products: FixedLenArray[OpenOrdersMetadata, 256]
    orders: FixedLenArray["OpenOrdersNode", 1024]
    # LOCK-END

    @classmethod
    def to_bytes(cls, obj, **kwargs):
        return cls.pack(obj, converter="bytes", **kwargs)

    @classmethod
    def from_bytes(cls, raw, **kwargs):
        return cls.unpack(raw, converter="bytes", **kwargs)
