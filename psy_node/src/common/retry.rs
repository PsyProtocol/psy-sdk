use std::{fmt::Debug, future::Future, time::Duration};

use anyhow::anyhow;
use tracing::{debug, error};

/// Configuration for retry mechanisms
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

/// Public standalone retry function for any async operation
pub async fn retry_with_backoff<T, F, Fut, E>(config: &RetryConfig, operation_name: &str, mut operation: F) -> anyhow::Result<T>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, E>>,
    E: Debug,
{
    for attempt in 0..config.max_retries {
        match operation().await {
            Ok(result) => {
                if attempt > 0 {
                    debug!("{} succeeded on attempt {}", operation_name, attempt + 1);
                }
                return Ok(result);
            }
            Err(err) => {
                error!("{} failed: {:?}, attempt {}/{}", operation_name, err, attempt + 1, config.max_retries);

                if attempt < config.max_retries - 1 {
                    let delay = if config.exponential_backoff {
                        Duration::from_millis(config.base_delay_ms * 2_u64.pow(attempt))
                    } else {
                        Duration::from_millis(config.base_delay_ms)
                    };

                    debug!("Retrying {} after {:?}", operation_name, delay);
                    tokio::time::sleep(delay).await;
                }
            }
        }
    }

    Err(anyhow!("{} failed after {} attempts", operation_name, config.max_retries))
}

// Optional: Keep the trait if you want backwards compatibility or might use it
// elsewhere
pub trait Retryable {
    fn retry_config(&self) -> RetryConfig {
        RetryConfig::default()
    }

    async fn retry_with_backoff<T, F, Fut, E>(&self, operation_name: &str, mut operation: F) -> anyhow::Result<T>
    where
        F: FnMut() -> Fut,
        Fut: Future<Output = Result<T, E>>,
        E: Debug,
    {
        retry_with_backoff(&self.retry_config(), operation_name, operation).await
    }
}
