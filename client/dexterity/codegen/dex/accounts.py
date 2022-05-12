# LOCK-BEGIN[imports]: DON'T MODIFY
from dexterity.codegen.dex.types.market_product_group import MarketProductGroup
from dexterity.codegen.dex.types.product_array import ProductArray
from dexterity.codegen.dex.types.risk_output_register import RiskOutputRegister
from dexterity.codegen.dex.types.trader_risk_group import TraderRiskGroup
from podite import (
    Enum,
    U64,
    pod,
)
from solmate.anchor import AccountDiscriminant

# LOCK-END

from podite import AutoTagTypeValueManager

# LOCK-BEGIN[accounts]: DON'T MODIFY
@pod
class Accounts(Enum[U64]):
    MARKET_PRODUCT_GROUP = AccountDiscriminant(field=MarketProductGroup)
    PRODUCT_ARRAY = AccountDiscriminant(field=ProductArray)
    RISK_OUTPUT_REGISTER = AccountDiscriminant(field=RiskOutputRegister)
    TRADER_RISK_GROUP = AccountDiscriminant(field=TraderRiskGroup)
    # LOCK-END

    @classmethod
    def to_bytes(cls, obj, **kwargs):
        return cls.pack(obj, converter="bytes", **kwargs)

    @classmethod
    def from_bytes(cls, raw, **kwargs):
        return cls.unpack(raw, converter="bytes", format="FORMAT_ZERO_COPY", **kwargs)

    @classmethod
    def _from_bytes_partial(cls, buffer, format="FORMAT_ZERO_COPY", **kwargs):
        # accounts don't have the same size variants, so must manually use zero-copy and
        # set auto tag type.
        # TODO make this api cleaner
        with AutoTagTypeValueManager(U64):
            return super()._inner_from_bytes_partial(buffer, format="FORMAT_ZERO_COPY", **kwargs)
