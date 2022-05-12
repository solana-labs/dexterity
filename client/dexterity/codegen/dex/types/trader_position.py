# LOCK-BEGIN[imports]: DON'T MODIFY
from dexterity.codegen.dex.types.account_tag import AccountTag
from dexterity.codegen.dex.types.fractional import Fractional
from podite import pod
from solana.publickey import PublicKey
from solmate.dtypes import Usize

# LOCK-END


# LOCK-BEGIN[class(TraderPosition)]: DON'T MODIFY
@pod
class TraderPosition:
    tag: "AccountTag"
    product_key: PublicKey
    position: "Fractional"
    pending_position: "Fractional"
    product_index: Usize
    last_cum_funding_snapshot: "Fractional"
    last_social_loss_snapshot: "Fractional"
    # LOCK-END

    @classmethod
    def to_bytes(cls, obj, **kwargs):
        return cls.pack(obj, converter="bytes", **kwargs)

    @classmethod
    def from_bytes(cls, raw, **kwargs):
        return cls.unpack(raw, converter="bytes", **kwargs)
