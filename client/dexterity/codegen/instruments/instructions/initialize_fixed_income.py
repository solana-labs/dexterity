# LOCK-BEGIN[imports]: DON'T MODIFY
from .instruction_tag import InstructionTag
from dataclasses import dataclass
from dexterity.codegen.instruments.types import InitializeFixedIncomeParams
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


# LOCK-BEGIN[ix_cls(initialize_fixed_income)]: DON'T MODIFY
@dataclass
class InitializeFixedIncomeIx:
    program_id: PublicKey

    # account metas
    fixed_income_metadata: AccountMeta
    market_product_group: AccountMeta
    payer: AccountMeta
    system_program: AccountMeta
    remaining_accounts: Optional[List[AccountMeta]]

    # data fields
    params: InitializeFixedIncomeParams

    def to_instruction(self):
        keys = []
        keys.append(self.fixed_income_metadata)
        keys.append(self.market_product_group)
        keys.append(self.payer)
        keys.append(self.system_program)
        if self.remaining_accounts is not None:
            keys.extend(self.remaining_accounts)

        buffer = BytesIO()
        buffer.write(InstructionTag.to_bytes(InstructionTag.INITIALIZE_FIXED_INCOME))
        buffer.write(BYTES_CATALOG.pack(InitializeFixedIncomeParams, self.params))

        return TransactionInstruction(
            keys=keys,
            program_id=self.program_id,
            data=buffer.getvalue(),
        )

# LOCK-END


# LOCK-BEGIN[ix_fn(initialize_fixed_income)]: DON'T MODIFY
def initialize_fixed_income(
    fixed_income_metadata: Union[str, PublicKey, AccountMeta],
    market_product_group: Union[str, PublicKey, AccountMeta],
    payer: Union[str, PublicKey, AccountMeta],
    params: InitializeFixedIncomeParams,
    system_program: Union[str, PublicKey, AccountMeta] = PublicKey("11111111111111111111111111111111"),
    remaining_accounts: Optional[List[AccountMeta]] = None,
    program_id: Optional[PublicKey] = None,
):
    if program_id is None:
        program_id = PublicKey("EF5kuCdtoPa6Y9J4YMut4YMBvWa17JAebpJCo5LHig9a")

    if isinstance(fixed_income_metadata, (str, PublicKey)):
        fixed_income_metadata = to_account_meta(
            fixed_income_metadata,
            is_signer=False,
            is_writable=True,
        )
    if isinstance(market_product_group, (str, PublicKey)):
        market_product_group = to_account_meta(
            market_product_group,
            is_signer=False,
            is_writable=False,
        )
    if isinstance(payer, (str, PublicKey)):
        payer = to_account_meta(
            payer,
            is_signer=True,
            is_writable=False,
        )
    if isinstance(system_program, (str, PublicKey)):
        system_program = to_account_meta(
            system_program,
            is_signer=False,
            is_writable=False,
        )

    return InitializeFixedIncomeIx(
        program_id=program_id,
        fixed_income_metadata=fixed_income_metadata,
        market_product_group=market_product_group,
        payer=payer,
        system_program=system_program,
        remaining_accounts=remaining_accounts,
        params=params,
    ).to_instruction()

# LOCK-END
