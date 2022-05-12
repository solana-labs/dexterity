from ctypes import Union
from solana.transaction import AccountMeta, TransactionInstruction
from typing import Optional

from solana.publickey import PublicKey
from solana.transaction import Transaction
import dexterity.program_ids as pids
from podite import pod, U64
from dexterity.utils.solana import (
    actionify
)
from dexterity.dex.actions import RISK_CONFIG_LAYOUT 
from dexterity.dex.addrs import get_risk_register_addr

@pod
class Params:
    instr: U64

def _post_init_risk_config(resp):
    if resp is None:
        return None, None
    addr = resp.instructions[0]["accounts"][1]
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


@actionify(post_process=_post_init_risk_config)
def initialize_risk_config_acct(
    admin: PublicKey,
    market_product_group: PublicKey,
    program_id: Optional[PublicKey] = pids.ALPHA_RISK_ENGINE_PROGRAM_ID,
    risk_model_config_acct: Optional[PublicKey] = None,
    layout_str: Optional[str] = RISK_CONFIG_LAYOUT,
):
    if program_id == pids.ALPHA_RISK_ENGINE_PROGRAM_ID: 
        return None

    if risk_model_config_acct is None:
        risk_model_config_acct = get_risk_register_addr(admin, market_product_group, program_id, layout_str)

    keys = [
        AccountMeta(pubkey=market_product_group, is_signer=False, is_writable=False),
        AccountMeta(pubkey=admin, is_signer=True, is_writable=False),
        AccountMeta(pubkey=risk_model_config_acct, is_signer=False, is_writable=True),        
    ]

    params = Params(
        instr=4,
    )

    return Transaction(fee_payer=admin).add(
        TransactionInstruction(
                keys=keys,
                program_id=program_id,
                data=Params.to_bytes(params),
            ),
        )

