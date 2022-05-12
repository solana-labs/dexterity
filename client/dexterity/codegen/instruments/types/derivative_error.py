# LOCK-BEGIN[imports]: DON'T MODIFY
from podite import (
    AutoTagType,
    Enum,
    pod,
)

# LOCK-END


# LOCK-BEGIN[class(DerivativeError)]: DON'T MODIFY
@pod
class DerivativeError(Enum[AutoTagType]):
    ACCOUNT_ALREADY_INITIALIZED = None
    INVALID_SETTLEMENT_TIME = None
    INVALID_CREATION_TIME = None
    UNINITIALIZED_ACCOUNT = None
    INVALID_SEQUENCE_NUMBER = None
    UNSETTLED_ACCOUNTS = None
    INVALID_ORACLE_CONFIG = None
    NUMERICAL_OVERFLOW = None
    CANNOT_BE_DELETED = None
    CONTRACT_IS_EXPIRED = None
    INVALID_DATE = None
    INVALID_ACCOUNT = None
    # LOCK-END

    @classmethod
    def _to_bytes_partial(cls, buffer, obj, **kwargs):
        # to modify packing, change this method
        return super()._to_bytes_partial(buffer, obj, **kwargs)

    @classmethod
    def _from_bytes_partial(cls, buffer, **kwargs):
        # to modify unpacking, change this method
        return super()._from_bytes_partial(buffer, **kwargs)

    @classmethod
    def to_bytes(cls, obj, **kwargs):
        return cls.pack(obj, converter="bytes", **kwargs)

    @classmethod
    def from_bytes(cls, raw, **kwargs):
        return cls.unpack(raw, converter="bytes", **kwargs)
