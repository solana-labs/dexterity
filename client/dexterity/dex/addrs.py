import hashlib
from typing import Optional, Tuple

from podite import U8
from solana.publickey import PublicKey

from dexterity import program_ids as pids
from dexterity.utils.aob.state import MarketState

DEFAULT_ORDERBOOK_SIZE = MarketState.calc_size()
DEFAULT_EVENT_QUEUE_SIZE = 100_000
DEFAULT_ASKS_SIZE = 100_000
DEFAULT_BIDS_SIZE = 100_000
DEFAULT_TICK_SIZE = 0.1
DEFAULT_DECIMALS = 6
DEFAULT_MATCH_LIMIT = 50
DEFAULT_OFFSET = 0
OUT_REGISTER_RISK_SIZE = 440  # fixed size taken from rust

PROGRAMS_SEED = f"{pids.AOB_PROGRAM_ID}:{pids.DEX_PROGRAM_ID}"
MARKET_PRODUCT_GROUP_SEED_LAYOUT = f"prod_grp:{PROGRAMS_SEED}:{{seed}}"
ORDERBOOK_SEED_LAYOUT = f"prod:ob:{PROGRAMS_SEED}:{{group}}:{{key}}"
EVENT_QUEUE_SEED_LAYOUT = f"prod:eq:{PROGRAMS_SEED}:{{group}}:{{key}}"
BIDS_SEED_LAYOUT = f"prod:bid:{PROGRAMS_SEED}:{{group}}:{{key}}"
ASKS_SEED_LAYOUT = f"prod:ask:{PROGRAMS_SEED}:{{group}}:{{key}}"
TRADER_RISK_GROUP_SEED_LAYOUT = f"trdr_grp:{PROGRAMS_SEED}:{{market_product_group}}"
MINT_SEED_LAYOUT = f"mint:{PROGRAMS_SEED}:{{seed}}"
OUT_REGISTER_RISK_LAYOUT = f"out_register_risk:{PROGRAMS_SEED}:{{group}}"
FEE_REGISTER_LAYOUT = f"fee:{PROGRAMS_SEED}:{{group}}"
RISK_CONFIG_LAYOUT = f"risk_config:{PROGRAMS_SEED}:{{group}}"
IN_REGISTER_RISK_LAYOUT = f"in_register_risk:{PROGRAMS_SEED}:{{group}}"


def crush(string):
    return hashlib.md5(string.encode("UTF-8")).hexdigest()


def get_market_product_group_addr(
        authority: PublicKey,
        seed: str,
):
    return PublicKey.create_with_seed(
        from_public_key=authority,
        seed=crush(MARKET_PRODUCT_GROUP_SEED_LAYOUT.format(seed=seed)),
        program_id=pids.DEX_PROGRAM_ID,
    )


def get_risk_signer(mpg: PublicKey):
    return PublicKey.find_program_address([bytes(mpg)], pids.DEX_PROGRAM_ID)[0]


def get_risk_register_addr(
        authority: PublicKey,
        group: PublicKey,
        program_id: PublicKey,
        layout_str: str,
):
    return PublicKey.create_with_seed(
        from_public_key=authority,
        seed=crush(layout_str.format(group=group)),
        program_id=program_id,
    )


def get_fee_register_addr(
        authority: PublicKey,
        group: PublicKey,
        program_id: PublicKey,
        layout_str: str,
):
    return PublicKey.create_with_seed(
        from_public_key=authority,
        seed=crush(layout_str.format(group=group)),
        program_id=program_id,
    )


def get_trader_fee_state_acct(
        trader_risk_group: PublicKey,
        market_product_group: PublicKey,
        program_id: PublicKey = pids.CONSTANT_FEES_MODEL_PROGRAM_ID,
) -> PublicKey:
    key, _ = PublicKey.find_program_address(
        seeds=[
            b"trader_fee_acct",
            bytes(trader_risk_group),
            bytes(market_product_group),
        ],
        program_id=program_id,
    )
    return key


def get_market_product_group_vault(mpg_key: PublicKey):
    return PublicKey.find_program_address(
        [b"market_vault", bytes(mpg_key)], pids.DEX_PROGRAM_ID
    )[0]


def get_market_signer(product_key: PublicKey, dex_pid: PublicKey = pids.DEX_PROGRAM_ID) -> PublicKey:
    return PublicKey.find_program_address([bytes(product_key)], dex_pid)[0]


def get_risk_model_configuration_addr(
        market_product_group_key: PublicKey,
        program_id: Optional[PublicKey] = None,
) -> PublicKey:
    if program_id is None:
        program_id = pids.CONSTANT_FEES_MODEL_PROGRAM_ID
    key, _ = PublicKey.find_program_address(
        seeds=[
            b"risk_model_config_acct",
            bytes(market_product_group_key),
        ],
        program_id=program_id,
    )
    return key


def get_fee_model_configuration_addr(
        market_product_group_key: PublicKey,
        program_id: Optional[PublicKey] = None,
) -> PublicKey:
    if program_id is None:
        program_id = pids.CONSTANT_FEES_MODEL_PROGRAM_ID
    key, _ = PublicKey.find_program_address(
        seeds=[
            b"fee_model_config_acct",
            bytes(market_product_group_key),
        ],
        program_id=program_id,
    )
    return key


def get_trader_risk_group_addr(
        trader: PublicKey,
        market_product_group: PublicKey,
):
    return PublicKey.create_with_seed(
        from_public_key=trader,
        seed=crush(
            TRADER_RISK_GROUP_SEED_LAYOUT.format(
                market_product_group=market_product_group
            )
        ),
        program_id=pids.DEX_PROGRAM_ID,
    )
