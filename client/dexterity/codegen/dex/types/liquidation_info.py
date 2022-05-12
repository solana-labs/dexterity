# LOCK-BEGIN[imports]: DON'T MODIFY
from dexterity.codegen.dex.types.action_status import ActionStatus
from dexterity.codegen.dex.types.fractional import Fractional
from dexterity.codegen.dex.types.health_status import HealthStatus
from dexterity.codegen.dex.types.social_loss import SocialLoss
from podite import (
    FixedLenArray,
    pod,
)

# LOCK-END


# LOCK-BEGIN[class(LiquidationInfo)]: DON'T MODIFY
@pod
class LiquidationInfo:
    health: "HealthStatus"
    action: "ActionStatus"
    total_social_loss: "Fractional"
    liquidation_price: "Fractional"
    social_losses: FixedLenArray["SocialLoss", 16]
    # LOCK-END

    @classmethod
    def to_bytes(cls, obj, **kwargs):
        return cls.pack(obj, converter="bytes", **kwargs)

    @classmethod
    def from_bytes(cls, raw, **kwargs):
        return cls.unpack(raw, converter="bytes", **kwargs)
