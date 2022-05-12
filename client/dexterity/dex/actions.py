from typing import Any, Optional, Tuple, List

import solana.system_program as sp
import solana.sysvar
from podite import pod, U64, U128, Option, I8, Static
from solana.keypair import Keypair
from solana.publickey import PublicKey
from solana.rpc import types
from solana.rpc.commitment import Confirmed
from solana.transaction import Transaction, AccountMeta
from spl.token.constants import MINT_LEN
from spl.token.instructions import (
    TOKEN_PROGRAM_ID,
    create_associated_token_account,
    get_associated_token_address,
    mint_to_checked,
    MintToCheckedParams,
)

import dexterity.codegen.dex.instructions as ixs
import dexterity.codegen.dex.types as dex_types
import dexterity.dex.addrs as dex_addrs
import dexterity.program_ids as pids
from dexterity.codegen.dex.types import MarketProductGroup, TraderRiskGroup
from dexterity.codegen.dex.types.fractional import Fractional
from dexterity.dex.addrs import crush
from dexterity.program_ids import DEX_PROGRAM_ID
from dexterity.utils import create_market_aob_ix
from dexterity.utils.aob.state.base import SelfTradeBehavior, Side
from dexterity.utils.aob.state.market_state import MarketState
from dexterity.utils.aob.state.slab import Slab
from dexterity.utils.solana import (
    actionify,
    Context,
    fetch_account_details,
    send_transaction,
    sighash_int,
)

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


def _to_rust_string_for_construct(s: str):
    return {"length": len(s), "chars": s}


def _calc_rent(space, client=None):
    if client is None:
        client = Context.get_global_client()
    return client.get_minimum_balance_for_rent_exemption(space)["result"]


def _post_create_market_product_group(resp):
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


@actionify(post_process=_post_create_market_product_group)
def create_market_product_group(
        authority: PublicKey,
        seed: str,
        vault_mint: PublicKey,
        fee_collector: PublicKey,
        fee_model_configuration_acct: PublicKey,
        name: Optional[str] = None,
        risk_engine_program: Optional[PublicKey] = pids.ALPHA_RISK_ENGINE_PROGRAM_ID,
        fee_model_program: Optional[PublicKey] = pids.CONSTANT_FEES_MODEL_PROGRAM_ID,
):
    market_product_group = dex_addrs.get_market_product_group_addr(authority, seed)
    space = Static[MarketProductGroup].calc_size() + 8
    rent = _calc_rent(space)

    return Transaction(fee_payer=authority).add(
        sp.create_account_with_seed(
            sp.CreateAccountWithSeedParams(
                from_pubkey=authority,
                new_account_pubkey=market_product_group,
                base_pubkey=authority,
                seed=_to_rust_string_for_construct(crush(MARKET_PRODUCT_GROUP_SEED_LAYOUT.format(seed=seed))),
                lamports=rent,
                space=space,
                program_id=pids.DEX_PROGRAM_ID,
            )
        )
    )


def _post_init_market_product_group(resp):
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


@actionify(post_process=_post_init_market_product_group)
def init_market_product_group(
        authority: PublicKey,
        seed: str,
        vault_mint: PublicKey,
        vault: PublicKey,
        fee_collector: PublicKey,
        fee_model_configuration_acct: PublicKey,
        risk_model_configuration_acct: PublicKey,
        name: Optional[str] = None,
        risk_engine_program: Optional[PublicKey] = pids.ALPHA_RISK_ENGINE_PROGRAM_ID,
        fee_model_program: Optional[PublicKey] = pids.CONSTANT_FEES_MODEL_PROGRAM_ID,
        is_risk_anchor: bool = True,
        params: dex_types.InitializeMarketProductGroupParams = None
):
    market_product_group = dex_addrs.get_market_product_group_addr(authority, seed)

    fee_output_register = dex_addrs.get_fee_register_addr(
        authority, market_product_group, fee_model_program, FEE_REGISTER_LAYOUT
    )
    risk_output_register = dex_addrs.get_risk_register_addr(
        authority, market_product_group, risk_engine_program, OUT_REGISTER_RISK_LAYOUT
    )

    if name is None:
        name = seed
    name = bytes(name, encoding="utf-8")
    if len(name) > 16:
        name = name[:16]
    elif len(name) < 16:
        name = name + b" " * (16 - len(name))

    if params is None:
        if is_risk_anchor:
            disc_len = U64(8)
            health = U64(sighash_int("validate_account_health"))
            liquidation = U64(sighash_int("validate_account_liquidation"))
            create_risk_state = U64(sighash_int("create_risk_state_account"))
        else:
            disc_len = U64(1)
            health = U64(0)
            liquidation = U64(1)
            create_risk_state = U64(2)
        params = dex_types.InitializeMarketProductGroupParams(
            name=name,
            validate_account_discriminant_len=disc_len,
            find_fees_discriminant_len=1,
            validate_account_health_discriminant=U64.to_bytes(health),
            validate_account_liquidation_discriminant=U64.to_bytes(liquidation),
            create_risk_state_account_discriminant=U64.to_bytes(create_risk_state),
            find_fees_discriminant=U64.to_bytes(0),
            max_maker_fee_bps=1000,
            min_maker_fee_bps=-10,
            max_taker_fee_bps=1000,
            min_taker_fee_bps=0,
        )

    return Transaction(fee_payer=authority).add(
        ixs.initialize_market_product_group(
            authority=authority,
            market_product_group=market_product_group,
            vault_mint=vault_mint,
            vault=vault,
            fee_collector=fee_collector,
            fee_model_program=fee_model_program,
            fee_model_configuration_acct=fee_model_configuration_acct,
            risk_engine_program=risk_engine_program,
            fee_output_register=fee_output_register,
            risk_model_configuration_acct=risk_model_configuration_acct,
            risk_output_register=risk_output_register,
            sysvar_rent=solana.sysvar.SYSVAR_RENT_PUBKEY,
            params=params,
        ),
    )


