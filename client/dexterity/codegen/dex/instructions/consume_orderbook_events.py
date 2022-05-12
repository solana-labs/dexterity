# LOCK-BEGIN[imports]: DON'T MODIFY
from .instruction_tag import InstructionTag
from dataclasses import dataclass
from dexterity.codegen.dex.types import ConsumeOrderbookEventsParams
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


# LOCK-BEGIN[ix_cls(consume_orderbook_events)]: DON'T MODIFY
@dataclass
class ConsumeOrderbookEventsIx:
    program_id: PublicKey

    # account metas
    aaob_program: AccountMeta
    market_product_group: AccountMeta
    product: AccountMeta
    market_signer: AccountMeta
    orderbook: AccountMeta
    event_queue: AccountMeta
    reward_target: AccountMeta
    fee_model_program: AccountMeta
    fee_model_configuration_acct: AccountMeta
    fee_output_register: AccountMeta
    risk_and_fee_signer: AccountMeta
    remaining_accounts: Optional[List[AccountMeta]]

    # data fields
    params: ConsumeOrderbookEventsParams

    def to_instruction(self):
        keys = []
        keys.append(self.aaob_program)
        keys.append(self.market_product_group)
        keys.append(self.product)
        keys.append(self.market_signer)
        keys.append(self.orderbook)
        keys.append(self.event_queue)
        keys.append(self.reward_target)
        keys.append(self.fee_model_program)
        keys.append(self.fee_model_configuration_acct)
        keys.append(self.fee_output_register)
        keys.append(self.risk_and_fee_signer)
        if self.remaining_accounts is not None:
            keys.extend(self.remaining_accounts)

        buffer = BytesIO()
        buffer.write(InstructionTag.to_bytes(InstructionTag.CONSUME_ORDERBOOK_EVENTS))
        buffer.write(BYTES_CATALOG.pack(ConsumeOrderbookEventsParams, self.params))

        return TransactionInstruction(
            keys=keys,
            program_id=self.program_id,
            data=buffer.getvalue(),
        )

# LOCK-END


# LOCK-BEGIN[ix_fn(consume_orderbook_events)]: DON'T MODIFY
def consume_orderbook_events(
    aaob_program: Union[str, PublicKey, AccountMeta],
    market_product_group: Union[str, PublicKey, AccountMeta],
    product: Union[str, PublicKey, AccountMeta],
    market_signer: Union[str, PublicKey, AccountMeta],
    orderbook: Union[str, PublicKey, AccountMeta],
    event_queue: Union[str, PublicKey, AccountMeta],
    reward_target: Union[str, PublicKey, AccountMeta],
    fee_model_program: Union[str, PublicKey, AccountMeta],
    fee_model_configuration_acct: Union[str, PublicKey, AccountMeta],
    fee_output_register: Union[str, PublicKey, AccountMeta],
    risk_and_fee_signer: Union[str, PublicKey, AccountMeta],
    params: ConsumeOrderbookEventsParams,
    remaining_accounts: Optional[List[AccountMeta]] = None,
    program_id: Optional[PublicKey] = None,
):
    if program_id is None:
        program_id = PublicKey("Dex1111111111111111111111111111111111111111")

    if isinstance(aaob_program, (str, PublicKey)):
        aaob_program = to_account_meta(
            aaob_program,
            is_signer=False,
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
    if isinstance(market_signer, (str, PublicKey)):
        market_signer = to_account_meta(
            market_signer,
            is_signer=False,
            is_writable=False,
        )
    if isinstance(orderbook, (str, PublicKey)):
        orderbook = to_account_meta(
            orderbook,
            is_signer=False,
            is_writable=True,
        )
    if isinstance(event_queue, (str, PublicKey)):
        event_queue = to_account_meta(
            event_queue,
            is_signer=False,
            is_writable=True,
        )
    if isinstance(reward_target, (str, PublicKey)):
        reward_target = to_account_meta(
            reward_target,
            is_signer=True,
            is_writable=True,
        )
    if isinstance(fee_model_program, (str, PublicKey)):
        fee_model_program = to_account_meta(
            fee_model_program,
            is_signer=False,
            is_writable=False,
        )
    if isinstance(fee_model_configuration_acct, (str, PublicKey)):
        fee_model_configuration_acct = to_account_meta(
            fee_model_configuration_acct,
            is_signer=False,
            is_writable=False,
        )
    if isinstance(fee_output_register, (str, PublicKey)):
        fee_output_register = to_account_meta(
            fee_output_register,
            is_signer=False,
            is_writable=True,
        )
    if isinstance(risk_and_fee_signer, (str, PublicKey)):
        risk_and_fee_signer = to_account_meta(
            risk_and_fee_signer,
            is_signer=False,
            is_writable=False,
        )

    return ConsumeOrderbookEventsIx(
        program_id=program_id,
        aaob_program=aaob_program,
        market_product_group=market_product_group,
        product=product,
        market_signer=market_signer,
        orderbook=orderbook,
        event_queue=event_queue,
        reward_target=reward_target,
        fee_model_program=fee_model_program,
        fee_model_configuration_acct=fee_model_configuration_acct,
        fee_output_register=fee_output_register,
        risk_and_fee_signer=risk_and_fee_signer,
        remaining_accounts=remaining_accounts,
        params=params,
    ).to_instruction()

# LOCK-END
