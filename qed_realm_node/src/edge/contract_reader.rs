use std::sync::Arc;

use super::error::{Result, RpcError};
use qed_core::data::qhashout::QHashOut;
use qed_data::guta::api::SimpleContractHeightCache;
use qed_store::{config::store_config::QEDFelt, node::realm::QEDRealmStoreReaderAsync};

/// Contract height provider struct
#[derive(Clone, Debug)]
pub struct ContractReader<SR: QEDRealmStoreReaderAsync<QEDFelt>> {
    pub store_reader: Arc<SR>,
    pub contract_cache: SimpleContractHeightCache<QEDFelt>,
}

impl<SR: QEDRealmStoreReaderAsync<QEDFelt>> ContractReader<SR> {
    /// Create a new ContractReader instance
    pub fn new(store_reader: Arc<SR>) -> Self {
        Self {
            store_reader,
            contract_cache: SimpleContractHeightCache::<QEDFelt>::new(),
        }
    }

    /// Get the height of a specified contract ID
    pub async fn get_contract_height(&mut self, _contract_id: u64) -> Result<u8> {
        panic!("Not implemented");
    }

    /// Get zero hash for the specified contract
    pub async fn get_contract_zero_hash(&mut self, contract_id: u64) -> Result<QHashOut<QEDFelt>> {
        // First try to get from cache
        match self.contract_cache.get_contract_zero_hash(contract_id) {
            Ok(zero_hash) => Ok(zero_hash),
            Err(_) => {
                // Get the contract height, this will also cache zero hash
                self.get_contract_height(contract_id).await?;

                // Now it should be available in the cache
                Ok(self.contract_cache.get_contract_zero_hash(contract_id)?)
            }
        }
    }
}
