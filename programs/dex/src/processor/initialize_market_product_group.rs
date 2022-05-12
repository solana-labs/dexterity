use anchor_lang::{
    prelude::*,
    solana_program::{
        log::sol_log_params,
        msg,
        program::invoke_signed,
        program_pack::Pack,
        pubkey::Pubkey,
        system_instruction,
        sysvar::{rent::Rent, Sysvar},
    },
};

use crate::{
    error::{DomainOrProgramResult, UtilError},
    state::{
        constants::{SLOTS_15_MIN, SLOTS_1_MIN, SLOTS_5_MIN, SLOTS_60_MIN},
        enums::AccountTag,
    },
    utils::validation::{assert_keys_equal, get_rent},
    InitializeMarketProductGroup, InitializeMarketProductGroupParams,
};

const TOKEN_ACCOUNT_SIZE: u64 = spl_token::state::Account::LEN as u64;

pub fn validate(ctx: &Context<InitializeMarketProductGroup>) -> DomainOrProgramResult {
    assert_keys_equal(
        *ctx.accounts.fee_model_configuration_acct.as_ref().owner,
        ctx.accounts.fee_model_program.key(),
    )?;
    Ok(())
}

pub fn process(
    ctx: Context<InitializeMarketProductGroup>,
    params: InitializeMarketProductGroupParams,
) -> DomainOrProgramResult {
    validate(&ctx)?;
    let accts = ctx.accounts;
    let mut market_product_group = accts.market_product_group.load_init().map_err(|e| {
        msg!("Failed to deserialize market product group");
        e
    })?;

    let vault_seeds_without_bump: &[&[u8]] = &[
        b"market_vault",
        &accts.market_product_group.key().to_bytes(),
    ];
    let (vault_key, vault_bump_seed) =
        Pubkey::find_program_address(vault_seeds_without_bump, ctx.program_id);
    let vault_seeds = &[
        vault_seeds_without_bump[0],
        vault_seeds_without_bump[1],
        &[vault_bump_seed],
    ];
    assert_keys_equal(vault_key, *accts.market_product_group_vault.key)?;
    msg!("Creating the market collateral vault");
    invoke_signed(
        &system_instruction::create_account(
            accts.authority.key,
            accts.market_product_group_vault.key,
            get_rent(
                &Rent::get()?,
                TOKEN_ACCOUNT_SIZE,
                &accts.market_product_group_vault,
            ),
            TOKEN_ACCOUNT_SIZE,
            accts.token_program.key,
        ),
        &[
            accts.authority.to_account_info(),
            accts.market_product_group_vault.clone(),
            accts.system_program.to_account_info(),
        ],
        &[vault_seeds],
    )?;

    msg!("Initializing the market collateral vault");
    invoke_signed(
        &spl_token::instruction::initialize_account2(
            accts.token_program.key,
            accts.market_product_group_vault.key,
            &accts.vault_mint.key(),
            accts.market_product_group_vault.key,
        )?,
        &[
            accts.market_product_group_vault.clone(),
            accts.vault_mint.to_account_info(),
            accts.sysvar_rent.clone(),
        ],
        &[vault_seeds],
    )?;
    let (_risk_and_fee_signer, risk_and_fee_bump) =
        Pubkey::find_program_address(&[accts.market_product_group.key().as_ref()], ctx.program_id);
    if market_product_group.tag != AccountTag::Uninitialized {
        msg!("MarketProductGroup account is already initialized");
        return Err(UtilError::AccountAlreadyInitialized.into());
    }
    market_product_group.tag = AccountTag::MarketProductGroup;
    market_product_group.name = params.name;
    market_product_group.authority = accts.authority.key();
    market_product_group.successor = accts.authority.key();
    market_product_group.vault_mint = accts.vault_mint.key();
    market_product_group.vault_bump = vault_bump_seed as u16;
    market_product_group.decimals = accts.vault_mint.decimals as u64;
    market_product_group.ewma_windows = [SLOTS_1_MIN, SLOTS_5_MIN, SLOTS_15_MIN, SLOTS_60_MIN];
    market_product_group.risk_engine_program_id = *accts.risk_engine_program.key;
    // discriminants
    market_product_group.validate_account_discriminant_len =
        params.validate_account_discriminant_len as u16;
    market_product_group.find_fees_discriminant_len = params.find_fees_discriminant_len as u16;
    market_product_group.find_fees_discriminant = params.find_fees_discriminant;
    market_product_group.validate_account_health_discriminant =
        params.validate_account_health_discriminant;
    market_product_group.create_risk_state_account_discriminant =
        params.create_risk_state_account_discriminant;
    market_product_group.validate_account_liquidation_discriminant =
        params.validate_account_liquidation_discriminant;
    // fees
    market_product_group.fee_collector = accts.fee_collector.key();
    market_product_group.fee_model_program_id = accts.fee_model_program.key();
    market_product_group.fee_model_configuration_acct = accts.fee_model_configuration_acct.key();
    market_product_group.max_maker_fee_bps = params.max_maker_fee_bps;
    market_product_group.min_maker_fee_bps = params.min_maker_fee_bps;
    market_product_group.max_taker_fee_bps = params.max_taker_fee_bps;
    market_product_group.min_taker_fee_bps = params.min_taker_fee_bps;
    // risk
    market_product_group.risk_model_configuration_acct = accts.risk_model_configuration_acct.key();
    // registers
    market_product_group.fee_output_register = accts.fee_output_register.key();
    market_product_group.risk_output_register = accts.risk_output_register.key();
    market_product_group.risk_and_fee_bump = risk_and_fee_bump as u16;
    market_product_group.sequence_number = 0;
    msg!("sequence: {}", market_product_group.sequence_number);
    accts.market_product_group.key().log();
    Ok(())
}
