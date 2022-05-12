from typing import List
from dexterity.program_ids import DEX_PROGRAM_ID, INSTRUMENTS_PROGRAM_ID

from solana.keypair import Keypair
from solana.publickey import PublicKey
from solana.transaction import Transaction

import dexterity.instruments.instructions as ixs

from dexterity.codegen.dex.types import Fractional
from dexterity.codegen.instruments import instructions as iixs
from dexterity.codegen.instruments import types as its
from dexterity.utils.solana import (
    actionify,
    Context,
)
from dexterity.codegen.instruments.types import (
    InstrumentType,
    OracleType,
)


def _calc_rent(space, client=None):
    if client is None:
        client = Context.get_global_client()
    return client.get_minimum_balance_for_rent_exemption(space)["result"]


def extract_acct_addr(resp, idx=0):
    addr = resp.instructions[0]["accounts"][idx]
    exists = False
    if resp.error:
        error_ix, error_info = resp.error["InstructionError"]
        if error_ix == 0 and error_info["Custom"] == 0:
            exists = True
    else:
        exists = True

    if exists:
        return addr, resp
    else:
        return None, resp


@actionify(post_process=lambda x: extract_acct_addr(x, idx=0))
def initialize_derivative(
    price_oracle: PublicKey,
    market_product_group: PublicKey,
    payer: PublicKey,
    instrument_type: InstrumentType,
    strike: float,
    full_funding_period: int,
    minimum_funding_period: int,
    initialization_time: int,
    oracle_type: OracleType,
    close_authority: PublicKey,
    clock: PublicKey = None,
    **kwargs,
):
    params = its.InitializeDerivativeParams(
        instrument_type=instrument_type,
        strike=Fractional.to_decimal(strike),
        full_funding_period=full_funding_period,
        minimum_funding_period=minimum_funding_period,
        initialization_time=initialization_time,
        close_authority=close_authority,
        oracle_type=oracle_type,
    )
    return Transaction().add(
        iixs.initialize_derivative(
            derivative_metadata=ixs.get_derivative_key(
                price_oracle=price_oracle,
                market_product_group=market_product_group,
                instrument_type=instrument_type,
                strike=strike,
                full_funding_period=full_funding_period,
                minimum_funding_period=minimum_funding_period,
                initialization_time=initialization_time,
            )[0],
            price_oracle=price_oracle,
            market_product_group=market_product_group,
            payer=payer,
            clock=clock,
            params=params,
        ),
    )


@actionify(post_process=lambda x: extract_acct_addr(x, idx=0))
def initialize_fixed_income(
    face_value: int,
    market_product_group: PublicKey,
    payer: Keypair,
    coupon_dates: List[int],
    coupon_rate: int,
    maturity_date: int,
    initialization_time: int,
    close_authority: PublicKey,
):
    return iixs.initialize_fixed_income(
        fixed_income_metadata=ixs.get_fixed_income_key(
            market_product_group=market_product_group,
            initialization_time=initialization_time,
            coupon_rate=coupon_rate,
            maturity_date=maturity_date
        )[0],
        market_product_group=market_product_group,
        payer=payer.public_key,
        params=its.InitializeFixedIncomeParams(
            face_value=face_value,
            coupon_rate=coupon_rate,
            initialization_time=initialization_time,
            coupon_dates=coupon_dates,
            maturity_date=maturity_date,
            close_authority=close_authority,
        )
    )


_tick = 0
@actionify
def settle_derivative(
    market_product_group: PublicKey,
    derivative_metadata: PublicKey,
    payer: PublicKey,
    price_oracle: PublicKey,
    clock: PublicKey = None,
):
    _tick += 1
    return Transaction(fee_payer=payer).add(
        iixs.settle_derivative(
            market_product_group,
            derivative_metadata=derivative_metadata,
            price_oracle=price_oracle,
            clock=clock,
            dex_program=DEX_PROGRAM_ID,
            tick=_tick,
        ),
    )


@actionify(post_process=lambda x: extract_acct_addr(x, idx=1))
def settle_fixed_income_ix(
    market_product_group: PublicKey,
    fixed_income_metadata: PublicKey,
):
    return iixs.settle_fixed_income(
        market_product_group=market_product_group,
        fixed_income_metadata=fixed_income_metadata,
        dex_program=DEX_PROGRAM_ID,
    )
