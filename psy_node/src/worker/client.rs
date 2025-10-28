use async_trait::async_trait;
use jsonrpsee::{
    core::RpcResult,
    http_client::{HttpClient, HttpClientBuilder},
    proc_macros::rpc,
};
use plonky2::field::goldilocks_field::GoldilocksField;
use psy_core::data::qhashout::QHashOut;
use tracing::{debug, error, info};

use crate::{
    common::retry::{RetryConfig, Retryable},
    worker::F,
};

#[rpc(server, client, namespace = "psy")]
pub trait CoordinatorRpc {
    #[method(name = "get_user_id")]
    async fn get_user_id(&self, public_key: String) -> RpcResult<u64>;
}

#[derive(Clone)]
pub struct WorkerCoordinatorClient {
    pub rpc_client: HttpClient,
    pub retry_config: RetryConfig,
}

impl WorkerCoordinatorClient {
    pub async fn new(rpc_url: &str) -> anyhow::Result<Self> {
        let rpc_client = HttpClientBuilder::default().build(rpc_url)?;
        Ok(Self {
            rpc_client,
            retry_config: RetryConfig::default(),
        })
    }

    pub async fn new_with_retry_config(rpc_url: &str, retry_config: RetryConfig) -> anyhow::Result<Self> {
        let rpc_client = HttpClientBuilder::default().build(rpc_url)?;
        Ok(Self { rpc_client, retry_config })
    }

    pub async fn get_user_id(&self, public_key: &QHashOut<F>) -> anyhow::Result<u64> {
        let pk_string = public_key.to_string();
        debug!("Requesting user ID for public key: {}", pk_string);

        let user_id = self
            .retry_with_backoff(&format!("get_user_id for {}", pk_string), || async {
                CoordinatorRpcClient::get_user_id(&self.rpc_client, pk_string.clone()).await
            })
            .await
            .map_err(|e| {
                error!("❌ Failed to get user_id after all retries - public key: {}", pk_string);
                anyhow::anyhow!("User not whitelisted or coordinator unreachable: {}", e)
            })?;

        info!("✅ Retrieved user_id: {} for public key: {}", user_id, pk_string);
        Ok(user_id)
    }
}

impl Retryable for WorkerCoordinatorClient {
    fn retry_config(&self) -> RetryConfig {
        self.retry_config.clone()
    }
}
