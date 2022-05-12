# LOCK-BEGIN[imports]: DON'T MODIFY
from .instruction_tag import InstructionTag
from dataclasses import dataclass
from dexterity.codegen.dex.types import ClearExpiredOrderbookParams
from io import BytesIO
from podite import BYTES_CATALOG
from solana.publickey import PublicKey
from solana.transaction import (
    AccountMeta,
    TransactionInstruction,
)
from solmate.utils import to_account_meta
from typing import (
    List,
    Optional,
    Union,
)

# LOCK-END


# LOCK-BEGIN[ix_cls(clear_expired_orderbook)]: DON'T MODIFY
@dataclass
class ClearExpiredOrderbookIx:
    program_id: PublicKey

    # account metas
    market_product_group: AccountMeta
    product: AccountMeta
    aaob_program: AccountMeta
    orderbook: AccountMeta
    market_signer: AccountMeta
    event_queue: AccountMeta
    bids: AccountMeta
    asks: AccountMeta
    remaining_accounts: Optional[List[AccountMeta]]

    # data fields
    params: ClearExpiredOrderbookParams

    def to_instruction(self):
        keys = []
        keys.append(self.market_product_group)
        keys.append(self.product)
        keys.append(self.aaob_program)
        keys.append(self.orderbook)
        keys.append(self.market_signer)
        keys.append(self.event_queue)
        keys.append(self.bids)
        keys.append(self.asks)
        if self.remaining_accounts is not None:
            keys.extend(self.remaining_accounts)

        buffer = BytesIO()
        buffer.write(InstructionTag.to_bytes(InstructionTag.CLEAR_EXPIRED_ORDERBOOK))
        buffer.write(BYTES_CATALOG.pack(ClearExpiredOrderbookParams, self.params))

        return TransactionInstruction(
            keys=keys,
            program_id=self.program_id,
            data=buffer.getvalue(),
        )

# LOCK-END


# LOCK-BEGIN[ix_fn(clear_expired_orderbook)]: DON'T MODIFY
def clear_expired_orderbook(
    market_product_group: Union[str, PublicKey, AccountMeta],
    product: Union[str, PublicKey, AccountMeta],
    aaob_program: Union[str, PublicKey, AccountMeta],
    orderbook: Union[str, PublicKey, AccountMeta],
    market_signer: Union[str, PublicKey, AccountMeta],
    event_queue: Union[str, PublicKey, AccountMeta],
    bids: Union[str, PublicKey, AccountMeta],
    asks: Union[str, PublicKey, AccountMeta],
    params: ClearExpiredOrderbookParams,
    remaining_accounts: Optional[List[AccountMeta]] = None,
    program_id: Optional[PublicKey] = None,
):
    if program_id is None:
        program_id = PublicKey("Dex1111111111111111111111111111111111111111")

    if isinstance(market_product_group, (str, PublicKey)):
        market_product_group = to_account_meta(
            market_product_group,
            is_signer=False,
            is_writable=False,
        )
    if isinstance(product, (str, PublicKey)):
        product = to_account_meta(
            product,
            is_signer=False,
            is_writable=False,
        )
    if isinstance(aaob_program, (str, PublicKey)):
        aaob_program = to_account_meta(
            aaob_program,
            is_signer=False,
            is_writable=False,
        )
    if isinstance(orderbook, (str, PublicKey)):
        orderbook = to_account_meta(
            orderbook,
            is_signer=False,
            is_writable=True,
        )
    if isinstance(market_signer, (str, PublicKey)):
        market_signer = to_account_meta(
            market_signer,
            is_signer=False,
            is_writable=False,
        )
    if isinstance(event_queue, (str, PublicKey)):
        event_queue = to_account_meta(
            event_queue,
            is_signer=False,
            is_writable=True,
        )
    if isinstance(bids, (str, PublicKey)):
        bids = to_account_meta(
            bids,
            is_signer=False,
            is_writable=True,
        )
    if isinstance(asks, (str, PublicKey)):
        asks = to_account_meta(
            asks,
            is_signer=False,
            is_writable=True,
        )

    return ClearExpiredOrderbookIx(
        program_id=program_id,
        market_product_group=market_product_group,
        product=product,
        aaob_program=aaob_program,
        orderbook=orderbook,
        market_signer=market_signer,
        event_queue=event_queue,
        bids=bids,
        asks=asks,
        remaining_accounts=remaining_accounts,
        params=params,
    ).to_instruction()

# LOCK-END
