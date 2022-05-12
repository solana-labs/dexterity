# LOCK-BEGIN[imports]: DON'T MODIFY
from dexterity.codegen.dex.types.health_result import HealthResult
from podite import pod

# LOCK-END


# LOCK-BEGIN[class(RiskOutputRegister)]: DON'T MODIFY
@pod
class RiskOutputRegister:
    risk_engine_output: HealthResult
    # LOCK-END

    @classmethod
    def to_bytes(cls, obj, **kwargs):
        return cls.pack(obj, converter="bytes", **kwargs)

    @classmethod
    def from_bytes(cls, raw, **kwargs):
        return cls.unpack(raw, converter="bytes", **kwargs)
