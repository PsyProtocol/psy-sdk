use async_trait::async_trait;
use qed_core::job::{
    id::{QProvingJobDataID, ProvingJobDataType},
    traits::QProofStoreReaderAsync,
};
use plonky2::plonk::{config::GenericConfig, proof::ProofWithPublicInputs};
use crate::common::retry::{RetryConfig, Retryable};
use std::sync::Arc;
use tracing::{debug, error, warn};

/// Wrapper around QProofStoreReaderAsync that adds retry logic and validates non-empty data
pub struct RetryableStore<S: QProofStoreReaderAsync> {
    inner_store: Arc<S>,
    retry_config: RetryConfig,
}

impl<S: QProofStoreReaderAsync> RetryableStore<S> {
    pub fn new(store: Arc<S>) -> Self {
        Self {
            inner_store: store,
            retry_config: RetryConfig {
                max_retries: 10,  // More retries for RPC
                base_delay_ms: 500,
                exponential_backoff: true,
            },
        }
    }

    pub fn with_retry_config(store: Arc<S>, retry_config: RetryConfig) -> Self {
        Self {
            inner_store: store,
            retry_config,
        }
    }

    /// Validate that data is not empty and has minimum expected size
    fn validate_data(data: &[u8], id: &str, min_size: usize) -> anyhow::Result<()> {
        if data.is_empty() {
            warn!("Retrieved empty data for ID: {}", id);
            Err(anyhow::anyhow!("Empty data returned from RPC"))
        } else if data.len() < min_size {
            warn!("Retrieved suspiciously small data ({} bytes) for ID: {}", data.len(), id);
            Err(anyhow::anyhow!("Incomplete data returned from RPC (only {} bytes)", data.len()))
        } else {
            debug!("Retrieved {} bytes for ID: {}", data.len(), id);
            Ok(())
        }
    }
}

impl<S: QProofStoreReaderAsync> Retryable for RetryableStore<S> {
    fn retry_config(&self) -> RetryConfig {
        self.retry_config.clone()
    }
}

#[async_trait]
impl<S: QProofStoreReaderAsync + Send + Sync> QProofStoreReaderAsync for RetryableStore<S> {
    async fn contains_id(&self, id: QProvingJobDataID) -> anyhow::Result<bool> {
        self.retry_with_backoff(
            &format!("contains_id for {}", id.to_hex_string()),
            || async {
                self.inner_store.contains_id(id).await
            }
        ).await
    }

    async fn get_bytes_by_id(&self, id: QProvingJobDataID) -> anyhow::Result<Vec<u8>> {
        let id_hex = id.to_hex_string();
        debug!("Fetching bytes for ID: {}", id_hex);

        self.retry_with_backoff(
            &format!("get_bytes_by_id for {}", id_hex),
            || async {
                let data = self.inner_store.get_bytes_by_id(id).await?;
                Self::validate_data(&data, &id_hex, 4)?;
                Ok::<Vec<u8>, anyhow::Error>(data)  // Explicitly specify the error type
            }
        ).await
            .map_err(|e| {
                error!("❌ Failed to fetch valid data after all retries - ID: {}", id_hex);
                anyhow::anyhow!("Failed to retrieve valid data: {}", e)
            })
    }

    async fn get_proof_by_id<C: GenericConfig<D>, const D: usize>(
        &self,
        id: QProvingJobDataID
    ) -> anyhow::Result<ProofWithPublicInputs<C::F, C, D>> {
        let id_hex = id.to_hex_string();
        debug!("Fetching proof for ID: {}", id_hex);

        self.retry_with_backoff(
            &format!("get_proof_by_id for {}", id_hex),
            || async {
                let proof_bytes = self.inner_store.get_bytes_by_id(id).await?;
                Self::validate_data(&proof_bytes, &id_hex, 4)?;

                // Try to deserialize to validate it's valid proof data
                let proof: ProofWithPublicInputs<C::F, C, D> =
                    bincode::deserialize(&proof_bytes)
                        .map_err(|e| anyhow::anyhow!("Invalid proof data: {}", e))?;
                Ok::<ProofWithPublicInputs<C::F, C, D>, anyhow::Error>(proof)  // Explicitly specify the error type
            }
        ).await
            .map_err(|e| {
                error!("❌ Failed to fetch valid proof after all retries - ID: {}", id_hex);
                anyhow::anyhow!("Failed to retrieve valid proof: {}", e)
            })
    }

    async fn get_public_input_by_id<C: GenericConfig<D>, const D: usize>(
        &self,
        id: QProvingJobDataID,
    ) -> anyhow::Result<Vec<C::F>> {
        let proof = self.get_proof_by_id::<C, D>(id).await?;
        Ok(proof.public_inputs)
    }
}