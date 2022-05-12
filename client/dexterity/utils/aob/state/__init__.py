from .base import *
from .event_queue import *
from .market_state import *
from .slab import *


def account_parser(data):
    tag = AccountTag.from_bytes(data[:1], byteorder="little")
    if tag == AccountTag.UNINITIALIZED:
        return None
    elif tag == AccountTag.MARKET:
        return MarketState.from_bytes(data)
    elif tag == AccountTag.EVENT_QUEUE:
        return EventQueue.from_bytes(data)
    elif tag == AccountTag.BIDS:
        return Slab.from_bytes(data)
    elif tag == AccountTag.ASKS:
        return Slab.from_bytes(data)
    raise ValueError()
