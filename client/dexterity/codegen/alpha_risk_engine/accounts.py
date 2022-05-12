# LOCK-BEGIN[imports]: DON'T MODIFY
from dexterity.codegen.alpha_risk_engine.types.health import Health
from podite import (
    Enum,
    U64,
    pod,
)
from solmate.anchor import AccountDiscriminant

# LOCK-END


# LOCK-BEGIN[accounts]: DON'T MODIFY
@pod
class Accounts(Enum[U64]):
    HEALTH = AccountDiscriminant(field=Health)
    # LOCK-END

    @classmethod
    def to_bytes(cls, obj, **kwargs):
        return cls.pack(obj, converter="bytes", **kwargs)

    @classmethod
    def from_bytes(cls, raw, **kwargs):
        return cls.unpack(raw, converter="bytes", **kwargs)