def _post_create_risk_register(resp):
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


def _post_create_risk_config(resp):
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


@actionify
def create_fee_register(
        authority: PublicKey,
        group: PublicKey,
        register_size: Any,
        program_id: Optional[PublicKey] = pids.CONSTANT_FEES_MODEL_PROGRAM_ID,
        layout_str: Optional[str] = FEE_REGISTER_LAYOUT,
):
    register_info = dex_addrs.get_fee_register_addr(
        authority, group, program_id, layout_str
    )

    return Transaction(fee_payer=authority).add(
        system_program.create_account_with_seed(
            system_program.CreateAccountWithSeedParams(
                from_pubkey=authority,
                new_account_pubkey=register_info,
                base_pubkey=authority,
                seed=_to_rust_string_for_construct(
                    dex_addrs.crush(layout_str.format(group=group))
                ),
                space=register_size,
                lamports=_calc_rent(register_size),
                program_id=program_id,
            )
        ),
    )


from solana import system_program


@actionify(post_process=_post_create_risk_config)
def create_risk_config_acct(
        authority: PublicKey,
        group: PublicKey,
        program_id: Optional[PublicKey] = pids.ALPHA_RISK_ENGINE_PROGRAM_ID,
        layout_str: Optional[str] = RISK_CONFIG_LAYOUT,
):
    risk_config_acct = dex_addrs.get_risk_register_addr(
        authority, group, program_id, layout_str
    )

    if program_id == pids.ALPHA_RISK_ENGINE_PROGRAM_ID:
        register_size = 0
    else:
        print("Only alpha risk engine initialization currently implemented")
        raise Exception("Only alpha risk engine initialization currently implemented")

    return Transaction(fee_payer=authority).add(
        system_program.create_account_with_seed(
            system_program.CreateAccountWithSeedParams(
                from_pubkey=authority,
                new_account_pubkey=risk_config_acct,
                base_pubkey=authority,
                seed=_to_rust_string_for_construct(
                    dex_addrs.crush(layout_str.format(group=group))
                ),
                space=register_size,
                lamports=_calc_rent(register_size),
                program_id=program_id,
            )
        )
    )


@actionify(post_process=_post_create_risk_register)
def create_risk_register(
        authority: PublicKey,
        group: PublicKey,
        register_size: Any,
        program_id: Optional[PublicKey] = pids.ALPHA_RISK_ENGINE_PROGRAM_ID,
        layout_str: Optional[str] = OUT_REGISTER_RISK_LAYOUT,
):
    register_info = dex_addrs.get_risk_register_addr(
        authority, group, program_id, layout_str
    )

    return Transaction(fee_payer=authority).add(
        system_program.create_account_with_seed(
            system_program.CreateAccountWithSeedParams(
                from_pubkey=authority,
                new_account_pubkey=register_info,
                base_pubkey=authority,
                seed=_to_rust_string_for_construct(
                    dex_addrs.crush(layout_str.format(group=group))
                ),
                space=register_size,
                lamports=_calc_rent(register_size),
                program_id=program_id,
            ),
        ),
    )


