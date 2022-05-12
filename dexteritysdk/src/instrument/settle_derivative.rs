use crate::{common::utils::*, sdk_client::SDKClient};
use anchor_lang::{InstructionData, ToAccountMetas};
use dex::utils::numeric::Fractional;
use instruments::{
    accounts,
    state::enums::{InstrumentType, OracleType},
};
use rand::Rng;
use solana_program::{
    clock::UnixTimestamp,
    instruction::Instruction,
    pubkey::Pubkey,
    sysvar::{clock::Clock, rent::Rent, Sysvar},
};
use solana_program_test::ProgramTestContext;
use solana_sdk::signature::{Keypair, Signer};

pub fn settle_derivative_ixs(
    price_oracle: Pubkey,
    market_product_group: Pubkey,
    clock: Pubkey,
    derivative_metadata: Pubkey,
) -> Vec<Instruction> {
    let account_metas = instruments::accounts::SettleDerivative {
        market_product_group,
        derivative_metadata,
        price_oracle,
        dex_program: dex::id(),
        clock,
    }
    .to_account_metas(Some(true));

    let mut data = instruments::instruction::SettleDerivative.data();
    // Hack to get back test runtime dedupe
    let mut rng = rand::prelude::thread_rng();
    let out = rng.gen_range(0..255);
    data.push(out as u8);
    let instruction = Instruction {
        program_id: instruments::ID,
        data: data,
        accounts: account_metas,
    };

    vec![instruction]
}

pub async fn settle_derivative(
    client: &SDKClient,
    market_product_group: Pubkey,
    price_oracle: Pubkey,
    clock: Pubkey,
    derivative_metadata: Pubkey,
) -> std::result::Result<Pubkey, SDKError> {
    let ixs = settle_derivative_ixs(
        price_oracle,
        market_product_group,
        clock,
        derivative_metadata,
    );
    client.sign_send_instructions(ixs, vec![]).await?;
    Ok(derivative_metadata)
}
