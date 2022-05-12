use solana_program::pubkey::Pubkey;
use solana_sdk::{signature::Keypair, signer::Signer};

use dex::utils::numeric::Fractional;

use crate::{common::utils::SDKError, KeypairD, SDKClient};

pub mod initialize_derivative;
pub mod settle_derivative;

pub struct InstrumentAdmin {
    pub client: SDKClient,
    pub authority: KeypairD,
    pub market_product_group: Pubkey,
    pub payer: KeypairD,
}

impl InstrumentAdmin {
    pub fn new(
        client: SDKClient,
        authority: KeypairD,
        market_product_group: Pubkey,
        payer: KeypairD,
    ) -> Self {
        Self {
            client,
            authority,
            market_product_group,
            payer,
        }
    }

    pub async fn initialize_derivative(
        &self,
        price_oracle: Pubkey,
        clock: Pubkey,
        strike: impl Into<Fractional>,
        optional_args: initialize_derivative::InitializeDerivativeOptionalArgs,
    ) -> std::result::Result<Pubkey, SDKError> {
        let instrument_type = optional_args.instrument_type;
        let initialization_time = optional_args.initialization_time;
        let full_funding_period = optional_args.full_funding_period;
        let minimum_funding_period = optional_args.minimum_funding_period;
        let oracle_type = optional_args.oracle_type;
        let strike = strike.into();

        let derivative_metadata = initialize_derivative::get_derivative_key(
            price_oracle,
            self.market_product_group,
            instrument_type,
            strike,
            full_funding_period as u64,
            minimum_funding_period,
            initialization_time,
        );

        let ixs = initialize_derivative::initialize_derivative_ixs(
            self.authority.pubkey(),
            price_oracle,
            self.market_product_group,
            self.payer.pubkey(),
            clock,
            derivative_metadata,
            instrument_type,
            strike,
            full_funding_period,
            minimum_funding_period,
            initialization_time,
            oracle_type,
        );
        self.client
            .sign_send_instructions(ixs, vec![&self.payer])
            .await?;
        Ok(derivative_metadata)
    }
}