def create_risk_model_configuration_acct(
        authority: Keypair,
        program_id: Optional[PublicKey] = pids.ALPHA_RISK_ENGINE_PROGRAM_ID,
):
    risk_model_key = Keypair()
    if program_id == pids.NOOP_RISK_ENGINE_PROGRAM_ID:
        space = 0
    elif program_id == pids.ALPHA_RISK_ENGINE_PROGRAM_ID:
        space = 0
    else:
        print("Unexpected program ID")
        raise Exception

    rent = _calc_rent(space)
    txn = Transaction().add(
        sp.create_account(
            sp.CreateAccountParams(
                from_pubkey=authority.public_key,
                new_account_pubkey=risk_model_key.public_key,
                lamports=rent,
                space=space,
                program_id=program_id,
            )
        )
    )

    send_transaction(
        txn,
        authority,
        risk_model_key,
        opts=types.TxOpts(
            skip_preflight=False,
            skip_confirmation=False,
            preflight_commitment=Confirmed,
        ),
    )
    return risk_model_key.public_key


def get_orderbook_addrs(
        authority: PublicKey,
        group: PublicKey,
        key: PublicKey,
):
    return (
        PublicKey.create_with_seed(
            from_public_key=authority,
            seed=crush(ORDERBOOK_SEED_LAYOUT.format(group=group, key=key)),
            program_id=pids.AOB_PROGRAM_ID,
        ),
        PublicKey.create_with_seed(
            from_public_key=authority,
            seed=crush(EVENT_QUEUE_SEED_LAYOUT.format(group=group, key=key)),
            program_id=pids.AOB_PROGRAM_ID,
        ),
        PublicKey.create_with_seed(
            from_public_key=authority,
            seed=crush(BIDS_SEED_LAYOUT.format(group=group, key=key)),
            program_id=pids.AOB_PROGRAM_ID,
        ),
        PublicKey.create_with_seed(
            from_public_key=authority,
            seed=crush(ASKS_SEED_LAYOUT.format(group=group, key=key)),
            program_id=pids.AOB_PROGRAM_ID,
        ),
    )


def create_aob_orderbook_helper(
        authority: PublicKey,
        market_product_group: PublicKey,
        product_key: PublicKey,
        orderbook_authority: PublicKey,
        callback_info_len: int = 32,
        callback_id_len: int = 32,
        min_base_order_size: int = 1,
        orderbook_size: int = DEFAULT_ORDERBOOK_SIZE,
        event_queue_size: int = DEFAULT_EVENT_QUEUE_SIZE,
        asks_size: int = DEFAULT_ASKS_SIZE,
        bids_size: int = DEFAULT_BIDS_SIZE,
):
    orderbook, event_queue, bids, asks = get_orderbook_addrs(
        authority, market_product_group, product_key
    )
    return Transaction(fee_payer=authority).add(
        sp.create_account_with_seed(
            sp.CreateAccountWithSeedParams(
                from_pubkey=authority,
                new_account_pubkey=orderbook,
                base_pubkey=authority,
                seed=_to_rust_string_for_construct(crush(
                    ORDERBOOK_SEED_LAYOUT.format(
                        group=market_product_group, key=product_key
                    )
                )),
                lamports=_calc_rent(orderbook_size),
                space=orderbook_size,
                program_id=pids.AOB_PROGRAM_ID,
            )
        ),
        sp.create_account_with_seed(
            sp.CreateAccountWithSeedParams(
                from_pubkey=authority,
                new_account_pubkey=event_queue,
                base_pubkey=authority,
                seed=_to_rust_string_for_construct(crush(
                    EVENT_QUEUE_SEED_LAYOUT.format(
                        group=market_product_group, key=product_key
                    )
                )),
                lamports=_calc_rent(event_queue_size),
                space=event_queue_size,
                program_id=pids.AOB_PROGRAM_ID,
            )),
        sp.create_account_with_seed(
            sp.CreateAccountWithSeedParams(
                from_pubkey=authority,
                new_account_pubkey=bids,
                base_pubkey=authority,
                seed=_to_rust_string_for_construct(crush(
                    BIDS_SEED_LAYOUT.format(group=market_product_group, key=product_key)
                )),
                lamports=_calc_rent(bids_size),
                space=bids_size,
                program_id=pids.AOB_PROGRAM_ID,
            )),
        sp.create_account_with_seed(
            sp.CreateAccountWithSeedParams(
                from_pubkey=authority,
                new_account_pubkey=asks,
                base_pubkey=authority,
                seed=_to_rust_string_for_construct(crush(
                    ASKS_SEED_LAYOUT.format(group=market_product_group, key=product_key)
                )),
                lamports=_calc_rent(asks_size),
                space=asks_size,
                program_id=pids.AOB_PROGRAM_ID,
            )),
        create_market_aob_ix(
            market=orderbook,
            event_queue=event_queue,
            bids=bids,
            asks=asks,
            caller_authority=orderbook_authority,
            callback_info_len=callback_info_len,
            callback_id_len=callback_id_len,
            min_base_order_size=min_base_order_size,
        ),
    )


