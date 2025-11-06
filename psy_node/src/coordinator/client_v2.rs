use async_trait::async_trait;
use jsonrpsee::{
    http_client::{HttpClient, HttpClientBuilder},
    proc_macros::rpc,
};
use plonky2::plonk::config::PoseidonGoldilocksConfig;
use psy_data::{config::store_config::PsyFelt, guta::api::SubmitGUTARealmResultAPINoProofInput, qdata::checkpoint::CheckpointSyncInfo};
use tracing::{error, info, trace};

use crate::{
    common::retry::{RetryConfig, Retryable},
    common_v2::traits::realm::*,
    coordinator::edge::rpc::CoordinatorEdgeRpcClient,
};

type F = PsyFelt;
type C = PoseidonGoldilocksConfig;
const D: usize = 2;

#[derive(Debug, Clone)]
pub struct ConcreteCoordinatorClient {
    pub rpc_client: HttpClient,
}

impl ConcreteCoordinatorClient {
    pub fn new(rpc_url: String) -> anyhow::Result<Self> {
        let rpc_client = HttpClientBuilder::default().build(&rpc_url)?;
        Ok(Self { rpc_client })
    }
}

impl Retryable for ConcreteCoordinatorClient {}

#[async_trait]
impl CoordinatorClient<F> for ConcreteCoordinatorClient {
    async fn get_current_checkpoint_id(&self) -> anyhow::Result<u64> {
        self.retry_with_backoff("get_current_checkpoint_id", || async {
            self.rpc_client.get_current_checkpoint_id().await
        })
        .await
    }
    async fn get_current_realm_status_on_coordinator(&self, realm_id: u64) -> anyhow::Result<BasicRealmStatusOnCoordinator<F>> {
        self.retry_with_backoff("get_current_realm_status_on_coordinator", || async {
            self.rpc_client.get_current_realm_status_on_coordinator(realm_id).await
        })
        .await
    }

    async fn wait_until_coordinator_completed(&self, realm_id: u64, checkpoint_id: u64) -> anyhow::Result<GlobalBlockUpdateFromCoordinator<F>> {
        self.retry_with_backoff("wait_until_coordinator_completed", || async {
            self.rpc_client.wait_until_coordinator_completed(realm_id, checkpoint_id).await
        })
        .await
    }

    async fn get_latest_block_updates_from_coordinator(
        &self,
        realm_id: u64,
        from_checkpoint: u64,
        to_checkpoint: u64,
    ) -> anyhow::Result<Vec<GlobalBlockUpdateFromCoordinator<F>>> {
        self.retry_with_backoff("get_latest_block_updates_from_coordinator", || async {
            self.rpc_client
                .get_latest_block_updates_from_coordinator(realm_id as u32, from_checkpoint, to_checkpoint)
                .await
        })
        .await
    }

    async fn submit_realm_result(&self, realm_result: &RealmDataForCoordinator<F>) -> anyhow::Result<()> {
        self.retry_with_backoff("submit_realm_result", || async {
            match self.rpc_client.submit_realm_result(realm_result.clone()).await {
                Ok(_) => {
                    trace!("Successfully submitted job to coordinator");
                    Ok(())
                }
                Err(err) => {
                    error!("Failed to submit job to coordinator: {:?}", err);
                    if err.to_string().contains("ServerError") {
                        return Ok(());
                    }
                    Err(err)
                }
            }
        })
        .await
    }

    async fn get_checkpoint_sync_info(&self, realm_id: u32, checkpoint_id: u64) -> anyhow::Result<CheckpointSyncInfo<F>> {
        self.rpc_client
            .get_checkpoint_sync_info(realm_id, checkpoint_id)
            .await
            .map_err(|e| anyhow::anyhow!(e))
    }

    async fn submit_guta_v1(&self, input: &SubmitGUTARealmResultAPINoProofInput<F>, proof: &[u8], realm_id: u64) -> anyhow::Result<()> {
        self.retry_with_backoff("submit_guta_v1", || async {
            match self.rpc_client.submit_guta_v1(input.clone(), proof.to_vec(), realm_id).await {
                Ok(_) => {
                    trace!("Successfully submitted job to coordinator");
                    Ok(())
                }
                Err(err) => {
                    error!("Failed to submit job to coordinator: {:?}", err);
                    if err.to_string().contains("ServerError") {
                        return Ok(());
                    }
                    Err(err)
                }
            }
        })
        .await
    }
    async fn has_pending_guta(&self, realm_id: u32) -> anyhow::Result<bool> {
        self.retry_with_backoff("has_pending_guta", || async { self.rpc_client.has_pending_guta(realm_id).await })
            .await
    }

    async fn get_latest_checkpoint_sync_info(&self, realm_id: u32) -> anyhow::Result<CheckpointSyncInfo<F>> {
        self.rpc_client
            .get_latest_checkpoint_sync_info(realm_id)
            .await
            .map_err(|e| anyhow::anyhow!(e))
    }
}
