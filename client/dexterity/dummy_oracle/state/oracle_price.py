from solana.publickey import PublicKey

from dexterity.dummy_oracle.state.common import AccountTag
from podite import pod, U64, I64


@pod
class OraclePrice:
    tag: AccountTag
    price: I64
    decimals: U64
    slot: U64
    update_authority: PublicKey
