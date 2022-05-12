from enum import IntEnum
from dataclasses import field
from typing import Union, List

from solana.publickey import PublicKey

from dexterity.utils import Usize
from podite import (
    U8,
    U64,
    U128,
    Option,
    pod,
    Enum,
    BYTES_CATALOG
)

from .base import AccountTag, Side


@pod
class EventQueueHeader:
    tag: AccountTag
    head: U64
    count: U64
    event_size: U64
    seq_num: U64
    unknown: Usize


@pod
class OrderSummary:
    posted_order_id: Option[U128]
    total_base_qty: U64
    total_quote_qty: U64
    total_base_qty_posted: U64


class EventKind(Enum[U8]):
    FILL = 0
    OUT = 1


@pod
class Callback:
    user_account: PublicKey


@pod
class FillEventData:
    taker_side: Side
    maker_order_id: U128
    quote_size: U64
    base_size: U64
    maker_callback_info: Callback
    taker_callback_info: Callback


@pod
class OutEventData:
    side: Side
    order_id: U128
    base_size: U64
    delete: bool
    callback_info: Callback


@pod
class Event:
    kind: EventKind
    event_data: Union[FillEventData, OutEventData]

    @classmethod
    def from_bytes_partial(cls, buffer):
        kind, buffer = BYTES_CATALOG.deserialize_to_type(EventKind, buffer)
        if kind == EventKind.FILL:
            event_data, _ = BYTES_CATALOG.deserialize_to_type(FillEventData, buffer)
        else:
            event_data, _ = BYTES_CATALOG.deserialize_to_type(OutEventData, buffer)

        max_size = max(
            BYTES_CATALOG.calcmaxsize_for_type(FillEventData),
            BYTES_CATALOG.calcmaxsize_for_type(OutEventData),
        )

        return Event(kind, event_data), buffer[max_size:]

    @classmethod
    def to_bytes_io(cls, obj, buffer):
        raise NotImplementedError

    def get_user_accounts(self) -> List[PublicKey]:
        if isinstance(self.event_data, FillEventData):
            return [
                self.event_data.maker_callback_info.user_account,
                self.event_data.taker_callback_info.user_account,
            ]
        else:
            return [
                self.event_data.callback_info.user_account,
            ]


@pod
class EventQueue:
    header: EventQueueHeader
    register: Option[OrderSummary]
    remaining_buffer: bytes = field(repr=False)

    def __getitem__(self, idx) -> Event:
        if idx >= self.header.count:
            raise ValueError("Index out of bound")

        head = self.header.head
        size = self.header.event_size
        buf_len = len(self.remaining_buffer)

        start = (head + idx * size) % buf_len
        end = start + size
        chunk = self.remaining_buffer[start:end]
        return Event.from_bytes_partial(chunk)[0]

    @classmethod
    def from_bytes_partial(cls, buffer):
        header, buffer = BYTES_CATALOG.deserialize_to_type(EventQueueHeader, buffer)
        register, buffer = BYTES_CATALOG.deserialize_to_type(
            Option[OrderSummary], buffer
        )

        return EventQueue(header, register, buffer), b""

    @classmethod
    def to_bytes_io(cls, obj, buffer):
        raise NotImplementedError
