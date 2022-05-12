import struct
from numpy import isin
import pandas as pd
from enum import IntEnum
from dataclasses import field
from typing import Union
from io import BytesIO

from solana.publickey import PublicKey


from podite import (
    U8,
    pod,
    U32,
    U64,
    U128,
    Enum,
    Option,
    FixedLenArray,
    BYTES_CATALOG,
)

from .base import AccountTag, Side

NODE_SIZE = 32
FREE_NODE_SIZE = 4

NODE_TAG_SIZE = 8
SLOT_SIZE = NODE_TAG_SIZE + NODE_SIZE

SLAB_HEADER_LEN = 97
PADDED_SLAB_HEADER_LEN = SLAB_HEADER_LEN + 7

BINARY_ORDER_SCALE = 32


@pod
class InnerNode:
    prefix_len: U64
    key: U128
    children: FixedLenArray[U32, 2]


@pod
class LeafNode:
    key: U128
    callback_info_pt: U64
    base_quantity: U64

    @property
    def price(self):
        return self.key >> (64 + BINARY_ORDER_SCALE)

    @property
    def order_id(self):
        return self.key

    def set_base_quantity(self, quantity):
        self.base_quantity = quantity
        return None


@pod
class FreeNode:
    next: U32


@pod
class NodeKind(Enum[U64]):
    Uninitialized = None
    Inner = None
    Leaf = None
    Free = None
    LastFree = None


@pod
class Node:
    kind: NodeKind
    node_data: Union[InnerNode, LeafNode, FreeNode, None]

    @classmethod
    def from_bytes_partial(cls, buffer):
        kind, buffer = BYTES_CATALOG.unpack_partial(NodeKind, BytesIO(buffer))
        if kind == NodeKind.Inner:
            node_data, _ = BYTES_CATALOG.unpack_partial(InnerNode, BytesIO(buffer))
        elif kind == NodeKind.Leaf:
            node_data, _ = BYTES_CATALOG.unpack_partial(LeafNode, BytesIO(buffer))
        elif (kind == NodeKind.Free) or (kind == NodeKind.LastFree):
            node_data, _ = BYTES_CATALOG.unpack_partial(FreeNode, BytesIO(buffer))
        else:
            node_data = None
        max_size = max(
            BYTES_CATALOG.calcmaxsize_for_type(InnerNode),
            BYTES_CATALOG.calcmaxsize_for_type(LeafNode),
            BYTES_CATALOG.calcmaxsize_for_type(FreeNode),
        )
        return Node(kind, node_data), buffer[max_size:]


@pod
class SlabHeader:
    account_tag: AccountTag
    bump_index: U64
    free_list_len: U64
    free_list_head: U32
    callback_memory_offset: U64
    callback_free_list_len: U64
    callback_free_list_head: U64
    callback_bump_index: U64
    root_node: U32
    leaf_count: U64
    market_address: PublicKey


@pod
class Slab:
    header: SlabHeader
    # register: Option[OrderSummary]
    buffer: bytes = field(repr=False)

    def __getitem__(self, idx):
        # if idx > self.header.leaf_count:
        #     raise ValueError("Index out of bound")

        start = PADDED_SLAB_HEADER_LEN + idx * SLOT_SIZE
        end = start + SLOT_SIZE
        chunk = self.buffer[start:end]
        return Node.from_bytes_partial(chunk)[0]

    @property
    def root(self):
        if self.header.leaf_count == 0:
            return None
        else:
            return self.header.root_node

    def get_node(self, key):
        start = PADDED_SLAB_HEADER_LEN + key * SLOT_SIZE
        end = start + NODE_SIZE + NODE_TAG_SIZE
        buffer = self.buffer[start:end]
        tag = struct.unpack("<Q", buffer[:8])[0]
        if tag == 1:
            return InnerNode.from_bytes(buffer[8:])
        if tag == 2:
            return LeafNode.from_bytes(buffer[8:])
        if tag == 3 or tag == 4:
            return FreeNode.from_bytes(buffer[8:])

    def find_min_max(self, find_max: bool):
        root = self.root
        if root is None:
            return None
        else:
            while 1:
                data_buffer = self.get_node(root)
                tag, data_buffer = BYTES_CATALOG.unpack_partial(
                    NodeKind, BytesIO(data_buffer)
                )
                if tag == NodeKind.Inner:
                    root_contents, _ = BYTES_CATALOG.unpack_partial(
                        InnerNode, BytesIO(data_buffer)
                    )
                    idx = 1 if find_max else 0
                    root = root_contents.children[idx]  # returns key
                elif (tag == NodeKind.Free) or (tag == NodeKind.LastFree):
                    root_contents, _ = BYTES_CATALOG.unpack_partial(
                        FreeNode, BytesIO(data_buffer)
                    )
                    return root_contents
                elif tag == NodeKind.Leaf:
                    root_contents, _ = BYTES_CATALOG.unpack_partial(
                        LeafNode, BytesIO(data_buffer)
                    )
                    return root_contents
                else:
                    return None

    def inorder_traversal(self, root, side: Side):
        node = self.get_node(root)
        res = []
        if isinstance(node, InnerNode):
            first = 0 if side == Side.BID else 1
            second = 1 - first

            first_child = node.children[first]
            second_child = node.children[second]
            res.extend(self.inorder_traversal(first_child, side))
            res.extend(self.inorder_traversal(second_child, side))

        elif isinstance(node, LeafNode):
            res.extend(
                [
                    (
                        node.price,
                        node.order_id,
                        node.base_quantity,
                    )
                ]
            )

        else:
            res.extend([(None, None, None)])

        return res

    def order_bookify(self, side: Side, group=False):
        root = self.root
        order_book = {}
        if root is None:
            return order_book
        else:
            orders = self.inorder_traversal(root, side)
            for ord in orders:
                price = ord[0]
                oid = ord[1]
                order_size = ord[-1]
                if (price, oid) in order_book:
                    order_book[(price, oid)][0] += order_size
                else:
                    order_book[(price, oid)] = [order_size]
        df = (
            pd.DataFrame(order_book)
            .T.reset_index()
            .rename(columns={"level_0": "Price", "level_1": "Oid", 0: "Qty"})
            .sort_values("Oid", ascending=False)
            .sort_values("Price", ascending=False)
            .reset_index(drop=True)
        )[["Oid", "Price", "Qty"]]
        if group:
            return df.groupby("Price").agg({"Qty": ["sum", "count"]})
        return df

    # key: U128
    # callback_info_pt: U64
    # base_quantity: U64

    @classmethod
    def from_bytes_partial(cls, buffer):
        header, _ = BYTES_CATALOG.unpack_partial(SlabHeader, BytesIO(buffer))

        return Slab(header, buffer), b""

    @classmethod
    def to_bytes_io(cls, obj, buffer):
        raise NotImplementedError
