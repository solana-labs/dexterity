use std::{
    fmt::{Debug, Formatter},
    ops::{Deref, DerefMut},
    path::Display,
};

use anyhow::anyhow;
use bincode::Options;
use derive_more::{Deref, DerefMut, From};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use solana_sdk::signature::{read_keypair_file, Keypair};

use instruments::state::derivative_metadata::DerivativeMetadata;

use crate::{SDKError, SDKResult};

pub mod utils;

pub fn get_local_payer() -> SDKResult<KeypairD> {
    let keypair = read_keypair_file(&*shellexpand::tilde("~/.config/solana/id.json"))?;
    Ok(keypair.into())
}

#[derive(From, Clone, Copy, PartialEq, Debug, Deref, DerefMut)]
pub struct Side {
    inner: agnostic_orderbook::state::Side,
}

impl Serialize for Side {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u8(self.inner as u8)
    }
}

impl<'de> Deserialize<'de> for Side {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        use num_traits::cast::FromPrimitive;
        let int: u8 = Deserialize::deserialize(deserializer)?;
        let side = agnostic_orderbook::state::Side::from_u8(int).unwrap();
        Ok(Side { inner: side })
    }
}

pub struct KeypairD(pub Keypair);

impl KeypairD {
    pub fn new() -> KeypairD {
        KeypairD(Keypair::new())
    }
}

impl Serialize for KeypairD {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.0.to_bytes().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for KeypairD {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let bytes: [[u8; 32]; 2] = Deserialize::deserialize(deserializer)?;
        let all: [u8; 64] = unsafe { std::mem::transmute(bytes) };
        Ok(Self(Keypair::from_bytes(&all).unwrap())) // todo use proper error
    }
}

pub fn clone_keypair(keypair: &Keypair) -> Keypair {
    Keypair::from_bytes(&keypair.to_bytes()).unwrap()
}

impl Clone for KeypairD {
    fn clone(&self) -> Self {
        Self(clone_keypair(&self.0))
    }
}

impl std::str::FromStr for KeypairD {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        Ok(Self(Keypair::from_base58_string(s)))
    }
}

impl std::fmt::Display for KeypairD {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", &self.0.to_base58_string())
    }
}

impl Debug for KeypairD {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", &self.0.to_base58_string())
    }
}

impl From<&Keypair> for KeypairD {
    fn from(x: &Keypair) -> Self {
        Self(clone_keypair(x))
    }
}

impl From<Keypair> for KeypairD {
    fn from(x: Keypair) -> Self {
        Self(x)
    }
}

impl Deref for KeypairD {
    type Target = Keypair;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for KeypairD {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
