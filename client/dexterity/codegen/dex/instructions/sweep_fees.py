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


# LOCK-BEGIN[ix_cls(sweep_fees)]: DON'T MODIFY
@dataclass
class SweepFeesIx:
    program_id: PublicKey

    # account metas
    market_product_group: AccountMeta
    fee_collector: AccountMeta
    market_product_group_vault: AccountMeta
    fee_collector_token_account: AccountMeta
    token_program: AccountMeta
    remaining_accounts: Optional[List[AccountMeta]]

    def to_instruction(self):
        keys = []
        keys.append(self.market_product_group)
        keys.append(self.fee_collector)
        keys.append(self.market_product_group_vault)
        keys.append(self.fee_collector_token_account)
        keys.append(self.token_program)
        if self.remaining_accounts is not None:
            keys.extend(self.remaining_accounts)

        buffer = BytesIO()
        buffer.write(InstructionTag.to_bytes(InstructionTag.SWEEP_FEES))

        return TransactionInstruction(
            keys=keys,
            program_id=self.program_id,
            data=buffer.getvalue(),
        )

# LOCK-END


# LOCK-BEGIN[ix_fn(sweep_fees)]: DON'T MODIFY
def sweep_fees(
    market_product_group: Union[str, PublicKey, AccountMeta],
    fee_collector: Union[str, PublicKey, AccountMeta],
    market_product_group_vault: Union[str, PublicKey, AccountMeta],
    fee_collector_token_account: Union[str, PublicKey, AccountMeta],
    token_program: Union[str, PublicKey, AccountMeta] = PublicKey("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"),
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
    if isinstance(fee_collector, (str, PublicKey)):
        fee_collector = to_account_meta(
            fee_collector,
            is_signer=False,
            is_writable=False,
        )
    if isinstance(market_product_group_vault, (str, PublicKey)):
        market_product_group_vault = to_account_meta(
            market_product_group_vault,
            is_signer=False,
            is_writable=True,
        )
    if isinstance(fee_collector_token_account, (str, PublicKey)):
        fee_collector_token_account = to_account_meta(
            fee_collector_token_account,
            is_signer=False,
            is_writable=True,
        )
    if isinstance(token_program, (str, PublicKey)):
        token_program = to_account_meta(
            token_program,
            is_signer=False,
            is_writable=False,
        )

    return SweepFeesIx(
        program_id=program_id,
        market_product_group=market_product_group,
        fee_collector=fee_collector,
        market_product_group_vault=market_product_group_vault,
        fee_collector_token_account=fee_collector_token_account,
        token_program=token_program,
        remaining_accounts=remaining_accounts,
    ).to_instruction()

# LOCK-END
