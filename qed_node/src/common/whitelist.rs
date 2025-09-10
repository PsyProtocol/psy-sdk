use alloy_primitives::Address;
use anyhow::Result;
use indexmap::IndexSet;
use plonky2::field::goldilocks_field::GoldilocksField;
use qed_core::data::qhashout::QHashOut;
use qed_prover::wallet::secp_sign::{Eip712Signable, SignedRequest};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::info;

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct WhitelistConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub secp256k1: Vec<Address>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub whitelist: WhitelistConfig,
}

#[derive(Clone, Debug)]
pub struct WhiteList {
    pub enabled: bool,
    pub secp256k1: IndexSet<Address>,
}

impl WhiteList {
    pub fn new() -> Self {
        Self {
            enabled: false,
            secp256k1: IndexSet::new(),
        }
    }

    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let contents = fs::read_to_string(&path)?;
        let config: Config = serde_json::from_str(&contents)?;

        let mut whitelist = Self::new();
        whitelist.enabled = config.whitelist.enabled;

        for address in config.whitelist.secp256k1 {
            whitelist.secp256k1.insert(address);
            info!("Loaded secp256k1 address: {}", address);
        }

        info!(
            "Loaded {} secp256k1 addresses (enabled: {})",
            whitelist.secp256k1.len(),
            whitelist.enabled
        );
        Ok(whitelist)
    }

    pub fn is_secp256k1_whitelisted(&self, address: &Address) -> bool {
        self.secp256k1.contains(address)
    }

    pub fn verify_request<T>(
        &self,
        request: &SignedRequest<QHashOut<GoldilocksField>>,
        original_data: &T,
        expiry_duration: Option<std::time::Duration>,
    ) -> Result<()>
    where
        T: serde::Serialize,
    {
        if !self.enabled {
            return Ok(());
        }

        if !self.is_secp256k1_whitelisted(&request.address) {
            return Err(anyhow::anyhow!(
                "Address not whitelisted: {}",
                request.address
            ));
        }

        let is_valid = request.verify_hashable(original_data, request.address, expiry_duration)?;
        if !is_valid {
            return Err(anyhow::anyhow!("Invalid signature or expired"));
        }

        Ok(())
    }
}
