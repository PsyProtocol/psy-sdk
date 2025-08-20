use std::time::Duration;
use anyhow::anyhow;
use tracing::error;
use qed_data::config::store_config::QEDHasher;

pub type C = plonky2::plonk::config::PoseidonGoldilocksConfig;
pub const D: usize = 2;
pub type F = qed_data::config::store_config::QEDFelt;

pub type H = QEDHasher;

pub mod config;
pub mod edge;
pub mod processor;
pub mod state;

pub use config::*;
pub use edge::*;
pub use processor::*;


/// Configuration for retry mechanisms in RealmProofSender
#[derive(Clone, Debug)]
pub struct RetryConfig {
    pub max_retries: u32,
    pub base_delay_ms: u64,
    pub exponential_backoff: bool,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 5,
            base_delay_ms: 1000,
            exponential_backoff: true,
        }
    }
}


trait Retryable {
    fn retry_config(&self) -> &RetryConfig;

    /// Generic retry function for any async operation
    async fn retry_with_backoff<T, F, Fut, E>(&self, operation_name: &str, mut operation: F) -> anyhow::Result<T>
    where
        F: FnMut() -> Fut,
        Fut: std::future::Future<Output =std::result::Result<T, E>>,
        E: std::fmt::Debug,
    {
        for attempt in 0..self.retry_config().max_retries {
            match operation().await {
                Ok(result) => return Ok(result),
                Err(err) => {
                    error!("{} failed: {:?}, attempt {}/{}", operation_name, err, attempt + 1, self.retry_config().max_retries);

                    if attempt < self.retry_config().max_retries - 1 {
                        let delay = if self.retry_config().exponential_backoff {
                            Duration::from_millis(self.retry_config().base_delay_ms * 2_u64.pow(attempt))
                        } else {
                            Duration::from_millis(self.retry_config().base_delay_ms)
                        };
                        tokio::time::sleep(delay).await;
                    }
                }
            }
        }

        Err(anyhow!("{} failed after {} attempts", operation_name, self.retry_config().max_retries))
    }
}

