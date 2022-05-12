use anchor_lang::{
    prelude::*,
    solana_program::{
        log::sol_log_compute_units, program::invoke_signed_unchecked, program_pack::IsInitialized,
        pubkey::Pubkey,
    },
};
use borsh::BorshSerialize;

use crate::{
    error::{DexError, DomainOrProgramResult, UtilError},
    state::{
        constants::{MAX_OUTRIGHTS, MAX_TRADER_POSITIONS},
        enums::AccountTag,
        products::Product,
        risk_engine_register::*,
    },
    utils::{
        cpi::risk_check,
        loadable::Loadable,
        numeric::{Fractional, ZERO_FRAC},
        validation::{assert, assert_keys_equal},
    },
    TransferFullPosition,
};
use ::std::cell::Ref;

fn validate(accts: &TransferFullPosition) -> DomainOrProgramResult {
    let liquidatee_risk_group = accts.liquidatee_risk_group.load()?;
    let liquidator_risk_group = accts.liquidator_risk_group.load()?;
    let market_product_group = accts.market_product_group.load()?;
    assert_keys_equal(
        accts.risk_engine_program.key(),
        market_product_group.risk_engine_program_id,
    )?;
    assert_keys_equal(liquidator_risk_group.owner, *accts.liquidator.key)?;
    assert_keys_equal(
        liquidatee_risk_group.market_product_group,
        accts.market_product_group.key(),
    )?;
    assert_keys_equal(
        liquidator_risk_group.market_product_group,
        accts.market_product_group.key(),
    )?;
    assert_keys_equal(
        *accts.risk_engine_program.key,
        market_product_group.risk_engine_program_id,
    )?;
    assert_keys_equal(
        accts.liquidatee_risk_state_account_info.key(),
        liquidatee_risk_group.risk_state_account,
    )?;
    assert_keys_equal(
        accts.liquidator_risk_state_account_info.key(),
        liquidator_risk_group.risk_state_account,
    )?;
    assert(
        liquidatee_risk_group.is_initialized(),
        UtilError::AccountUninitialized,
    )?;
    assert(
        liquidator_risk_group.is_initialized(),
        UtilError::AccountUninitialized,
    )?;
    assert_keys_equal(accts.liquidator.key(), liquidator_risk_group.owner)?;
    assert_keys_equal(
        accts.risk_model_configuration_acct.key(),
        market_product_group.risk_model_configuration_acct,
    )?;
    assert(
        liquidatee_risk_group.open_orders.total_open_orders == 0,
        DexError::UserAccountStillActive,
    )?;
    Ok(())
}

