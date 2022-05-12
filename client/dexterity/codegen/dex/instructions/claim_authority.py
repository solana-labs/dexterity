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


# LOCK-BEGIN[ix_cls(claim_authority)]: DON'T MODIFY
@dataclass
class ClaimAuthorityIx:
    program_id: PublicKey

    # account metas
    market_product_group: AccountMeta
    new_authority: AccountMeta
    remaining_accounts: Optional[List[AccountMeta]]

    def to_instruction(self):
        keys = []
        keys.append(self.market_product_group)
        keys.append(self.new_authority)
        if self.remaining_accounts is not None:
            keys.extend(self.remaining_accounts)

        buffer = BytesIO()
        buffer.write(InstructionTag.to_bytes(InstructionTag.CLAIM_AUTHORITY))

        return TransactionInstruction(
            keys=keys,
            program_id=self.program_id,
            data=buffer.getvalue(),
        )

# LOCK-END


# LOCK-BEGIN[ix_fn(claim_authority)]: DON'T MODIFY
def claim_authority(
    market_product_group: Union[str, PublicKey, AccountMeta],
    new_authority: Union[str, PublicKey, AccountMeta],
    remaining_accounts: Optional[List[AccountMeta]] = None,
    program_id: Optional[PublicKey] = None,
):
    if program_id is None:
        program_id = PublicKey("Dex1111111111111111111111111111111111111111")

    if isinstance(market_product_group, (str, PublicKey)):
        market_product_group = to_account_meta(
            market_product_group,
            is_signer=False,
            is_writable=True,
        )
    if isinstance(new_authority, (str, PublicKey)):
        new_authority = to_account_meta(
            new_authority,
            is_signer=True,
            is_writable=False,
        )

    return ClaimAuthorityIx(
        program_id=program_id,
        market_product_group=market_product_group,
        new_authority=new_authority,
        remaining_accounts=remaining_accounts,
    ).to_instruction()

# LOCK-END