def get_agnostic_orderbook_authority(product_key: PublicKey):
    return PublicKey.find_program_address([bytes(product_key)], DEX_PROGRAM_ID)[0]


@actionify
def create_market_product(
        authority: PublicKey,
        market_product_group: PublicKey,
        product_key: PublicKey,
        name: str,
        callback_info_len: int = 32,
        callback_id_len: int = 32,
        min_base_order_size: int = 1,
        orderbook_size: int = DEFAULT_ORDERBOOK_SIZE,
        event_queue_size: int = DEFAULT_EVENT_QUEUE_SIZE,
        asks_size: int = DEFAULT_ASKS_SIZE,
        bids_size: int = DEFAULT_BIDS_SIZE,
        tick_size: float = DEFAULT_TICK_SIZE,
        base_decimals: int = DEFAULT_DECIMALS,
        price_offset: float = DEFAULT_OFFSET,
):
    tick_size = Fractional.to_decimal(tick_size)
    price_offset = Fractional.to_decimal(price_offset)
    orderbook, _, _, _ = get_orderbook_addrs(
        authority, market_product_group, product_key
    )
    orderbook_authority = get_agnostic_orderbook_authority(product_key)
    return Transaction(fee_payer=authority).add(
        create_aob_orderbook_helper(
            authority,
            market_product_group,
            product_key,
            orderbook_authority,
            callback_info_len=callback_info_len,
            callback_id_len=callback_id_len,
            min_base_order_size=min_base_order_size,
            orderbook_size=orderbook_size,
            event_queue_size=event_queue_size,
            asks_size=asks_size,
            bids_size=bids_size,
        ),
        ixs.initialize_market_product(
            authority=authority,
            market_product_group=market_product_group,
            product=product_key,
            orderbook=orderbook,
            params=dex_types.InitializeMarketProductParams(
                name=name.encode("utf-8").ljust(16, b"\x00"),
                tick_size=tick_size,
                base_decimals=base_decimals,
                price_offset=price_offset,
            ),
        ),
    )


# Format of the seeds is [product_key_1, ..., product_key_N, [ratio_1, ..., ratio_N]]
def get_combo_product_key(product_keys, ratios):
    sort_idx = [i[0] for i in sorted(enumerate(product_keys), key=lambda x: str(x[1]))]
    product_keys = [product_keys[idx] for idx in sort_idx]
    ratios = [ratios[idx] for idx in sort_idx]
    seeds = [bytes(product_key) for product_key in product_keys] + [
        I8.to_bytes(r) for r in ratios
    ]
    return PublicKey.find_program_address(seeds, DEX_PROGRAM_ID)[0]


@actionify
def create_combo(
        authority: PublicKey,
        market_product_group: PublicKey,
        products: List[PublicKey],
        ratios: List[int],
        name: str,
        callback_info_len: int = 32,
        callback_id_len: int = 32,
        min_base_order_size: int = 1,
        orderbook_size: int = DEFAULT_ORDERBOOK_SIZE,
        event_queue_size: int = DEFAULT_EVENT_QUEUE_SIZE,
        asks_size: int = DEFAULT_ASKS_SIZE,
        bids_size: int = DEFAULT_BIDS_SIZE,
        tick_size: float = DEFAULT_TICK_SIZE,
        base_decimals: int = DEFAULT_DECIMALS,
        price_offset: float = DEFAULT_OFFSET,
):
    product_key = get_combo_product_key(products, ratios)
    orderbook_authority = get_agnostic_orderbook_authority(product_key)

    tick_size = Fractional.to_decimal(tick_size)
    price_offset = Fractional.to_decimal(price_offset)
    orderbook, _, _, _ = get_orderbook_addrs(
        authority, market_product_group, product_key
    )
    return Transaction(fee_payer=authority).add(
        create_aob_orderbook_helper(
            authority,
            market_product_group,
            product_key,
            orderbook_authority,
            callback_info_len=callback_info_len,
            callback_id_len=callback_id_len,
            min_base_order_size=min_base_order_size,
            orderbook_size=orderbook_size,
            event_queue_size=event_queue_size,
            asks_size=asks_size,
            bids_size=bids_size,
        ),
        ixs.initialize_combo(
            authority=authority,
            market_product_group=market_product_group,
            orderbook=orderbook,
            params=dex_types.InitializeComboParams(
                name=name.encode("utf-8").ljust(16, b"\x00"),
                ratios=ratios,
                tick_size=tick_size,
                base_decimals=base_decimals,
                price_offset=price_offset,
            ),
            remaining_accounts=[AccountMeta(pubkey=p, is_signer=False, is_writable=False) for p in products]
        ),
    )


