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


# LOCK-BEGIN[ix_cls(settle_fixed_income)]: DON'T MODIFY
@dataclass
class SettleFixedIncomeIx:
    program_id: PublicKey

    # account metas
    market_product_group: AccountMeta
    fixed_income_metadata: AccountMeta
    dex_program: AccountMeta
    remaining_accounts: Optional[List[AccountMeta]]

    def to_instruction(self):
        keys = []
        keys.append(self.market_product_group)
        keys.append(self.fixed_income_metadata)
        keys.append(self.dex_program)
        if self.remaining_accounts is not None:
            keys.extend(self.remaining_accounts)

        buffer = BytesIO()
        buffer.write(InstructionTag.to_bytes(InstructionTag.SETTLE_FIXED_INCOME))

        return TransactionInstruction(
            keys=keys,
            program_id=self.program_id,
            data=buffer.getvalue(),
        )

# LOCK-END


# LOCK-BEGIN[ix_fn(settle_fixed_income)]: DON'T MODIFY
def settle_fixed_income(
    market_product_group: Union[str, PublicKey, AccountMeta],
    fixed_income_metadata: Union[str, PublicKey, AccountMeta],
    dex_program: Union[str, PublicKey, AccountMeta],
    remaining_accounts: Optional[List[AccountMeta]] = None,
    program_id: Optional[PublicKey] = None,
):
    if program_id is None:
        program_id = PublicKey("EF5kuCdtoPa6Y9J4YMut4YMBvWa17JAebpJCo5LHig9a")

    if isinstance(market_product_group, (str, PublicKey)):
        market_product_group = to_account_meta(
            market_product_group,
            is_signer=False,
            is_writable=True,
        )
    if isinstance(fixed_income_metadata, (str, PublicKey)):
        fixed_income_metadata = to_account_meta(
            fixed_income_metadata,
            is_signer=False,
            is_writable=True,
        )
    if isinstance(dex_program, (str, PublicKey)):
        dex_program = to_account_meta(
            dex_program,
            is_signer=False,
            is_writable=False,
        )

    return SettleFixedIncomeIx(
        program_id=program_id,
        market_product_group=market_product_group,
        fixed_income_metadata=fixed_income_metadata,
        dex_program=dex_program,
        remaining_accounts=remaining_accounts,
    ).to_instruction()

# LOCK-END
