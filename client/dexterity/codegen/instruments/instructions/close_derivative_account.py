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


# LOCK-BEGIN[ix_cls(close_derivative_account)]: DON'T MODIFY
@dataclass
class CloseDerivativeAccountIx:
    program_id: PublicKey

    # account metas
    derivative_metadata: AccountMeta
    close_authority: AccountMeta
    destination: AccountMeta
    remaining_accounts: Optional[List[AccountMeta]]

    def to_instruction(self):
        keys = []
        keys.append(self.derivative_metadata)
        keys.append(self.close_authority)
        keys.append(self.destination)
        if self.remaining_accounts is not None:
            keys.extend(self.remaining_accounts)

        buffer = BytesIO()
        buffer.write(InstructionTag.to_bytes(InstructionTag.CLOSE_DERIVATIVE_ACCOUNT))

        return TransactionInstruction(
            keys=keys,
            program_id=self.program_id,
            data=buffer.getvalue(),
        )

# LOCK-END


# LOCK-BEGIN[ix_fn(close_derivative_account)]: DON'T MODIFY
def close_derivative_account(
    derivative_metadata: Union[str, PublicKey, AccountMeta],
    close_authority: Union[str, PublicKey, AccountMeta],
    destination: Union[str, PublicKey, AccountMeta],
    remaining_accounts: Optional[List[AccountMeta]] = None,
    program_id: Optional[PublicKey] = None,
):
    if program_id is None:
        program_id = PublicKey("instruments11111111111111111111111111111111")

    if isinstance(derivative_metadata, (str, PublicKey)):
        derivative_metadata = to_account_meta(
            derivative_metadata,
            is_signer=False,
            is_writable=True,
        )
    if isinstance(close_authority, (str, PublicKey)):
        close_authority = to_account_meta(
            close_authority,
            is_signer=True,
            is_writable=False,
        )
    if isinstance(destination, (str, PublicKey)):
        destination = to_account_meta(
            destination,
            is_signer=False,
            is_writable=False,
        )

    return CloseDerivativeAccountIx(
        program_id=program_id,
        derivative_metadata=derivative_metadata,
        close_authority=close_authority,
        destination=destination,
        remaining_accounts=remaining_accounts,
    ).to_instruction()

# LOCK-END