def _post_create_trader_risk_group(resp):
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


_risk_state_account_num = 0
@actionify(post_process=_post_create_trader_risk_group)
def create_trader_risk_group(
        trader: PublicKey,
        market_product_group: PublicKey,
        fee_ix: Optional[Any] = None,
        program_id: Optional[PublicKey] = pids.ALPHA_RISK_ENGINE_PROGRAM_ID,
):
    trader_risk_group = dex_addrs.get_trader_risk_group_addr(trader, market_product_group)
    space = TraderRiskGroup.calc_size() + 8
    rent = _calc_rent(space)

    trader_fee_acct = dex_addrs.get_trader_fee_state_acct(trader_risk_group, market_product_group)

    if program_id == pids.ALPHA_RISK_ENGINE_PROGRAM_ID:
        register_size = alpha_state.Health.calc_max_size() + 8
    else:
        raise Exception("Not implemented")

    risk_state_account = Keypair.generate()
    global _risk_state_account_num
    Context.add_signers((risk_state_account, "risk_state_account" + str(_risk_state_account_num)))
    _risk_state_account_num = _risk_state_account_num + 1

    if fee_ix is None:
        import dexterity.constant_fees.instructions as fee_ixs

        fee_model_config_acct = dex_addrs.get_fee_model_configuration_addr(
            market_product_group, pids.CONSTANT_FEES_MODEL_PROGRAM_ID
        )
        fee_ix = fee_ixs.initialize_trader_acct_ix(
            pids.CONSTANT_FEES_MODEL_PROGRAM_ID,
            trader,
            fee_model_config_acct,
            trader_fee_acct,
            market_product_group,
            trader_risk_group=trader_risk_group,
            system_program=pids.SYSTEM_PROGRAM_ID,
        )

    return Transaction(fee_payer=trader).add(
        sp.create_account_with_seed(
            sp.CreateAccountWithSeedParams(
                from_pubkey=trader,
                new_account_pubkey=trader_risk_group,
                base_pubkey=trader,
                seed=_to_rust_string_for_construct(crush(
                    TRADER_RISK_GROUP_SEED_LAYOUT.format(
                        market_product_group=market_product_group
                    )
                )),
                lamports=rent,
                space=space,
                program_id=pids.DEX_PROGRAM_ID,
            )),
        fee_ix,
        ixs.initialize_trader_risk_group(
            owner=trader,
            trader_risk_group=trader_risk_group,
            market_product_group=market_product_group,
            risk_signer=dex_addrs.get_risk_signer(mpg=market_product_group),
            risk_state_account=risk_state_account.public_key,
            fee_state_account=trader_fee_acct,
            risk_engine_program=pids.ALPHA_RISK_ENGINE_PROGRAM_ID,
        ),
    )


def _post_init_mint(resp):
    addr = resp.instructions[0]["accounts"][1]
    exists = False
    if resp.error:
        error_ix, error_info = resp.error["InstructionError"]
        print(error_info)
        if error_ix == 0 and type(error_info) != str and error_info["Custom"] == 0:
            exists = True
    else:
        exists = True

    if exists:
        return addr, resp
    else:
        return None, resp


@actionify(post_process=_post_init_mint)
def init_mint(
        authority: PublicKey,
        mint: PublicKey,
        mint_decimals: int = 6,
):
    from spl import token
    from spl.token import instructions as spl_ixs
    create_mint_ix = system_program.create_account(system_program.CreateAccountParams(
        from_pubkey=authority,
        new_account_pubkey=mint,
        lamports=1000_000_000,
        space=token.constants.MINT_LEN,
        program_id=token.constants.TOKEN_PROGRAM_ID,
    ))

    init_mint_ix = spl_ixs.initialize_mint(spl_ixs.InitializeMintParams(
        decimals=mint_decimals,
        freeze_authority=authority,
        mint=mint,
        mint_authority=authority,
        program_id=token.constants.TOKEN_PROGRAM_ID,
    ))
    return Transaction().add(create_mint_ix, init_mint_ix)


def _post_init_trader_mint_account(resp):
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


@actionify(post_process=_post_init_trader_mint_account)
def init_trader_mint_account(
        trader: PublicKey,
        mint: PublicKey,
):
    return Transaction(fee_payer=trader).add(
        create_associated_token_account(trader, trader, mint)
    )


