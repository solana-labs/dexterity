from typing import Optional

from solana.publickey import PublicKey
from solana.transaction import Transaction

import dexterity.constant_fees.instructions as ixs
import dexterity.program_ids as pids
from dexterity.dex.addrs import (
    get_trader_fee_state_acct,
    get_trader_risk_group_addr,
    get_fee_model_configuration_addr,
)
from dexterity.utils.solana import (
    actionify,
)


@actionify
def update_fees(
    payer: PublicKey,
    market_product_group: PublicKey,
    maker_fee_bps: int,
    taker_fee_bps: int,
    fee_model_config_acct: Optional[PublicKey] = None,
    program_id: PublicKey = pids.CONSTANT_FEES_MODEL_PROGRAM_ID,
):
    if fee_model_config_acct is None:
        fee_model_config_acct = get_fee_model_configuration_addr(
            market_product_group, program_id
        )

    return Transaction(fee_payer=payer).add(
        ixs.update_fees_ix(
            payer=payer,
            fee_model_config_acct=fee_model_config_acct,
            market_product_group=market_product_group,
            system_program=pids.SYSTEM_PROGRAM_ID,
            maker_fee_bps=maker_fee_bps,
            taker_fee_bps=taker_fee_bps,
            program_id=program_id,
        )
    )


@actionify
def initialize_trader_fee_acct(
    payer: PublicKey,
    market_product_group: PublicKey,
    program_id: PublicKey = pids.CONSTANT_FEES_MODEL_PROGRAM_ID,
    trader_risk_group: Optional[PublicKey] = None,
    system_program: Optional[PublicKey] = pids.SYSTEM_PROGRAM_ID,
    fee_model_config_acct: Optional[PublicKey] = None,
):
    if trader_risk_group is None:
        trader_risk_group = get_trader_risk_group_addr(
            payer,
            market_product_group,
        )

    if fee_model_config_acct is None:
        fee_model_config_acct = get_fee_model_configuration_addr(
            market_product_group,
            program_id,
        )

    trader_fee_acct = get_trader_fee_state_acct(
        payer,
        market_product_group,
        program_id,
    )

    return Transaction(fee_payer=payer).add(
        ixs.initialize_trader_acct_ix(
            payer=payer,
            fee_model_config_acct=fee_model_config_acct,
            trader_fee_acct=trader_fee_acct,
            market_product_group=market_product_group,
            trader_risk_group=trader_risk_group,
            system_program=system_program,
            program_id=program_id,
        )
    )
