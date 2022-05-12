import base64
import json
from contextlib import contextmanager
from functools import wraps
from hashlib import sha256
from typing import Optional, Dict
from typing import Union, Tuple, Iterable

import base58
from solana.keypair import Keypair
from solana.publickey import PublicKey
from solana.rpc.api import Client
from solana.rpc.commitment import Confirmed
from solana.rpc.types import TxOpts
from solana.transaction import AccountMeta, Transaction, TransactionInstruction

DEX_LOG_PREFIX = "Program log: dex-log "


def calc_rent(space, client=None):
    if client is None:
        client = Context.get_global_client()
    return client.get_minimum_balance_for_rent_exemption(space)["result"]


class Context:
    client: Optional[Client] = None
    parser: Optional["AccountParser"] = None
    signers: Dict[bytes, Tuple[Keypair, str]] = {}
    fee_payer: Optional[Keypair] = None
    raise_on_error = False

    @staticmethod
    def init_globals(
            fee_payer: Keypair,
            client: Client,
            signers: Iterable[Tuple[Keypair, str]],
            parser: Optional["AccountParser"] = None,
            raise_on_error=False,
    ):
        Context.fee_payer = fee_payer
        Context.client = client
        Context.parser = parser
        Context.signers = {}
        Context.raise_on_error = raise_on_error
        Context.add_signers(*signers)

    @staticmethod
    def get_global_client():
        return Context.client

    @staticmethod
    def set_global_client(client):
        Context.client = client

    @staticmethod
    def get_global_parser():
        return Context.parser

    @staticmethod
    def set_global_parser(parser):
        Context.parser = parser

    @staticmethod
    def get_signers():
        return Context.signers

    # todo: rename to add signers
    @staticmethod
    def add_signers(*signers: Tuple[Keypair, str], verify=True):
        for (signer, name) in signers:
            if not isinstance(signer, Keypair) or not isinstance(name, str):
                raise ValueError(f"signers must be a list iterable of (Keypair, str) tuples. Found: {signer, name}")
            if bytes(signer.public_key) not in Context.signers:
                Context.signers[bytes(signer.public_key)] = (signer, name)
        if verify:
            names = set()
            for (_, name) in Context.signers.values():
                if name in names:
                    raise ValueError("Each signer name must be unique")
                names.add(name)

    @staticmethod
    def get_global_fee_payer():
        return Context.fee_payer

    @staticmethod
    def set_global_fee_payer(fee_payer: Keypair):
        Context.fee_payer = fee_payer

    @staticmethod
    def get_raise_on_error():
        return Context.raise_on_error

    @staticmethod
    def set_raise_on_error(raise_on_error: bool):
        Context.raise_on_error = raise_on_error


@contextmanager
def global_fee_payer(fee_payer):
    old_fee_payer = Context.get_global_fee_payer()
    Context.set_global_fee_payer(fee_payer)
    yield
    Context.set_global_fee_payer(old_fee_payer)


class AccountParser:
    _parsers: Dict[bytes, callable]  # key: program_id

    def __init__(self):
        self._parsers = dict()

    def register_parser(self, program_id, parser):
        self._parsers[bytes(program_id)] = parser

    def register_parser_from_account_enum(self, program_id: PublicKey, accounts_enum):
        def parser(info):
            return accounts_enum.from_bytes(info).field

        self.register_parser(program_id, parser)

    def parse(self, info):
        owner = info["result"]["value"]["owner"]
        data = info["result"]["value"]["data"][0]

        try:
            parser = self._parsers[base58.b58decode(owner)]
        except Exception as e:
            raise ValueError(f"Failed to find parser corresponding to account owner. Owner={owner}",
                             [PublicKey(p).to_base58() for p in self._parsers.keys()])
        return parser(base64.b64decode(data))


class TransactionDetails:
    def __init__(self, content, cluster="devnet"):
        self.content = content
        self.cluster = cluster

    @property
    def account_keys(self):
        accounts = self.content["result"]["transaction"]["message"]["accountKeys"]
        return [PublicKey(account) for account in accounts]

    @property
    def signatures(self):
        return self.content["result"]["transaction"]["signatures"]

    @property
    def tx_string(self):
        return self.signatures[0]

    @property
    def instructions(self):
        account_keys = self.account_keys
        raw_instructions = self.content["result"]["transaction"]["message"][
            "instructions"
        ]
        return [
            dict(
                accounts=[account_keys[int(i)] for i in instr["accounts"]],
                data=instr["data"],
            )
            for instr in raw_instructions
        ]

    @property
    def log_messages(self):
        return self.content["result"]["meta"]["logMessages"]

    @property
    def emitted_logs(self):
        result = dict()
        for msg in self.log_messages:
            if msg.startswith(DEX_LOG_PREFIX):
                key, val = msg[len(DEX_LOG_PREFIX):].split(" ")
                result[key] = base64.b64decode(val)

        return result

    @property
    def error(self):
        return self.content["result"]["meta"]["err"]

    def __str__(self) -> str:
        return f"TransactionDetails({self.tx_string})"

    def __repr__(self) -> str:
        return str(self)