@actionify
def mint_to_trader(
        trader: PublicKey, mint: PublicKey, mint_authority: PublicKey, amount, decimals
):
    token_address = get_associated_token_address(trader, mint)
    return Transaction(fee_payer=trader).add(
        mint_to_checked(
            MintToCheckedParams(
                TOKEN_PROGRAM_ID,
                mint,
                token_address,
                mint_authority,
                amount,
                decimals,
            )
        )
    )


@pod
class DexOrderSummary:
    posted_order_id: Option[U128]
    total_base_qty: U64
    total_quote_qty: U64
    total_base_qty_posted: U64

    @classmethod
    def from_bytes_partial(cls, buffer: bytes) -> Tuple[object, bytes]:
        obj, remaining = super().from_bytes_partial(buffer)

        # TODO do any modification needed here (re fixed-point stuff)

        return obj, remaining


def _post_new_order(resp):
    if resp.error:
        return None, resp

    raw_summary = resp.emitted_logs["new-order:order-summary"]

    return DexOrderSummary.from_bytes(raw_summary), resp


@actionify(post_process=_post_new_order)
def new_order(
        trader: PublicKey,
        market_product_group: PublicKey,
        product_key: PublicKey,
        side: Side,
        limit_price: U64,
        max_base_qty: U64,
        order_type: dex_types.OrderType,
        trader_fee_acct: Optional[PublicKey] = None,
        fee_model_program: Optional[PublicKey] = pids.CONSTANT_FEES_MODEL_PROGRAM_ID,
        fee_register_acct: Optional[PublicKey] = None,
        risk_model_configuration_acct: Optional[PublicKey] = None,
        risk_register_acct: Optional[PublicKey] = None,
        risk_state_account: Optional[PublicKey] = None,
        self_trade_behavior: SelfTradeBehavior = SelfTradeBehavior.CANCEL_PROVIDE,
        match_limit: U64 = DEFAULT_MATCH_LIMIT,
        tick_size: Optional[U64] = None,
        decimals: Optional[U64] = None,
        authority: Optional[PublicKey] = None,
        risk_engine_program: Optional[PublicKey] = None,
):
    if (
            authority is None
            or decimals is None
            or tick_size is None
            or risk_engine_program is None
            or fee_model_program is None
    ):
        group_details = fetch_account_details(market_product_group)
        group_obj: MarketProductGroup = group_details.data_obj
        authority = group_obj.authority
        decimals = group_obj.decimals
        product = group_obj.get_product_by_key(
            product_key
        ) or group_obj.get_combo_by_key(product_key)
        tick_size = product.tick_size
        risk_engine_program = group_obj.risk_engine_program_id
        fee_model_program = group_obj.fee_model_program_id
        risk_model_configuration_acct = group_obj.risk_model_configuration_acct

    fee_model_configuration_acct = ixs.common.get_fee_model_configuration_addr(
        market_product_group,
        fee_model_program,
    )

    trader_risk_group = dex_addrs.get_trader_risk_group_addr(trader, market_product_group)
    if trader_fee_acct is None:
        trader_fee_acct = dex_addrs.get_trader_fee_state_acct(
            trader_risk_group,
            market_product_group,
            fee_model_program,
        )

    if risk_register_acct is None:
        out_register_acct = dex_addrs.get_risk_register_addr(
            authority,
            market_product_group,
            risk_engine_program,
            OUT_REGISTER_RISK_LAYOUT,
        )
    if risk_state_account is None:
        risk_state_account = dex_addrs.get_risk_register_addr(
            trader,
            trader_risk_group,
            risk_engine_program,
            IN_REGISTER_RISK_LAYOUT,
        )

    if fee_register_acct is None:
        fee_register_acct = dex_addrs.get_fee_register_addr(
            authority,
            market_product_group,
            fee_model_program,
            FEE_REGISTER_LAYOUT,
        )

    (orderbook, event_queue, bids, asks) = get_orderbook_addrs(
        authority,
        market_product_group,
        product_key,
    )
    limit_price = Fractional.to_decimal(limit_price)
    max_base_qty = Fractional.to_decimal(max_base_qty)

    return Transaction(fee_payer=trader).add(
        ixs.new_order(
            user=trader,
            trader_risk_group=trader_risk_group,
            market_product_group=market_product_group,
            product=product_key,
            orderbook=orderbook,
            event_queue=event_queue,
            bids=bids,
            asks=asks,
            params=dex_types.NewOrderParams(
                side=side,
                limit_price=limit_price,
                max_base_qty=max_base_qty,
                order_type=order_type,
                self_trade_behavior=self_trade_behavior,
                match_limit=match_limit,
            ),
            risk_engine_program=risk_engine_program,
            risk_model_configuration_acct=risk_model_configuration_acct,
            fee_model_configuration_acct=fee_model_configuration_acct,
            fee_output_register=fee_register_acct,
            trader_fee_state=trader_fee_acct,
            fee_model_program=fee_model_program,
            risk_output_register=out_register_acct,
            risk_state_account_info=risk_state_account,
        )
    )


