from podite import (
    pod,
    I32,
    U64,
)


@pod
class FeeConfig:
    maker_fee_bps: I32
    taker_fee_bps: I32

@pod
class TraderFeeState:
    bump: U64
