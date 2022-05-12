# LOCK-BEGIN[imports]: DON'T MODIFY
from dexterity.codegen.dex.types.action_status import ActionStatus
from dexterity.codegen.dex.types.health_status import HealthStatus
from podite import pod

# LOCK-END


# LOCK-BEGIN[class(HealthInfo)]: DON'T MODIFY
@pod
class HealthInfo:
    health: "HealthStatus"
    action: "ActionStatus"
    # LOCK-END

    @classmethod
    def to_bytes(cls, obj, **kwargs):
        return cls.pack(obj, converter="bytes", **kwargs)

    @classmethod
    def from_bytes(cls, raw, **kwargs):
        return cls.unpack(raw, converter="bytes", **kwargs)
