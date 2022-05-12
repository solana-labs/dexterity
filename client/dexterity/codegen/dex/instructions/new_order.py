# LOCK-BEGIN[imports]: DON'T MODIFY
from .instruction_tag import InstructionTag
from dataclasses import dataclass
from dexterity.codegen.dex.types import NewOrderParams
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


# LOCK-BEGIN[ix_cls(new_order)]: DON'T MODIFY
@dataclass
class NewOrderIx:
    program_id: PublicKey

    # account metas
    user: AccountMeta
    trader_risk_group: AccountMeta
    market_product_group: AccountMeta
    product: AccountMeta
    aaob_program: AccountMeta
    orderbook: AccountMeta
    market_signer: AccountMeta
    event_queue: AccountMeta
    bids: AccountMeta
    asks: AccountMeta
    system_program: AccountMeta
    fee_model_program: AccountMeta
    fee_model_configuration_acct: AccountMeta
    trader_fee_state_acct: AccountMeta
    fee_output_register: AccountMeta
    risk_engine_program: AccountMeta
    risk_model_configuration_acct: AccountMeta
    risk_output_register: AccountMeta
    trader_risk_state_acct: AccountMeta
    risk_and_fee_signer: AccountMeta
    remaining_accounts: Optional[List[AccountMeta]]

    # data fields
    params: NewOrderParams

    def to_instruction(self):
        keys = []
        keys.append(self.user)
        keys.append(self.trader_risk_group)
        keys.append(self.market_product_group)
        keys.append(self.product)
        keys.append(self.aaob_program)
        keys.append(self.orderbook)
        keys.append(self.market_signer)
        keys.append(self.event_queue)
        keys.append(self.bids)
        keys.append(self.asks)
        keys.append(self.system_program)
        keys.append(self.fee_model_program)
        keys.append(self.fee_model_configuration_acct)
        keys.append(self.trader_fee_state_acct)
        keys.append(self.fee_output_register)
        keys.append(self.risk_engine_program)
        keys.append(self.risk_model_configuration_acct)
        keys.append(self.risk_output_register)
        keys.append(self.trader_risk_state_acct)
        keys.append(self.risk_and_fee_signer)
        if self.remaining_accounts is not None:
            keys.extend(self.remaining_accounts)

        buffer = BytesIO()
        buffer.write(InstructionTag.to_bytes(InstructionTag.NEW_ORDER))
        buffer.write(BYTES_CATALOG.pack(NewOrderParams, self.params))

        return TransactionInstruction(
            keys=keys,
            program_id=self.program_id,
            data=buffer.getvalue(),
        )

# LOCK-END


# LOCK-BEGIN[ix_fn(new_order)]: DON'T MODIFY
def new_order(
    user: Union[str, PublicKey, AccountMeta],
    trader_risk_group: Union[str, PublicKey, AccountMeta],
    market_product_group: Union[str, PublicKey, AccountMeta],
    product: Union[str, PublicKey, AccountMeta],
    aaob_program: Union[str, PublicKey, AccountMeta],
    orderbook: Union[str, PublicKey, AccountMeta],
    market_signer: Union[str, PublicKey, AccountMeta],
    event_queue: Union[str, PublicKey, AccountMeta],
    bids: Union[str, PublicKey, AccountMeta],
    asks: Union[str, PublicKey, AccountMeta],
    fee_model_program: Union[str, PublicKey, AccountMeta],
    fee_model_configuration_acct: Union[str, PublicKey, AccountMeta],
    trader_fee_state_acct: Union[str, PublicKey, AccountMeta],
    fee_output_register: Union[str, PublicKey, AccountMeta],
    risk_engine_program: Union[str, PublicKey, AccountMeta],
    risk_model_configuration_acct: Union[str, PublicKey, AccountMeta],
    risk_output_register: Union[str, PublicKey, AccountMeta],
    trader_risk_state_acct: Union[str, PublicKey, AccountMeta],
    risk_and_fee_signer: Union[str, PublicKey, AccountMeta],
    params: NewOrderParams,
    system_program: Union[str, PublicKey, AccountMeta] = PublicKey("11111111111111111111111111111111"),
    remaining_accounts: Optional[List[AccountMeta]] = None,
    program_id: Optional[PublicKey] = None,
):
    if program_id is None:
        program_id = PublicKey("Dex1111111111111111111111111111111111111111")

    if isinstance(user, (str, PublicKey)):
        user = to_account_meta(
            user,
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
            is_writable=True,
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
    if isinstance(system_program, (str, PublicKey)):
        system_program = to_account_meta(
            system_program,
            is_signer=False,
            is_writable=False,
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
    if isinstance(trader_fee_state_acct, (str, PublicKey)):
        trader_fee_state_acct = to_account_meta(
            trader_fee_state_acct,
            is_signer=False,
            is_writable=True,
        )
    if isinstance(fee_output_register, (str, PublicKey)):
        fee_output_register = to_account_meta(
            fee_output_register,
            is_signer=False,
            is_writable=True,
        )
    if isinstance(risk_engine_program, (str, PublicKey)):
        risk_engine_program = to_account_meta(
            risk_engine_program,
            is_signer=False,
            is_writable=False,
        )
    if isinstance(risk_model_configuration_acct, (str, PublicKey)):
        risk_model_configuration_acct = to_account_meta(
            risk_model_configuration_acct,
            is_signer=False,
            is_writable=False,
        )
    if isinstance(risk_output_register, (str, PublicKey)):
        risk_output_register = to_account_meta(
            risk_output_register,
            is_signer=False,
            is_writable=True,
        )
    if isinstance(trader_risk_state_acct, (str, PublicKey)):
        trader_risk_state_acct = to_account_meta(
            trader_risk_state_acct,
            is_signer=False,
            is_writable=True,
        )
    if isinstance(risk_and_fee_signer, (str, PublicKey)):
        risk_and_fee_signer = to_account_meta(
            risk_and_fee_signer,
            is_signer=False,
            is_writable=False,
        )

    return NewOrderIx(
        program_id=program_id,
        user=user,
        trader_risk_group=trader_risk_group,
        market_product_group=market_product_group,
        product=product,
        aaob_program=aaob_program,
        orderbook=orderbook,
        market_signer=market_signer,
        event_queue=event_queue,
        bids=bids,
        asks=asks,
        system_program=system_program,
        fee_model_program=fee_model_program,
        fee_model_configuration_acct=fee_model_configuration_acct,
        trader_fee_state_acct=trader_fee_state_acct,
        fee_output_register=fee_output_register,
        risk_engine_program=risk_engine_program,
        risk_model_configuration_acct=risk_model_configuration_acct,
        risk_output_register=risk_output_register,
        trader_risk_state_acct=trader_risk_state_acct,
        risk_and_fee_signer=risk_and_fee_signer,
        remaining_accounts=remaining_accounts,
        params=params,
    ).to_instruction()

# LOCK-END
