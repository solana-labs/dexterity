# LOCK-BEGIN[imports]: DON'T MODIFY
from .instruction_tag import InstructionTag
from dataclasses import dataclass
from dexterity.codegen.dex.types import InitializeMarketProductParams
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


# LOCK-BEGIN[ix_cls(initialize_market_product)]: DON'T MODIFY
@dataclass
class InitializeMarketProductIx:
    program_id: PublicKey

    # account metas
    authority: AccountMeta
    market_product_group: AccountMeta
    product: AccountMeta
    orderbook: AccountMeta
    remaining_accounts: Optional[List[AccountMeta]]

    # data fields
    params: InitializeMarketProductParams

    def to_instruction(self):
        keys = []
        keys.append(self.authority)
        keys.append(self.market_product_group)
        keys.append(self.product)
        keys.append(self.orderbook)
        if self.remaining_accounts is not None:
            keys.extend(self.remaining_accounts)

        buffer = BytesIO()
        buffer.write(InstructionTag.to_bytes(InstructionTag.INITIALIZE_MARKET_PRODUCT))
        buffer.write(BYTES_CATALOG.pack(InitializeMarketProductParams, self.params))

        return TransactionInstruction(
            keys=keys,
            program_id=self.program_id,
            data=buffer.getvalue(),
        )

# LOCK-END


# LOCK-BEGIN[ix_fn(initialize_market_product)]: DON'T MODIFY
def initialize_market_product(
    authority: Union[str, PublicKey, AccountMeta],
    market_product_group: Union[str, PublicKey, AccountMeta],
    product: Union[str, PublicKey, AccountMeta],
    orderbook: Union[str, PublicKey, AccountMeta],
    params: InitializeMarketProductParams,
    remaining_accounts: Optional[List[AccountMeta]] = None,
    program_id: Optional[PublicKey] = None,
):
    if program_id is None:
        program_id = PublicKey("Dex1111111111111111111111111111111111111111")

    if isinstance(authority, (str, PublicKey)):
        authority = to_account_meta(
            authority,
            is_signer=True,
            is_writable=False,
        )
    if isinstance(market_product_group, (str, PublicKey)):
        market_product_group = to_account_meta(
            market_product_group,
            is_signer=False,
            is_writable=True,
        )
    if isinstance(product, (str, PublicKey)):
        product = to_account_meta(
            product,
            is_signer=False,
            is_writable=False,
        )
    if isinstance(orderbook, (str, PublicKey)):
        orderbook = to_account_meta(
            orderbook,
            is_signer=False,
            is_writable=False,
        )

    return InitializeMarketProductIx(
        program_id=program_id,
        authority=authority,
        market_product_group=market_product_group,
        product=product,
        orderbook=orderbook,
        remaining_accounts=remaining_accounts,
        params=params,
    ).to_instruction()

# LOCK-END
