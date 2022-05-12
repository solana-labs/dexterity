import os

import solana.system_program
import solana.sysvar
import spl.token.constants
from solana.publickey import PublicKey

# from solmate import set_pid_by_protocol_name

from dexterity.scripts import extract_program_ids

try:
    programs = extract_program_ids.get_program_to_id()
except:
    programs = {}

RENT_PROGRAM_ID = solana.sysvar.SYSVAR_RENT_PUBKEY
CLOCK_PROGRAM_ID = solana.sysvar.SYSVAR_CLOCK_PUBKEY
SYSTEM_PROGRAM_ID = solana.system_program.SYS_PROGRAM_ID
SPL_TOKEN_PROGRAM_ID = spl.token.constants.TOKEN_PROGRAM_ID

DEX_PROGRAM_ID = PublicKey(os.environ.get("DEX", programs.get("dex", "")))
INSTRUMENTS_PROGRAM_ID = PublicKey(
    os.environ.get("INSTRUMENTS", programs.get("instruments", ""))
)
ORACLE_PROGRAM_ID = PublicKey(
    os.environ.get("DUMMY_ORACLE", programs.get("dummy_oracle", ""))
)
NOOP_RISK_ENGINE_PROGRAM_ID = PublicKey(
    os.environ.get("NOOP_RISK_ENGINE", programs.get("noop_risk_engine", ""))
)
ALPHA_RISK_ENGINE_PROGRAM_ID = PublicKey(
    os.environ.get("ALPHA_RISK_ENGINE", programs.get("alpha_risk_engine", ""))
)
RISK_ENGINE_PROGRAM_ID = PublicKey(
    os.environ.get("RISK_ENGINE", ALPHA_RISK_ENGINE_PROGRAM_ID)
)
AOB_PROGRAM_ID = PublicKey(
    os.environ.get("AGNOSTIC_ORDERBOOK", programs.get("agnostic_orderbook", ""))
)
CONSTANT_FEES_MODEL_PROGRAM_ID = PublicKey(
    os.environ.get("CONSTANT_FEES", programs.get("constant_fees", ""))
)

# todo: make this ~better~ -> work...
# set_pid_by_protocol_name("risk", ALPHA_RISK_ENGINE_PROGRAM_ID)
# set_pid_by_protocol_name("dex", DEX_PROGRAM_ID)
# set_pid_by_protocol_name("instruments", INSTRUMENTS_PROGRAM_ID)
