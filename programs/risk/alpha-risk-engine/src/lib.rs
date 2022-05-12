use std::ops::Deref;

use anchor_lang::prelude::*;
use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::AccountInfo, declare_id, entrypoint::ProgramResult, program_error::ProgramError,
    program_pack::IsInitialized, pubkey::Pubkey,
};

use dex::{
    error::{DomainOrProgramError, DomainOrProgramResult},
    state::{
        constants::{MAX_OUTRIGHTS, MAX_TRADER_POSITIONS, NO_ASK_PRICE, NO_BID_PRICE},
        market_product_group::MarketProductGroup,
        risk_engine_register::*,
        trader_risk_group::TraderRiskGroup,
    },
    utils::{
        loadable::Loadable,
        numeric::{Fractional, ZERO_FRAC},
        validation::assert_keys_equal,
    },
};

declare_id!("ARiskEngine11111111111111111111111111111111");

const MINIMUM_HEALTH_THRESHOLD: Fractional = Fractional { m: 5, exp: 1 };
const LIQUIDATION_THRESHOLD: Fractional = Fractional { m: 2, exp: 1 };
const BETA: Fractional = Fractional { m: 2, exp: 1 };
const GAMMA: Fractional = Fractional { m: 1, exp: 1 };
const ALPHA: Fractional = Fractional { m: 9, exp: 1 };

#[program]
pub mod risk {
    use super::*;

    pub fn validate_account_health(ctx: Context<RiskAccounts>) -> ProgramResult {
        let (risk_signer_key, _) = Pubkey::find_program_address(
            &[ctx.accounts.market_product_group.key().as_ref()],
            &dex::ID,
        );
        assert_keys_equal(risk_signer_key, ctx.accounts.risk_signer.key())?;
        let account_health = compute_health(
            ctx.accounts.trader_risk_group.load()?.deref(),
            ctx.accounts.market_product_group.load()?.deref(),
        )?;
        let margin_req = account_health.margin_req;
        let portfolio_value = account_health.portfolio_value;
        let health_threshold = MINIMUM_HEALTH_THRESHOLD.checked_mul(margin_req)?;
        let liq_threshold = LIQUIDATION_THRESHOLD.checked_mul(margin_req)?;
        msg!("Portfolio value: {}", portfolio_value);
        msg!("Margin requirement: {}", margin_req);
        let mut out_register = RiskOutputRegister::load_mut(&ctx.accounts.out_register_risk_info)?;
        out_register.risk_engine_output = if portfolio_value >= health_threshold {
            HealthResult::Health {
                health_info: HealthInfo {
                    health: HealthStatus::Healthy,
                    action: ActionStatus::Approved,
                },
            }
        } else if (portfolio_value < health_threshold) && (portfolio_value >= liq_threshold) {
            HealthResult::Health {
                health_info: HealthInfo {
                    health: HealthStatus::Unhealthy,
                    action: ActionStatus::NotApproved,
                },
            }
        } else {
            HealthResult::Health {
                health_info: HealthInfo {
                    health: HealthStatus::Liquidatable,
                    action: ActionStatus::NotApproved,
                },
            }
        };
        Ok(())
    }