@actionify
def cancel_order(
        trader: PublicKey,
        market_product_group: PublicKey,
        product_key: PublicKey,
        order_id: U128,
        authority: Optional[PublicKey] = None,
        risk_engine_program: Optional[PublicKey] = None,
        risk_register_acct: Optional[PublicKey] = None,
        risk_state_account: Optional[PublicKey] = None,
):
    if authority is None or risk_engine_program is None:
        group_details = fetch_account_details(market_product_group)
        group_obj: MarketProductGroup = group_details.data_obj
        authority = group_obj.authority
        risk_engine_program = group_obj.risk_engine_program_id

    trader_risk_group = dex_addrs.get_trader_risk_group_addr(trader, market_product_group)

    if risk_register_acct is None:
        risk_register_acct = dex_addrs.get_risk_register_addr(
            authority,
            market_product_group,
            risk_engine_program,
            OUT_REGISTER_RISK_LAYOUT,
        )

    if risk_state_account is None:
        risk_state_account = dex_addrs.get_risk_register_addr(
            trader,
            trader_risk_group,
            risk_engine_program,
            IN_REGISTER_RISK_LAYOUT,
        )

    (orderbook, event_queue, bids, asks) = get_orderbook_addrs(
        authority,
        market_product_group,
        product_key,
    )

    return Transaction(fee_payer=trader).add(
        ixs.cancel_order(
            user=trader,
            trader_risk_group=trader_risk_group,
            market_product_group=market_product_group,
            product=product_key,
            orderbook=orderbook,
            event_queue=event_queue,
            bids=bids,
            asks=asks,
            risk_output_register=risk_register_acct,
            risk_engine_program=risk_engine_program,
            risk_state_account_info=risk_state_account,
            aaob_program=pids.AOB_PROGRAM_ID,
            market_signer=authority,
            risk_model_configuration_acct=dex_addrs.get_risk_model_configuration_addr(market_product_group,
                                                                                      risk_engine_program),
            params=dex_types.CancelOrderParams(
                order_id=order_id,
            ),
        )
    )


@actionify
def consume_orderbook_events(
        market_product_group: PublicKey,
        product_key: PublicKey,
        reward_target: PublicKey,
        max_iterations: U64,
        risk_model_configuration_acct: Optional[PublicKey] = None,
        fee_register_acct: Optional[PublicKey] = None,
        user_accounts: Optional[List[PublicKey]] = None,
        authority: Optional[PublicKey] = None,
        fee_program: PublicKey = None,
        risk_output_register: Optional[PublicKey] = None,
        risk_engine_program: Optional[PublicKey] = None,
):
    if authority is None or fee_program is None or risk_engine_program is None:
        group_details = fetch_account_details(market_product_group)
        group_obj: MarketProductGroup = group_details.data_obj
        authority = group_obj.authority
        fee_program = group_obj.fee_model_program_id
        risk_engine_program = group_obj.risk_engine_program_id
        risk_model_configuration_acct = group_obj.risk_model_configuration_acct
        risk_output_register = group_obj.risk_output_register

    orderbook, event_queue, *_ = get_orderbook_addrs(
        authority,
        market_product_group,
        product_key,
    )

    if user_accounts is None:
        event_queue_details = fetch_account_details(event_queue)
        event_queue_obj: Slab = event_queue_details.data_obj

        if event_queue_obj.header.count == 0:
            raise RuntimeError("No event is in event_queue")

        trader_risk_groups = []
        n_events_to_process = min(max_iterations, event_queue_obj.header.count)
        for i in range(n_events_to_process):
            trader_risk_groups.extend(event_queue_obj[i].get_user_accounts())

        trader_risk_groups = list(
            map(PublicKey, set(list(map(bytes, trader_risk_groups))))
        )
        user_accounts = []
        for trader in trader_risk_groups:
            trader_fee_account = dex_addrs.get_trader_fee_state_acct(trader, market_product_group)
            trader_group_details = fetch_account_details(trader)
            trader_group_obj: TraderRiskGroup = trader_group_details.data_obj
            user_accounts.extend(
                [trader, trader_fee_account, trader_group_obj.risk_register]
            )

    if fee_register_acct is None:
        fee_register_acct = dex_addrs.get_risk_register_addr(
            authority,
            market_product_group,
            fee_program,
            FEE_REGISTER_LAYOUT,
        )

    risk_and_fee_signer, _ = PublicKey.find_program_address(
        seeds=[
            bytes(market_product_group),
        ],
        program_id=pids.DEX_PROGRAM_ID,
    )

    return Transaction(fee_payer=reward_target).add(
        ixs.consume_orderbook_events(
            market_product_group=market_product_group,
            product=product_key,
            orderbook=orderbook,
            event_queue=event_queue,
            reward_target=reward_target,
            fee_output_register=fee_register_acct,
            risk_and_fee_signer=risk_and_fee_signer,
            fee_model_program=fee_program,
            aaob_program=pids.AOB_PROGRAM_ID,
            fee_model_configuration_acct=dex_addrs.get_fee_model_configuration_addr(market_product_group),
            market_signer=authority,
            params=dex_types.ConsumeOrderbookEventsParams(
                max_iterations=max_iterations,
            ),
            remaining_accounts=user_accounts
        )
    )