pub fn process<'info>(
    ctx: Context<'_, '_, '_, 'info, TransferFullPosition<'info>>,
) -> DomainOrProgramResult {
    let accts = ctx.accounts;
    validate(accts)?;
    let mut liquidatee_risk_group = accts.liquidatee_risk_group.load_mut()?;
    let mut liquidator_risk_group = accts.liquidator_risk_group.load_mut()?;
    let mut market_product_group = accts.market_product_group.load_mut()?;

    // Apply all unsettled funding prior to calling the risk engine
    liquidator_risk_group.apply_all_funding(&mut market_product_group)?;
    liquidatee_risk_group.apply_all_funding(&mut market_product_group)?;

    // Validate that the liquidatee is a liquidation candidate
    {
        let risk_engine_output = risk_check(
            &accts.risk_engine_program,
            &accts.market_product_group,
            &accts.liquidatee_risk_group,
            &accts.risk_output_register,
            &accts.liquidatee_risk_state_account_info,
            &accts.risk_model_configuration_acct,
            &accts.risk_signer,
            ctx.remaining_accounts,
            &OrderInfo {
                operation_type: OperationType::CheckHealth,
                ..Default::default()
            },
            market_product_group.get_validate_account_liquidation_discriminant(),
            market_product_group.risk_and_fee_bump as u8,
        )?;
        let mut liquidation_info = match risk_engine_output {
            HealthResult::Health { health_info: _ } => {
                return Err(DexError::InvalidAccountHealthError.into());
            }
            HealthResult::Liquidation {
                liquidation_info: v,
            } => v,
        };

        if liquidation_info.health != HealthStatus::Liquidatable {
            return Err(DexError::InvalidAccountHealthError.into());
        }
        msg!("Liquidatee account health is below liquidation threshold");
        let social_losses = liquidation_info.social_losses;
        let cash_decimals = market_product_group.decimals;
        let mut total_social_loss = ZERO_FRAC;
        // Attempt to transfer over full position
        for (mut liquidatee_position, social_loss) in liquidatee_risk_group
            .trader_positions
            .iter_mut()
            .zip(social_losses.iter())
        {
            if !liquidatee_position.is_initialized() {
                continue;
            }
            let product_index = liquidatee_position.product_index as usize;
            let market_product =
                market_product_group.market_products[product_index].try_to_outright()?;
            // For now outright assertion is sufficient for combos
            assert(
                liquidatee_position.pending_position == ZERO_FRAC,
                DexError::UserAccountStillActive,
            )?;
            if liquidatee_position.position == ZERO_FRAC {
                continue;
            }

            liquidator_risk_group.activate_if_uninitialized(
                product_index,
                &liquidatee_position.product_key,
                market_product.cum_funding_per_share,
                market_product.cum_social_loss_per_share,
                market_product_group.active_combos(),
            )?;
            let liquidator_index = liquidator_risk_group.active_products[product_index] as usize;
            let liquidator_position = &mut liquidator_risk_group.trader_positions[liquidator_index];
            let (buyer_short_position, seller_long_position) =
                if liquidatee_position.position > ZERO_FRAC {
                    (
                        liquidator_position.position.min(ZERO_FRAC).abs(),
                        liquidatee_position.position,
                    )
                } else {
                    (
                        liquidatee_position.position.abs(),
                        liquidator_position.position.max(ZERO_FRAC),
                    )
                };
            let outright =
                market_product_group.market_products[product_index].try_to_outright_mut()?;
            outright.update_open_interest_change(
                liquidatee_position.position.abs(),
                buyer_short_position,
                seller_long_position,
            )?;
            liquidator_position.position = liquidator_position
                .position
                .checked_add(liquidatee_position.position)?;
            if liquidator_position.position == ZERO_FRAC {
                liquidator_position.tag = AccountTag::Uninitialized;
                liquidator_risk_group.active_products[product_index] = u8::max_value();
            }
            liquidatee_position.position = ZERO_FRAC;
            liquidatee_position.tag = AccountTag::Uninitialized;

            if social_loss.is_active() {
                assert(
                    liquidatee_position.product_index == social_loss.product_index,
                    DexError::ProductIndexMismatch,
                )?;
                if outright.open_long_interest == ZERO_FRAC {
                    liquidation_info.total_social_loss = liquidation_info
                        .total_social_loss
                        .checked_sub(social_loss.amount)?;
                } else {
                    total_social_loss = total_social_loss.checked_add(social_loss.amount)?;
                    market_product_group.market_products[product_index]
                        .try_to_outright_mut()?
                        .apply_social_loss(social_loss.amount, cash_decimals)?;
                }
            }
        }
        liquidatee_risk_group.active_products = [u8::MAX; MAX_OUTRIGHTS];
        if total_social_loss != liquidation_info.total_social_loss {
            return Err(DexError::InvalidSocialLossCalculation.into());
        }
        assert(
            liquidatee_risk_group.pending_cash_balance == ZERO_FRAC,
            DexError::UserAccountStillActive,
        )?;
        let liquidatee_cash = if liquidation_info.liquidation_price.m > 0 {
            liquidation_info
                .liquidation_price
                .checked_sub(liquidation_info.total_social_loss)?
        } else {
            ZERO_FRAC
        };
        liquidator_risk_group.cash_balance = liquidator_risk_group
            .cash_balance
            .checked_add(liquidatee_risk_group.cash_balance)?
            .checked_sub(liquidation_info.liquidation_price)?;
        liquidatee_risk_group.cash_balance = liquidatee_cash;
    }

    {
        // Validate that the liquidator's account is still healthy
        let risk_engine_output = risk_check(
            &accts.risk_engine_program,
            &accts.market_product_group,
            &accts.liquidator_risk_group,
            &accts.risk_output_register,
            &accts.liquidator_risk_state_account_info,
            &accts.risk_model_configuration_acct,
            &accts.risk_signer,
            ctx.remaining_accounts,
            &OrderInfo {
                operation_type: OperationType::PositionTransfer,
                ..Default::default()
            },
            market_product_group.get_validate_account_health_discriminant(),
            market_product_group.risk_and_fee_bump as u8,
        )?;
        let health_info = match risk_engine_output {
            HealthResult::Health { health_info: v } => v,
            HealthResult::Liquidation {
                liquidation_info: _,
            } => return Err(DexError::InvalidAccountHealthError.into()),
        };
        if health_info.action != ActionStatus::Approved {
            return Err(DexError::InvalidAccountHealthError.into());
        }
    }
    market_product_group.sequence_number += 1;
    msg!("sequence: {}", market_product_group.sequence_number);
    accts.market_product_group.key().log();
    Ok(())
}