class AccountDetails:
    def __init__(self, public_key, content, cluster="devnet"):
        self.public_key = public_key
        self.content = content
        self.cluster = cluster

    def __str__(self) -> str:
        return f"AccountDetails({self.public_key})"

    def __repr__(self) -> str:
        return str(self)

    @property
    def data(self):
        value = self.content["result"]["value"]
        if not value:
            return None

        return value["data"]

    @property
    def data_obj(self):
        parser = Context.get_global_parser()
        return parser.parse(self.content)


def fetch_transaction_details(addr, client=None):
    if client is None:
        client = Context.get_global_client()

    content = client.get_confirmed_transaction(
        addr,
        {
            "encoding": "json",
            "commitment": "confirmed",
        },
    )
    return TransactionDetails(content)


def fetch_account_details(addr, client=None):
    if client is None:
        client = Context.get_global_client()

    content = client.get_account_info(addr, commitment=Confirmed)
    return AccountDetails(addr, content)


def explore(addr):
    if isinstance(addr, str):
        if len(base58.b58decode(addr)) == 64:
            kind = "tx"
        else:
            kind = "acc"
            addr = PublicKey(addr)
    elif isinstance(addr, PublicKey):
        kind = "acc"
    else:
        raise ValueError()

    if kind == "tx":
        return fetch_transaction_details(addr)
    else:
        return fetch_account_details(addr)


def send_instructions(
        *ixs: TransactionInstruction,
        **kwargs
) -> Union[TransactionDetails, AccountDetails]:
    return send_transaction(Transaction().add(*ixs), **kwargs)


def send_transaction(
        tx,
        *signers: Keypair,
        opts=TxOpts(
            skip_preflight=True,
            skip_confirmation=False,
            preflight_commitment=Confirmed,
        ),
        recent_blockhash=None,
        client=None,
        raise_on_error=None,
) -> Union[TransactionDetails, AccountDetails]:
    if fee_payer := Context.get_global_fee_payer():
        tx = Transaction(fee_payer=fee_payer.public_key).add(tx)

    raise_on_error = raise_on_error if raise_on_error is not None else Context.get_raise_on_error()

    if len(signers) == 0:
        signers = Context.get_signers()
    else:
        signers = {bytes(signer.public_key): (signer, f"arg  {i}") for i, signer in enumerate(signers)}

    if client is None:
        client = Context.get_global_client()

    # filtering private keys to only contain the relevant ones
    # otherwise, there will be a problem with fee_payer
    signers_public_keys = []
    if tx.fee_payer:
        signers_public_keys.append(tx.fee_payer)
    for ix in tx.instructions:
        for i, meta in enumerate(ix.keys):
            if not isinstance(meta, AccountMeta):
                print(f'{i} is {meta}')
            if meta.is_signer and meta.pubkey not in signers_public_keys:
                signers_public_keys.append(meta.pubkey)

    signer_keypairs = []
    for pk in signers_public_keys:
        if bytes(pk) not in signers:
            names = [(name, PublicKey(p).to_base58()) for p, (_, name) in signers.items()]
            raise ValueError(f"Required signer PublicKey not in list of Keypairs. Have {names}, want: {pk}")
        signer_keypairs.append(signers[bytes(pk)][0])

    result = client.send_transaction(
        tx, *signer_keypairs, opts=opts, recent_blockhash=recent_blockhash
    )
    parsed_result = explore(result["result"])
    if parsed_result.error and raise_on_error:
        err_str = json.dumps(parsed_result.error)
        log_str = "\n".join(parsed_result.log_messages)
        raise ValueError(
            f"Transaction returned error:\n{err_str}, \nLog messages:\n{log_str}")

    return parsed_result


def actionify(func=None, /, post_process=lambda resp: (None, resp), raise_error=False):
    assert not raise_error, "Raise_error is not implemented"

    def _actionify(make):
        @wraps(make)
        def send(*args, **kwargs):
            tx = make(*args, **kwargs)
            if tx is None:
                return post_process(None)
            if isinstance(tx, TransactionInstruction):
                tx = Transaction().add(tx)

            opts = TxOpts(
                skip_preflight=True,
                skip_confirmation=False,
                preflight_commitment=Confirmed,
            )
            response = send_transaction(
                tx,
                opts=opts,
            )
            return post_process(response)

        send.make = make
        return send

    if func is None:
        return _actionify
    return _actionify(func)


def sighash(ix_name: str) -> bytes:
    """Not technically sighash, since we don't include the arguments.
    (Because Rust doesn't allow function overloading.)
    Args:
        ix_name: The instruction name.
    Returns:
        The sighash bytes.
    """
    formatted_str = f"global:{ix_name}"
    return sha256(formatted_str.encode()).digest()[:8]


def sighash_int(ix_name: str) -> int:
    return int.from_bytes(sighash(ix_name), byteorder="little")