@actionify
def deposit_funds(
        trader: PublicKey,
        trader_wallet: PublicKey,
        market_product_group: PublicKey,
        quantity: U64,
):
    trader_risk_group = dex_addrs.get_trader_risk_group_addr(trader, market_product_group)
    return Transaction(fee_payer=trader).add(
        ixs.deposit_funds(
            user=trader,
            user_token_account=trader_wallet,
            trader_risk_group=trader_risk_group,
            market_product_group=market_product_group,
            market_product_group_vault=dex_addrs.get_market_product_group_vault(market_product_group),
            params=dex_types.DepositFundsParams(
                quantity=Fractional.to_decimal(quantity),
            ),
        )
    )


@actionify
def withdraw_funds(
        trader: PublicKey,
        trader_wallet: PublicKey,
        market_product_group: PublicKey,
        quantity: U64,
):
    # todo fixme
    trader_risk_group = dex_addrs.get_trader_risk_group_addr(trader, market_product_group)
    return Transaction(fee_payer=trader).add(
        ixs.withdraw_funds(
            user=trader,
            user_token_account=trader_wallet,
            trader_risk_group=trader_risk_group,
            market_product_group=market_product_group,
            market_product_group_vault=dex_addrs.get_market_product_group_vault(mpg_key=market_product_group),
            params=dex_types.WithdrawFundsParams(
                quantity=Fractional.to_decimal(quantity),
            ),
        )
    )


@actionify
def update_trader_funding(
        market_product_group: PublicKey,
        trader: Optional[PublicKey] = None,
        trader_risk_group: Optional[PublicKey] = None,
        fee_payer: Optional[PublicKey] = None,  # note: anyone can call this
):
    # todo fixme
    if (trader is None and trader_risk_group is None) or (
            trader is not None and trader_risk_group is not None
    ):
        raise RuntimeError("Exactly one of trader and trader_risk_group must be passed")

    if trader_risk_group is None:
        trader_risk_group = dex_addrs.get_trader_risk_group_addr(trader, market_product_group)
    return Transaction(fee_payer=trader).add(
        ixs.update_trader_funding_ix(
            market_product_group=market_product_group,
            trader_risk_group=trader_risk_group,
        )
    )


@actionify
def clear_expired_orders(
        market_product_group: PublicKey,
        product_key: PublicKey,
        authority: PublicKey = None,
        num_orders_to_cancel: int = 5,
):
    # todo fixme
    if authority is None:
        group_details = fetch_account_details(market_product_group)
        group_obj: MarketProductGroup = group_details.data_obj
        authority = group_obj.authority
    (orderbook, event_queue, bids, asks) = get_orderbook_addrs(
        authority,
        market_product_group,
        product_key,
    )
    return Transaction(fee_payer=authority).add(
        ixs.clear_expired_orderbook_ix(
            market_product_group=market_product_group,
            product_key=product_key,
            orderbook=orderbook,
            event_queue=event_queue,
            bids=bids,
            asks=asks,
            num_orders_to_cancel=num_orders_to_cancel,
        )
    )


@actionify
def remove_market_product(
        market_product_group: PublicKey,
        product_key: PublicKey,
        authority: PublicKey = None,
):
    # todo fixme
    if authority is None:
        group_details = fetch_account_details(market_product_group)
        group_obj: MarketProductGroup = group_details.data_obj
        authority = group_obj.authority
    (orderbook, event_queue, bids, asks) = get_orderbook_addrs(
        authority,
        market_product_group,
        product_key,
    )
    return Transaction(fee_payer=authority).add(
        ixs.remove_market_product_ix(
            authority=authority,
            market_product_group=market_product_group,
            product_key=product_key,
            orderbook=orderbook,
            event_queue=event_queue,
            bids=bids,
            asks=asks,
        )
    )
