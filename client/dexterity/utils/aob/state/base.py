from enum import IntEnum

from podite import pod, U8, Enum, AutoTagType


CALLBACK_INFO_LEN = 32
ORDER_SUMMARY_SIZE = 41
EVENT_QUEUE_HEADER_LEN = 37
REGISTER_SIZE = ORDER_SUMMARY_SIZE + 1


@pod
class Side(Enum[AutoTagType]):
    BID = None
    ASK = None


@pod
class AccountTag(Enum[AutoTagType]):
    UNINITIALIZED = None
    MARKET = None
    EVENT_QUEUE = None
    BIDS = None
    ASKS = None


@pod
class SelfTradeBehavior(Enum[AutoTagType]):
    DECREMENT_TAKE = None
    CANCEL_PROVIDE = None
    ABORT_TRANSACTION = None
