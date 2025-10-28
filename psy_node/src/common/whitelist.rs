use std::{
    fs,
    path::Path,
    sync::{Arc, RwLock},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use alloy_primitives::Address;
use anyhow::Result;
use indexmap::IndexSet;
use plonky2::field::goldilocks_field::GoldilocksField;
use psy_core::data::qhashout::QHashOut;
use psy_rust_sdk::wallet::secp_sign::{Eip712Signable, SignedRequest};
use serde::{Deserialize, Serialize};
use tokio::task::JoinHandle;
use tracing::{error, info};

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
            return Err(anyhow::anyhow!("Address not whitelisted: {}", request.address));
        }

        let is_valid = request.verify_hashable(original_data, request.address, expiry_duration)?;
        if !is_valid {
            return Err(anyhow::anyhow!("Invalid signature or expired"));
        }

        Ok(())
    }
}

#[derive(Clone)]
pub struct WhiteListCache {
    config_path: String,
    whitelist: Arc<RwLock<WhiteList>>,
    _reload_task: Arc<JoinHandle<()>>,
}

impl WhiteListCache {
    pub fn new<P: AsRef<Path>>(config_path: P) -> Result<Self> {
        let path_str = config_path.as_ref().to_string_lossy().to_string();
        let initial_whitelist = WhiteList::from_file(&config_path)?;
        let whitelist = Arc::new(RwLock::new(initial_whitelist));

        let whitelist_clone = Arc::clone(&whitelist);
        let path_clone = path_str.clone();

        let reload_task = tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(30));
            loop {
                interval.tick().await;

                match WhiteList::from_file(&path_clone) {
                    Ok(new_whitelist) => {
                        if let Ok(mut cache) = whitelist_clone.write() {
                            *cache = new_whitelist;
                            info!("Successfully reloaded whitelist from config");
                        } else {
                            error!("Failed to acquire write lock for whitelist cache");
                        }
                    }
                    Err(e) => {
                        error!("Failed to reload whitelist from config: {}", e);
                    }
                }
            }
        });

        Ok(Self {
            config_path: path_str,
            whitelist,
            _reload_task: Arc::new(reload_task),
        })
    }

    pub fn get_whitelist(&self) -> Arc<RwLock<WhiteList>> {
        Arc::clone(&self.whitelist)
    }

    pub fn verify_request<T>(
        &self,
        request: &SignedRequest<QHashOut<GoldilocksField>>,
        original_data: &T,
        expiry_duration: Option<Duration>,
    ) -> Result<()>
    where
        T: serde::Serialize,
    {
        let whitelist = self.whitelist.read().map_err(|_| anyhow::anyhow!("Failed to acquire read lock"))?;
        whitelist.verify_request(request, original_data, expiry_duration)
    }
}
