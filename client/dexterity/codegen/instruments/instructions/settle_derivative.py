# LOCK-BEGIN[imports]: DON'T MODIFY
from .instruction_tag import InstructionTag
from dataclasses import dataclass
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


# LOCK-BEGIN[ix_cls(settle_derivative)]: DON'T MODIFY
@dataclass
class SettleDerivativeIx:
    program_id: PublicKey

    # account metas
    market_product_group: AccountMeta
    derivative_metadata: AccountMeta
    price_oracle: AccountMeta
    dex_program: AccountMeta
    clock: AccountMeta
    remaining_accounts: Optional[List[AccountMeta]]

    def to_instruction(self):
        keys = []
        keys.append(self.market_product_group)
        keys.append(self.derivative_metadata)
        keys.append(self.price_oracle)
        keys.append(self.dex_program)
        keys.append(self.clock)
        if self.remaining_accounts is not None:
            keys.extend(self.remaining_accounts)

        buffer = BytesIO()
        buffer.write(InstructionTag.to_bytes(InstructionTag.SETTLE_DERIVATIVE))

        return TransactionInstruction(
            keys=keys,
            program_id=self.program_id,
            data=buffer.getvalue(),
        )

# LOCK-END


# LOCK-BEGIN[ix_fn(settle_derivative)]: DON'T MODIFY
def settle_derivative(
    market_product_group: Union[str, PublicKey, AccountMeta],
    derivative_metadata: Union[str, PublicKey, AccountMeta],
    price_oracle: Union[str, PublicKey, AccountMeta],
    dex_program: Union[str, PublicKey, AccountMeta],
    clock: Union[str, PublicKey, AccountMeta],
    remaining_accounts: Optional[List[AccountMeta]] = None,
    program_id: Optional[PublicKey] = None,
):
    if program_id is None:
        program_id = PublicKey("instruments11111111111111111111111111111111")

    if isinstance(market_product_group, (str, PublicKey)):
        market_product_group = to_account_meta(
            market_product_group,
            is_signer=False,
            is_writable=True,
        )
    if isinstance(derivative_metadata, (str, PublicKey)):
        derivative_metadata = to_account_meta(
            derivative_metadata,
            is_signer=False,
            is_writable=True,
        )
    if isinstance(price_oracle, (str, PublicKey)):
        price_oracle = to_account_meta(
            price_oracle,
            is_signer=False,
            is_writable=False,
        )
    if isinstance(dex_program, (str, PublicKey)):
        dex_program = to_account_meta(
            dex_program,
            is_signer=False,
            is_writable=False,
        )
    if isinstance(clock, (str, PublicKey)):
        clock = to_account_meta(
            clock,
            is_signer=False,
            is_writable=False,
        )

    return SettleDerivativeIx(
        program_id=program_id,
        market_product_group=market_product_group,
        derivative_metadata=derivative_metadata,
        price_oracle=price_oracle,
        dex_program=dex_program,
        clock=clock,
        remaining_accounts=remaining_accounts,
    ).to_instruction()

# LOCK-END
