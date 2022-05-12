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


# LOCK-BEGIN[ix_cls(initialize_trader_risk_group)]: DON'T MODIFY
@dataclass
class InitializeTraderRiskGroupIx:
    program_id: PublicKey

    # account metas
    owner: AccountMeta
    trader_risk_group: AccountMeta
    market_product_group: AccountMeta
    risk_signer: AccountMeta
    trader_risk_state_acct: AccountMeta
    trader_fee_state_acct: AccountMeta
    risk_engine_program: AccountMeta
    system_program: AccountMeta
    remaining_accounts: Optional[List[AccountMeta]]

    def to_instruction(self):
        keys = []
        keys.append(self.owner)
        keys.append(self.trader_risk_group)
        keys.append(self.market_product_group)
        keys.append(self.risk_signer)
        keys.append(self.trader_risk_state_acct)
        keys.append(self.trader_fee_state_acct)
        keys.append(self.risk_engine_program)
        keys.append(self.system_program)
        if self.remaining_accounts is not None:
            keys.extend(self.remaining_accounts)

        buffer = BytesIO()
        buffer.write(InstructionTag.to_bytes(InstructionTag.INITIALIZE_TRADER_RISK_GROUP))

        return TransactionInstruction(
            keys=keys,
            program_id=self.program_id,
            data=buffer.getvalue(),
        )

# LOCK-END


# LOCK-BEGIN[ix_fn(initialize_trader_risk_group)]: DON'T MODIFY
def initialize_trader_risk_group(
    owner: Union[str, PublicKey, AccountMeta],
    trader_risk_group: Union[str, PublicKey, AccountMeta],
    market_product_group: Union[str, PublicKey, AccountMeta],
    risk_signer: Union[str, PublicKey, AccountMeta],
    trader_risk_state_acct: Union[str, PublicKey, AccountMeta],
    trader_fee_state_acct: Union[str, PublicKey, AccountMeta],
    risk_engine_program: Union[str, PublicKey, AccountMeta],
    system_program: Union[str, PublicKey, AccountMeta] = PublicKey("11111111111111111111111111111111"),
    remaining_accounts: Optional[List[AccountMeta]] = None,
    program_id: Optional[PublicKey] = None,
):
    if program_id is None:
        program_id = PublicKey("Dex1111111111111111111111111111111111111111")

    if isinstance(owner, (str, PublicKey)):
        owner = to_account_meta(
            owner,
            is_signer=True,
            is_writable=True,
        )
    if isinstance(trader_risk_group, (str, PublicKey)):
        trader_risk_group = to_account_meta(
            trader_risk_group,
            is_signer=False,
            is_writable=True,
        )
    if isinstance(market_product_group, (str, PublicKey)):
        market_product_group = to_account_meta(
            market_product_group,
            is_signer=False,
            is_writable=False,
        )
    if isinstance(risk_signer, (str, PublicKey)):
        risk_signer = to_account_meta(
            risk_signer,
            is_signer=False,
            is_writable=False,
        )
    if isinstance(trader_risk_state_acct, (str, PublicKey)):
        trader_risk_state_acct = to_account_meta(
            trader_risk_state_acct,
            is_signer=True,
            is_writable=True,
        )
    if isinstance(trader_fee_state_acct, (str, PublicKey)):
        trader_fee_state_acct = to_account_meta(
            trader_fee_state_acct,
            is_signer=False,
            is_writable=False,
        )
    if isinstance(risk_engine_program, (str, PublicKey)):
        risk_engine_program = to_account_meta(
            risk_engine_program,
            is_signer=False,
            is_writable=False,
        )
    if isinstance(system_program, (str, PublicKey)):
        system_program = to_account_meta(
            system_program,
            is_signer=False,
            is_writable=False,
        )

    return InitializeTraderRiskGroupIx(
        program_id=program_id,
        owner=owner,
        trader_risk_group=trader_risk_group,
        market_product_group=market_product_group,
        risk_signer=risk_signer,
        trader_risk_state_acct=trader_risk_state_acct,
        trader_fee_state_acct=trader_fee_state_acct,
        risk_engine_program=risk_engine_program,
        system_program=system_program,
        remaining_accounts=remaining_accounts,
    ).to_instruction()

# LOCK-END
