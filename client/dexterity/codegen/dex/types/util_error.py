# LOCK-BEGIN[imports]: DON'T MODIFY
from podite import (
    AutoTagType,
    Enum,
    pod,
)

# LOCK-END


# LOCK-BEGIN[class(UtilError)]: DON'T MODIFY
@pod
class UtilError(Enum[AutoTagType]):
    ACCOUNT_ALREADY_INITIALIZED = None
    ACCOUNT_UNINITIALIZED = None
    DUPLICATE_PRODUCT_KEY = None
    PUBLIC_KEY_MISMATCH = None
    ASSERTION_ERROR = None
    INVALID_MINT_AUTHORITY = None
    INCORRECT_OWNER = None
    PUBLIC_KEYS_SHOULD_BE_UNIQUE = None
    NOT_RENT_EXEMPT = None
    NUMERICAL_OVERFLOW = None
    ROUND_ERROR = None
    DIVISIONBY_ZERO = None
    INVALID_RETURN_VALUE = None
    SQRT_ROOT_ERROR = None
    ZERO_PRICE_ERROR = None
    ZERO_QUANTITY_ERROR = None
    SERIALIZE_ERROR = None
    DESERIALIZE_ERROR = None
    INVALID_BITSET_INDEX = None
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
