from dexterity.program_ids import ORACLE_PROGRAM_ID
from dexterity.dummy_oracle.state import OraclePrice, Clock

from solana.keypair import Keypair
from solana.publickey import PublicKey
from solana.transaction import Transaction
from solana.system_program import create_account, CreateAccountParams

import dexterity.dummy_oracle.instructions as ixs

from dexterity.utils.solana import (
    actionify,
    Context,
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


@actionify
def initialize_oracle(
    authority: PublicKey,
    oracle: PublicKey,
    price: int,
    decimals: int = 0,
):
    space = OraclePrice.calc_size()
    rent = _calc_rent(space)
    return Transaction().add(
        create_account(
            CreateAccountParams(
                from_pubkey=authority,
                new_account_pubkey=oracle,
                lamports=rent,
                space=space,
                program_id=ORACLE_PROGRAM_ID,
            )
        ),
        ixs.initialize_oracle_ix(
            oracle_price=oracle,
            update_authority=authority,
            price=price,
            decimals=decimals,
        ),
    )


@actionify
def initialize_clock(
    authority: PublicKey,
    clock: PublicKey,
):
    space = Clock.calc_size()
    rent = _calc_rent(space)
    return Transaction().add(
        create_account(
            CreateAccountParams(
                from_pubkey=authority,
                new_account_pubkey=clock,
                lamports=rent,
                space=space,
                program_id=ORACLE_PROGRAM_ID,
            )
        ),
        ixs.initialize_clock_ix(
            clock=clock,
            update_authority=authority,
        ),
    )


@actionify
def update_clock(
    clock: PublicKey,
    slot: int = 0,
    epoch_start_timestamp: int = 0,
    epoch: int = 0,
    leader_schedule_epoch: int = 0,
    unix_timestamp: int = 0,
):
    return Transaction().add(
        ixs.update_clock_ix(
            clock=clock,
            slot=slot,
            epoch_start_timestamp=epoch_start_timestamp,
            epoch=epoch,
            leader_schedule_epoch=leader_schedule_epoch,
            unix_timestamp=unix_timestamp,
        )
    )


@actionify
def update_price(
    oracle: PublicKey,
    authority: Keypair,
    price: int,
    decimals: int = 0,
):
    return Transaction().add(
        ixs.update_price_ix(
            oracle_price=oracle,
            update_authority=authority,
            price=price,
            decimals=decimals,
        )
    )
