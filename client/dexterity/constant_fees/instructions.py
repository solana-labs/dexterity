from podite import U8, I32, pod, Enum
from solana.publickey import PublicKey
from solana.transaction import AccountMeta, TransactionInstruction


@pod
class InstructionCode(Enum[U8]):
    FindFees = 0
    InitializeTraderAcct = 1
    UpdateFees = 2


@pod
class NoParams:
    instr: InstructionCode


@pod
class UpdateFeesParams:
    instr: InstructionCode
    maker_fee_bps: I32
    taker_fee_bps: I32


def update_fees_ix(
    program_id: PublicKey,
    payer: PublicKey,
    fee_model_config_acct: PublicKey,
    market_product_group: PublicKey,
    system_program: PublicKey,
    maker_fee_bps: int,
    taker_fee_bps: int,
) -> TransactionInstruction:
    keys = [
        AccountMeta(pubkey=payer, is_signer=True, is_writable=False),
        AccountMeta(pubkey=fee_model_config_acct, is_signer=False, is_writable=True),
        AccountMeta(pubkey=market_product_group, is_signer=False, is_writable=False),
        AccountMeta(pubkey=system_program, is_signer=False, is_writable=False),
    ]
    return TransactionInstruction(
        keys=keys,
        program_id=program_id,
        data=UpdateFeesParams.to_bytes(UpdateFeesParams(
            instr=InstructionCode.UpdateFees,
            maker_fee_bps=maker_fee_bps,
            taker_fee_bps=taker_fee_bps,
        )),
    )


def initialize_trader_acct_ix(
    program_id: PublicKey,
    payer: PublicKey,
    fee_model_config_acct: PublicKey,
    trader_fee_acct: PublicKey,
    market_product_group: PublicKey,
    trader_risk_group: PublicKey,
    system_program: PublicKey,
) -> TransactionInstruction:
    keys = [
        AccountMeta(pubkey=payer, is_signer=True, is_writable=False),
        AccountMeta(pubkey=fee_model_config_acct, is_signer=False, is_writable=False),
        AccountMeta(pubkey=trader_fee_acct, is_signer=False, is_writable=True),
        AccountMeta(pubkey=market_product_group, is_signer=False, is_writable=False),
        AccountMeta(pubkey=trader_risk_group, is_signer=False, is_writable=False),
        AccountMeta(pubkey=system_program, is_signer=False, is_writable=False),
    ]
    return TransactionInstruction(
        program_id=program_id,
        keys=keys,
        data=NoParams.to_bytes(NoParams(instr=InstructionCode.InitializeTraderAcct)),
    )
