# LOCK-BEGIN[imports]: DON'T MODIFY
from dexterity.codegen.dex.types.fractional import Fractional
from podite import pod
from solmate.dtypes import Usize

# LOCK-END


# LOCK-BEGIN[class(SocialLoss)]: DON'T MODIFY
@pod
class SocialLoss:
    product_index: Usize
    amount: "Fractional"
    # LOCK-END

    @classmethod
    def to_bytes(cls, obj, **kwargs):
        return cls.pack(obj, converter="bytes", **kwargs)

    @classmethod
    def from_bytes(cls, raw, **kwargs):
        return cls.unpack(raw, converter="bytes", **kwargs)