    pub fn validate_account_liquidation(ctx: Context<RiskAccounts>) -> ProgramResult {
        let (risk_signer_key, _) = Pubkey::find_program_address(
            &[ctx.accounts.market_product_group.key().as_ref()],
            &dex::ID,
        );
        assert_keys_equal(risk_signer_key, ctx.accounts.risk_signer.key())?;
        let trader_risk_group = ctx.accounts.trader_risk_group.load()?;
        let market_product_group = ctx.accounts.market_product_group.load()?;
        let account_health =
            compute_health(trader_risk_group.deref(), market_product_group.deref())?;
        let margin_req = account_health.margin_req;
        let portfolio_value = account_health.portfolio_value;
        msg!("Portfolio value: {}", portfolio_value);
        msg!("Margin requirement: {}", margin_req);
        let health_threshold = MINIMUM_HEALTH_THRESHOLD.checked_mul(margin_req)?;
        let liq_threshold = LIQUIDATION_THRESHOLD.checked_mul(margin_req)?;
        let liquidation_price = if portfolio_value.m >= 0 {
            portfolio_value * (ALPHA - BETA)
        } else {
            portfolio_value * (Fractional::from(1) - BETA)
        };
        let social_loss = if liquidation_price > ZERO_FRAC {
            liquidation_price * GAMMA
        } else {
            liquidation_price
        };

        if account_health.total_abs_dollar_position.m == 0 {
            return Err(ProgramError::InvalidAccountData);
        }

        let liquidation_info = get_liquidation_status(
            trader_risk_group.deref(),
            portfolio_value,
            liquidation_price,
            liq_threshold,
            health_threshold,
            social_loss,
            &account_health,
        )?;
        let mut out_register = RiskOutputRegister::load_mut(&ctx.accounts.out_register_risk_info)?;
        out_register.risk_engine_output = HealthResult::Liquidation {
            liquidation_info: liquidation_info,
        };
        Ok(())
    }

    pub fn create_risk_state_account(ctx: Context<RiskState>) -> ProgramResult {
        let (risk_signer_key, _) = Pubkey::find_program_address(
            &[ctx.accounts.market_product_group.key().as_ref()],
            &dex::ID,
        );
        assert_keys_equal(risk_signer_key, ctx.accounts.risk_signer.key())?;
        Ok(())
    }
}

fn get_liquidation_status(
    trader_risk_group: &TraderRiskGroup,
    portfolio_value: Fractional,
    liquidation_price: Fractional,
    liq_threshold: Fractional,
    health_threshold: Fractional,
    social_loss: Fractional,
    account_health: &Health,
) -> DomainOrProgramResult<LiquidationInfo> {
    let zero_social = SocialLoss {
        product_index: MAX_OUTRIGHTS,
        amount: ZERO_FRAC,
    };
    if portfolio_value <= liq_threshold {
        let mut liquidation_info = LiquidationInfo {
            health: HealthStatus::Liquidatable,
            action: ActionStatus::Approved,
            total_social_loss: social_loss,
            liquidation_price,
            social_losses: [zero_social; MAX_TRADER_POSITIONS],
        };
        for (i, position) in trader_risk_group.trader_positions.iter().enumerate() {
            if !position.is_initialized() {
                continue;
            }
            liquidation_info.social_losses[i] = SocialLoss {
                product_index: position.product_index,
                amount: social_loss
                    .checked_mul(account_health.abs_dollar_position[position.product_index])?
                    .checked_div(account_health.total_abs_dollar_position)?,
            };
        }
        Ok(liquidation_info)
    } else if (portfolio_value > liq_threshold) && (portfolio_value <= health_threshold) {
        Ok(LiquidationInfo {
            health: HealthStatus::Unhealthy,
            action: ActionStatus::NotApproved,
            total_social_loss: social_loss,
            liquidation_price,
            social_losses: [zero_social; MAX_TRADER_POSITIONS],
        })
    } else {
        Ok(LiquidationInfo {
            health: HealthStatus::Healthy,
            action: ActionStatus::NotApproved,
            total_social_loss: social_loss,
            liquidation_price,
            social_losses: [zero_social; MAX_TRADER_POSITIONS],
        })
    }
}

fn fetch_price(
    i: usize,
    market_product_group: &MarketProductGroup,
) -> std::result::Result<Fractional, DomainOrProgramError> {
    let (prev_ask, prev_bid, bid, ask) = (
        market_product_group.market_products[i].prices.prev_ask,
        market_product_group.market_products[i].prices.prev_bid,
        market_product_group.market_products[i].prices.bid,
        market_product_group.market_products[i].prices.ask,
    );

    let mark_price = match (prev_ask < NO_ASK_PRICE, prev_bid > NO_BID_PRICE) {
        (true, true) => {
            let sum_price = prev_ask + prev_bid;
            Fractional::new(sum_price.m * 5, sum_price.exp + 1)
        }
        (true, false) => prev_ask,
        (false, true) => prev_bid,
        (false, false) => match (ask < NO_ASK_PRICE, bid > NO_BID_PRICE) {
            (true, true) => {
                let sum_price = ask + bid;
                Fractional::new(sum_price.m * 5, sum_price.exp + 1)
            }
            (true, false) => ask,
            (false, true) => bid,
            (false, false) => ZERO_FRAC,
        },
    };
    Ok(mark_price)
}

