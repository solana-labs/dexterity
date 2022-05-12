# LOCK-BEGIN[imports]: DON'T MODIFY
from podite import (
    FixedLenArray,
    I16,
    U64,
    U8,
    pod,
)

# LOCK-END


# LOCK-BEGIN[class(InitializeMarketProductGroupParams)]: DON'T MODIFY
@pod
class InitializeMarketProductGroupParams:
    name: FixedLenArray[U8, 16]
    validate_account_discriminant_len: U64
    find_fees_discriminant_len: U64
    validate_account_health_discriminant: FixedLenArray[U8, 8]
    validate_account_liquidation_discriminant: FixedLenArray[U8, 8]
    create_risk_state_account_discriminant: FixedLenArray[U8, 8]
    find_fees_discriminant: FixedLenArray[U8, 8]
    max_maker_fee_bps: I16
    min_maker_fee_bps: I16
    max_taker_fee_bps: I16
    min_taker_fee_bps: I16
    # LOCK-END

    @classmethod
    def to_bytes(cls, obj, **kwargs):
        return cls.pack(obj, converter="bytes", **kwargs)

    @classmethod
    def from_bytes(cls, raw, **kwargs):
        return cls.unpack(raw, converter="bytes", **kwargs)
