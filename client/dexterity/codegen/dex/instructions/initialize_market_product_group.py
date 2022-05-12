# LOCK-BEGIN[imports]: DON'T MODIFY
from .instruction_tag import InstructionTag
from dataclasses import dataclass
from dexterity.codegen.dex.types import InitializeMarketProductGroupParams
from io import BytesIO
from podite import BYTES_CATALOG
from solana.publickey import PublicKey
from solana.transaction import (
    AccountMeta,
    TransactionInstruction,
)
from solmate.utils import to_account_meta
from typing import (
    List,
    Optional,
    Union,
)

# LOCK-END


# LOCK-BEGIN[ix_cls(initialize_market_product_group)]: DON'T MODIFY
@dataclass
class InitializeMarketProductGroupIx:
    program_id: PublicKey

    # account metas
    authority: AccountMeta
    market_product_group: AccountMeta
    market_product_group_vault: AccountMeta
    vault_mint: AccountMeta
    fee_collector: AccountMeta
    fee_model_program: AccountMeta
    fee_model_configuration_acct: AccountMeta
    risk_model_configuration_acct: AccountMeta
    risk_engine_program: AccountMeta
    sysvar_rent: AccountMeta
    system_program: AccountMeta
    token_program: AccountMeta
    fee_output_register: AccountMeta
    risk_output_register: AccountMeta
    remaining_accounts: Optional[List[AccountMeta]]

    # data fields
    params: InitializeMarketProductGroupParams

    def to_instruction(self):
        keys = []
        keys.append(self.authority)
        keys.append(self.market_product_group)
        keys.append(self.market_product_group_vault)
        keys.append(self.vault_mint)
        keys.append(self.fee_collector)
        keys.append(self.fee_model_program)
        keys.append(self.fee_model_configuration_acct)
        keys.append(self.risk_model_configuration_acct)
        keys.append(self.risk_engine_program)
        keys.append(self.sysvar_rent)
        keys.append(self.system_program)
        keys.append(self.token_program)
        keys.append(self.fee_output_register)
        keys.append(self.risk_output_register)
        if self.remaining_accounts is not None:
            keys.extend(self.remaining_accounts)

        buffer = BytesIO()
        buffer.write(InstructionTag.to_bytes(InstructionTag.INITIALIZE_MARKET_PRODUCT_GROUP))
        buffer.write(BYTES_CATALOG.pack(InitializeMarketProductGroupParams, self.params))

        return TransactionInstruction(
            keys=keys,
            program_id=self.program_id,
            data=buffer.getvalue(),
        )

# LOCK-END


# LOCK-BEGIN[ix_fn(initialize_market_product_group)]: DON'T MODIFY
def initialize_market_product_group(
    authority: Union[str, PublicKey, AccountMeta],
    market_product_group: Union[str, PublicKey, AccountMeta],
    market_product_group_vault: Union[str, PublicKey, AccountMeta],
    vault_mint: Union[str, PublicKey, AccountMeta],
    fee_collector: Union[str, PublicKey, AccountMeta],
    fee_model_program: Union[str, PublicKey, AccountMeta],
    fee_model_configuration_acct: Union[str, PublicKey, AccountMeta],
    risk_model_configuration_acct: Union[str, PublicKey, AccountMeta],
    risk_engine_program: Union[str, PublicKey, AccountMeta],
    fee_output_register: Union[str, PublicKey, AccountMeta],
    risk_output_register: Union[str, PublicKey, AccountMeta],
    params: InitializeMarketProductGroupParams,
    sysvar_rent: Union[str, PublicKey, AccountMeta] = PublicKey("SysvarRent111111111111111111111111111111111"),
    system_program: Union[str, PublicKey, AccountMeta] = PublicKey("11111111111111111111111111111111"),
    token_program: Union[str, PublicKey, AccountMeta] = PublicKey("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"),
    remaining_accounts: Optional[List[AccountMeta]] = None,
    program_id: Optional[PublicKey] = None,
):
    if program_id is None:
        program_id = PublicKey("Dex1111111111111111111111111111111111111111")

    if isinstance(authority, (str, PublicKey)):
        authority = to_account_meta(
            authority,
            is_signer=True,
            is_writable=False,
        )
    if isinstance(market_product_group, (str, PublicKey)):
        market_product_group = to_account_meta(
            market_product_group,
            is_signer=False,
            is_writable=True,
        )
    if isinstance(market_product_group_vault, (str, PublicKey)):
        market_product_group_vault = to_account_meta(
            market_product_group_vault,
            is_signer=False,
            is_writable=True,
        )
    if isinstance(vault_mint, (str, PublicKey)):
        vault_mint = to_account_meta(
            vault_mint,
            is_signer=False,
            is_writable=False,
        )
    if isinstance(fee_collector, (str, PublicKey)):
        fee_collector = to_account_meta(
            fee_collector,
            is_signer=False,
            is_writable=False,
        )
    if isinstance(fee_model_program, (str, PublicKey)):
        fee_model_program = to_account_meta(
            fee_model_program,
            is_signer=False,
            is_writable=False,
        )
    if isinstance(fee_model_configuration_acct, (str, PublicKey)):
        fee_model_configuration_acct = to_account_meta(
            fee_model_configuration_acct,
            is_signer=False,
            is_writable=False,
        )
    if isinstance(risk_model_configuration_acct, (str, PublicKey)):
        risk_model_configuration_acct = to_account_meta(
            risk_model_configuration_acct,
            is_signer=False,
            is_writable=False,
        )
    if isinstance(risk_engine_program, (str, PublicKey)):
        risk_engine_program = to_account_meta(
            risk_engine_program,
            is_signer=False,
            is_writable=False,
        )
    if isinstance(sysvar_rent, (str, PublicKey)):
        sysvar_rent = to_account_meta(
            sysvar_rent,
            is_signer=False,
            is_writable=False,
        )
    if isinstance(system_program, (str, PublicKey)):
        system_program = to_account_meta(
            system_program,
            is_signer=False,
            is_writable=False,
        )
    if isinstance(token_program, (str, PublicKey)):
        token_program = to_account_meta(
            token_program,
            is_signer=False,
            is_writable=False,
        )
    if isinstance(fee_output_register, (str, PublicKey)):
        fee_output_register = to_account_meta(
            fee_output_register,
            is_signer=False,
            is_writable=False,
        )
    if isinstance(risk_output_register, (str, PublicKey)):
        risk_output_register = to_account_meta(
            risk_output_register,
            is_signer=False,
            is_writable=False,
        )

    return InitializeMarketProductGroupIx(
        program_id=program_id,
        authority=authority,
        market_product_group=market_product_group,
        market_product_group_vault=market_product_group_vault,
        vault_mint=vault_mint,
        fee_collector=fee_collector,
        fee_model_program=fee_model_program,
        fee_model_configuration_acct=fee_model_configuration_acct,
        risk_model_configuration_acct=risk_model_configuration_acct,
        risk_engine_program=risk_engine_program,
        sysvar_rent=sysvar_rent,
        system_program=system_program,
        token_program=token_program,
        fee_output_register=fee_output_register,
        risk_output_register=risk_output_register,
        remaining_accounts=remaining_accounts,
        params=params,
    ).to_instruction()

# LOCK-END