fn compute_health(
    trader_risk_group: &TraderRiskGroup,
    market_product_group: &MarketProductGroup,
) -> std::result::Result<Health, ProgramError> {
    let mut margin_req = ZERO_FRAC;
    let mut open_combos: Vec<usize> = vec![];
    let mut abs_dollar_position: Vec<Fractional> = vec![ZERO_FRAC; MAX_OUTRIGHTS];

    let combo_indices: Vec<usize> = market_product_group
        .active_combos()
        .map(|(idx, _)| idx)
        .collect();

    for idx in combo_indices.iter() {
        let combo = trader_risk_group.open_orders.products[*idx];
        if combo.ask_qty_in_book + combo.bid_qty_in_book > ZERO_FRAC {
            open_combos.push(*idx);
        }
    }

    let mut trader_portfolio_value = trader_risk_group
        .cash_balance
        .checked_add(trader_risk_group.pending_cash_balance)?;

    let mut total_abs_dollar_position = ZERO_FRAC;

    for trader_position in trader_risk_group.trader_positions.iter() {
        if !trader_position.is_initialized() {
            continue;
        }
        let idx = trader_position.product_index;
        let price_i = fetch_price(idx, market_product_group)?;
        let size = trader_position
            .position
            .checked_add(trader_position.pending_position)?;
        let trader_position_value = price_i.checked_mul(size)?;
        abs_dollar_position[idx] = trader_position_value.abs();

        trader_portfolio_value = trader_portfolio_value.checked_add(trader_position_value)?;
        margin_req = margin_req.checked_add(abs_dollar_position[idx])?;
        total_abs_dollar_position =
            total_abs_dollar_position.checked_add(abs_dollar_position[idx])?;

        let outright_qty = trader_risk_group.open_orders.products[idx]
            .ask_qty_in_book
            .max(trader_risk_group.open_orders.products[idx].bid_qty_in_book);
        margin_req = margin_req.checked_add(outright_qty.checked_mul(price_i)?)?;
    }

    for &idx in open_combos.iter() {
        let price_i = fetch_price(idx, market_product_group)?;
        let combo_qty = trader_risk_group.open_orders.products[idx]
            .ask_qty_in_book
            .max(trader_risk_group.open_orders.products[idx].bid_qty_in_book);
        margin_req = margin_req.checked_add(combo_qty.checked_mul(price_i)?.abs())?;
    }

    Ok(Health {
        margin_req,
        abs_dollar_position,
        total_abs_dollar_position,
        portfolio_value: trader_portfolio_value,
    })
}

#[derive(Accounts)]
pub struct RiskAccounts<'info> {
    market_product_group: AccountLoader<'info, MarketProductGroup>,
    trader_risk_group: AccountLoader<'info, TraderRiskGroup>,
    out_register_risk_info: AccountInfo<'info>,
    _risk_state: AccountInfo<'info>,
    _risk_model_configuration: AccountInfo<'info>,
    risk_signer: Signer<'info>,
}

#[derive(Accounts)]
pub struct RiskState<'info> {
    #[account(mut)]
    payer: Signer<'info>,
    risk_signer: Signer<'info>,
    #[account(
        init,
        payer = payer,
        space = 0,
    )]
    risk_state: AccountInfo<'info>,
    market_product_group: AccountLoader<'info, MarketProductGroup>,
    system_program: Program<'info, System>,
}
#[account]
pub struct Health {
    pub margin_req: Fractional,
    pub portfolio_value: Fractional,
    pub total_abs_dollar_position: Fractional,
    pub abs_dollar_position: Vec<Fractional>,
}
