use anchor_lang::prelude::*;
use dex::{
    state::{
        constants::MAX_TRADER_POSITIONS, market_product_group::MarketProductGroup,
        risk_engine_register::*, trader_risk_group::TraderRiskGroup,
    },
    utils::{loadable::Loadable, numeric::ZERO_FRAC, validation::assert_keys_equal},
};
use solana_program::{
    account_info::AccountInfo, declare_id, entrypoint::ProgramResult, pubkey::Pubkey,
};

declare_id!("Noop111111111111111111111111111111111111111");

#[program]
pub mod risk {
    use super::*;

    pub fn validate_account_health(ctx: Context<RiskAccounts>) -> ProgramResult {
        let (risk_signer_key, _) = Pubkey::find_program_address(
            &[ctx.accounts.market_product_group.key().as_ref()],
            &dex::ID,
        );
        assert_keys_equal(risk_signer_key, ctx.accounts.risk_signer.key())?;
        let data = &mut ctx.accounts.out_register_risk_info.try_borrow_mut_data()?
            [..std::mem::size_of::<RiskOutputRegister>()];
        let out_register_risk_info = RiskOutputRegister::load_from_bytes_mut(data)?;
        out_register_risk_info.risk_engine_output = HealthResult::Health {
            health_info: HealthInfo {
                health: HealthStatus::Healthy,
                action: ActionStatus::Approved,
            },
        };
        Ok(())
    }

    pub fn validate_account_liquidation(ctx: Context<RiskAccounts>) -> ProgramResult {
        let (risk_signer_key, _) = Pubkey::find_program_address(
            &[ctx.accounts.market_product_group.key().as_ref()],
            &dex::ID,
        );
        assert_keys_equal(risk_signer_key, ctx.accounts.risk_signer.key())?;
        let data = &mut ctx.accounts.out_register_risk_info.try_borrow_mut_data()?
            [..std::mem::size_of::<RiskOutputRegister>()];
        let out_register_risk_info = RiskOutputRegister::load_from_bytes_mut(data)?;
        let zero_social = SocialLoss {
            product_index: 0,
            amount: ZERO_FRAC,
        };
        out_register_risk_info.risk_engine_output = HealthResult::Liquidation {
            liquidation_info: LiquidationInfo {
                health: HealthStatus::Healthy,
                action: ActionStatus::Approved,
                total_social_loss: ZERO_FRAC,
                liquidation_price: ZERO_FRAC,
                social_losses: [zero_social; MAX_TRADER_POSITIONS],
            },
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

#[derive(Accounts)]
pub struct RiskAccounts<'info> {
    market_product_group: AccountLoader<'info, MarketProductGroup>,
    trader_risk_group: AccountLoader<'info, TraderRiskGroup>,
    out_register_risk_info: AccountInfo<'info>,
    _risk_state_account_info: AccountInfo<'info>,
    _risk_model_configuration_acct: AccountInfo<'info>,
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
