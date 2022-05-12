# LOCK-BEGIN[imports]: DON'T MODIFY
from dexterity.codegen.dex.types.account_tag import AccountTag
from dexterity.codegen.dex.types.fractional import Fractional
from dexterity.codegen.dex.types.open_orders import OpenOrders
from dexterity.codegen.dex.types.trader_position import TraderPosition
from podite import (
    FixedLenArray,
    I32,
    U128,
    U8,
    pod,
)
from solana.publickey import PublicKey
from solmate.dtypes import UnixTimestamp

# LOCK-END


# LOCK-BEGIN[class(TraderRiskGroup)]: DON'T MODIFY
@pod
class TraderRiskGroup:
    tag: AccountTag
    market_product_group: PublicKey
    owner: PublicKey
    active_products: FixedLenArray[U8, 128]
    total_deposited: Fractional
    total_withdrawn: Fractional
    cash_balance: Fractional
    pending_cash_balance: Fractional
    pending_fees: Fractional
    valid_until: UnixTimestamp
    maker_fee_bps: I32
    taker_fee_bps: I32
    trader_positions: FixedLenArray[TraderPosition, 16]
    risk_state_account: PublicKey
    fee_state_account: PublicKey
    client_order_id: U128
    open_orders: OpenOrders
    # LOCK-END

    @classmethod
    def to_bytes(cls, obj, **kwargs):
        return cls.pack(obj, converter="bytes", **kwargs)

    @classmethod
    def from_bytes(cls, raw, **kwargs):
        return cls.unpack(raw, converter="bytes", **kwargs)
