use super::error::RpcError;
use super::rpc::RealmEdgeRpcServer;
use crate::common::jobs::{JobSchedulerRpcServer};
use crate::common::ConcreteProofWithPublicInputs;
use crate::realm::state::edge::RealmEdgeContext;
use crate::realm::{C, D, F};
use async_trait::async_trait;
use jsonrpsee::core::{client::ClientT, RpcResult};
use jsonrpsee::http_client::{HeaderMap, HeaderValue, HttpClient, HttpClientBuilder};
use jsonrpsee::rpc_params;
use plonky2::plonk::config::PoseidonGoldilocksConfig;
use plonky2::{field::types::PrimeField64, plonk::proof::ProofWithPublicInputs};
use qed_core::config::network_constants::REALM_PROOF_SYNC_CHANNEL;
use qed_core::job::history_queue::CheckpointHistoryQueueConsumerAsyncImm;
use qed_core::job::id::ProvingJobDataId;
use qed_core::job::worker_queue::WorkerEventReceiverAsyncImm;
use qed_core::{
    data::qhashout::QHashOut,
    job::{
        drain_queue::CheckpointDrainQueueEmitterAsyncImm,
        id::{ProvingJobCircuitType, QJobTopic, QProvingJobDataID},
        traits::QProofStoreAsyncImm,
    },
};
use qed_crypto::hash::merkle::core::MerkleProofCore;
use qed_data::config::store_config::QEDFelt;
use qed_data::guta::api::{GUTARealmCheckpointResult, SubmitGUTARealmResultAPINoProofInput};
use qed_data::guta::end_cap_input::SubmitUserEndCapNonProofInput;
use qed_data::qdata::checkpoint::{
    QEDCheckpointGlobalStateRoots, QEDCheckpointLeaf, QEDL2BlockState,
};
use qed_data::qdata::user::QEDUserLeaf;
use qed_rollup_utils::generate_jwt_token;
use qed_store::node::realm::QEDRealmStoreReaderAsync;
use qed_store::queue::ProofStoreRedisAsync;
use std::env;
use std::sync::Arc;
use std::time::Duration;
use anyhow::anyhow;
use jsonrpsee::types::{ErrorCode, ErrorObject};

use tracing::{debug, error, info, warn};
use qed_store::queue::task_queue::{JobTaskStore, JobTaskStoreImpl, JobValidationStatus, QJob};

#[derive(Clone)]
pub struct RealmEdgeHandler<
    SR: QEDRealmStoreReaderAsync<F> + Sync,
    DQ: CheckpointDrainQueueEmitterAsyncImm,
    PS: QProofStoreAsyncImm,
> {
    ctx: RealmEdgeContext<SR, DQ, PS>,
    job_notify_queue: Arc<ProofStoreRedisAsync>,
    job_task_store: Arc<JobTaskStoreImpl>,

}

impl<SR, DQ, PS> RealmEdgeHandler<SR, DQ, PS>
where
    SR: QEDRealmStoreReaderAsync<F> + Sync,
    DQ: CheckpointDrainQueueEmitterAsyncImm,
    PS: QProofStoreAsyncImm,
{
    pub fn new(
        ctx: RealmEdgeContext<SR, DQ, PS>,
        job_notify_queue: Arc<ProofStoreRedisAsync>,
        job_task_store: Arc<JobTaskStoreImpl>,
    ) -> Self {
        Self {
            ctx,
            job_notify_queue,
            job_task_store,
        }
    }
    async fn log_suspicious_activity(&self, job: &QJob, reason: &str) {
        //todo! add some operation to log suspicious activity or ban user
        error!(
            "🚨 SECURITY ALERT: Invalid job submission - Reason: {}, Job: {:?}, Layer: {}, MsgId: {}",
            reason, job.job_id, job.layer_id, job.msg_id
        );
    }
}

#[async_trait]
impl<SR, DQ, PS> RealmEdgeRpcServer for RealmEdgeHandler<SR, DQ, PS>
where
    SR: QEDRealmStoreReaderAsync<F> + Sync + Send + 'static,
    DQ: CheckpointDrainQueueEmitterAsyncImm + Sync + Send + 'static,
    PS: QProofStoreAsyncImm + Sync + Send + 'static,
{
    async fn check_user_id_in_realm(&self, user_id: u64) -> RpcResult<bool> {
        Ok(self.ctx.includes_user_id(user_id))
    }

    async fn submit_user_end_cap(
        &self,
        user_ec_input: SubmitUserEndCapNonProofInput<F>,
        proof: ProofWithPublicInputs<F, C, D>,
    ) -> RpcResult<String> {
        Ok(self
            .ctx
            .handle_recv_end_cap_from_user(user_ec_input, &proof)
            .await
            .map(|_| "ok".to_string())
            .map_err(RpcError::Anyhow)?)
    }

    async fn get_checkpoint_leaf_data(
        &self,
        checkpoint_id: u64,
    ) -> RpcResult<QEDCheckpointLeaf<F>> {
        Ok(self
            .ctx
            .store_reader
            .get_checkpoint_leaf_data(checkpoint_id)
            .await
            .map_err(RpcError::Anyhow)?)
    }

    async fn get_checkpoint_leaf_data_f(
        &self,
        checkpoint_id: F,
    ) -> RpcResult<QEDCheckpointLeaf<F>> {
        Ok(self
            .ctx
            .store_reader
            .get_checkpoint_leaf_data(checkpoint_id.to_canonical_u64())
            .await
            .map_err(RpcError::Anyhow)?)
    }

    async fn get_latest_l2_block_state(&self) -> RpcResult<QEDL2BlockState> {
        Ok(self
            .ctx
            .store_reader
            .get_latest_l2_block_state()
            .await
            .map_err(RpcError::Anyhow)?)
    }

    async fn get_l2_block_state(&self, checkpoint_id: u64) -> RpcResult<QEDL2BlockState> {
        Ok(self
            .ctx
            .store_reader
            .get_l2_block_state(checkpoint_id)
            .await
            .map_err(RpcError::Anyhow)?)
    }

    async fn get_l2_block_state_f(&self, checkpoint_id: F) -> RpcResult<QEDL2BlockState> {
        Ok(self
            .ctx
            .store_reader
            .get_l2_block_state(checkpoint_id.to_canonical_u64())
            .await
            .map_err(RpcError::Anyhow)?)
    }

    async fn get_user_registration_tree_root(&self, checkpoint_id: u64) -> RpcResult<QHashOut<F>> {
        Ok(self
            .ctx
            .store_reader
            .get_user_registration_tree_root(checkpoint_id)
            .await
            .map_err(RpcError::Anyhow)?)
    }

    async fn get_latest_checkpoint_tree_root(&self) -> RpcResult<QHashOut<F>> {
        Ok(self
            .ctx
            .store_reader
            .get_latest_checkpoint_tree_root()
            .await
            .map_err(RpcError::Anyhow)?)
    }

    async fn get_checkpoint_tree_root(&self, checkpoint_id: u64) -> RpcResult<QHashOut<F>> {
        Ok(self
            .ctx
            .store_reader
            .get_checkpoint_tree_root(checkpoint_id)
            .await
            .map_err(RpcError::Anyhow)?)
    }

    async fn get_checkpoint_tree_root_f(&self, checkpoint_id: F) -> RpcResult<QHashOut<F>> {
        Ok(self
            .ctx
            .store_reader
            .get_checkpoint_tree_root(checkpoint_id.to_canonical_u64())
            .await
            .map_err(RpcError::Anyhow)?)
    }

    async fn get_checkpoint_tree_leaf_hash(
        &self,
        checkpoint_id: u64,
        leaf_checkpoint_id: u64,
    ) -> RpcResult<QHashOut<F>> {
        Ok(self
            .ctx
            .store_reader
            .get_checkpoint_tree_leaf_hash(checkpoint_id, leaf_checkpoint_id)
            .await
            .map_err(RpcError::Anyhow)?)
    }

    async fn get_checkpoint_tree_leaf_hash_f(
        &self,
        checkpoint_id: F,
        leaf_checkpoint_id: F,
    ) -> RpcResult<QHashOut<F>> {
        Ok(self
            .ctx
            .store_reader
            .get_checkpoint_tree_leaf_hash(
                checkpoint_id.to_canonical_u64(),
                leaf_checkpoint_id.to_canonical_u64(),
            )
            .await
            .map_err(RpcError::Anyhow)?)
    }

    async fn get_checkpoint_tree_merkle_proof(
        &self,
        checkpoint_id: u64,
        leaf_checkpoint_id: u64,
    ) -> RpcResult<MerkleProofCore<QHashOut<F>>> {
        Ok(self
            .ctx
            .store_reader
            .get_checkpoint_tree_merkle_proof(checkpoint_id, leaf_checkpoint_id)
            .await
            .map_err(RpcError::Anyhow)?)
    }

    async fn get_checkpoint_tree_merkle_proof_f(
        &self,
        checkpoint_id: F,
        leaf_checkpoint_id: F,
    ) -> RpcResult<MerkleProofCore<QHashOut<F>>> {
        Ok(self
            .ctx
            .store_reader
            .get_checkpoint_tree_merkle_proof(
                checkpoint_id.to_canonical_u64(),
                leaf_checkpoint_id.to_canonical_u64(),
            )
            .await
            .map_err(RpcError::Anyhow)?)
    }
    async fn get_checkpoint_global_state_roots(
        &self,
        checkpoint_id: u64,
    ) -> RpcResult<QEDCheckpointGlobalStateRoots<F>> {
        Ok(self
            .ctx
            .store_reader
            .get_checkpoint_global_state_roots(checkpoint_id)
            .await
            .map_err(RpcError::Anyhow)?)
    }

    async fn get_user_leaf_data(
        &self,
        checkpoint_id: u64,
        user_id: u64,
    ) -> RpcResult<QEDUserLeaf<F>> {
        Ok(self
            .ctx
            .store_reader
            .get_user_leaf_data(checkpoint_id, user_id)
            .await
            .map_err(RpcError::Anyhow)?)
    }

    async fn get_user_leaf_data_f(
        &self,
        checkpoint_id: F,
        user_id: F,
    ) -> RpcResult<QEDUserLeaf<F>> {
        Ok(self
            .ctx
            .store_reader
            .get_user_leaf_data(checkpoint_id.to_canonical_u64(), user_id.to_canonical_u64())
            .await
            .map_err(RpcError::Anyhow)?)
    }

    async fn get_user_contract_state_tree_root(
        &self,
        checkpoint_id: u64,
        user_id: u64,
        contract_id: u32,
    ) -> RpcResult<QHashOut<F>> {
        Ok(self
            .ctx
            .store_reader
            .get_user_contract_state_tree_root(checkpoint_id, user_id, contract_id)
            .await
            .map_err(RpcError::Anyhow)?)
    }

    async fn get_user_contract_state_tree_root_f(
        &self,
        checkpoint_id: F,
        user_id: F,
        contract_id: F,
    ) -> RpcResult<QHashOut<F>> {
        Ok(self
            .ctx
            .store_reader
            .get_user_contract_state_tree_root(
                checkpoint_id.to_canonical_u64(),
                user_id.to_canonical_u64(),
                contract_id.to_canonical_u64() as u32,
            )
            .await
            .map_err(RpcError::Anyhow)?)
    }

    async fn get_user_contract_state_tree_leaf_hash(
        &self,
        checkpoint_id: u64,
        user_id: u64,
        contract_id: u32,
        height: u8,
        leaf_id: u64,
    ) -> RpcResult<QHashOut<F>> {
        Ok(self
            .ctx
            .store_reader
            .get_user_contract_state_tree_leaf_hash(
                checkpoint_id,
                user_id,
                contract_id,
                height,
                leaf_id,
            )
            .await
            .map_err(RpcError::Anyhow)?)
    }

    async fn get_user_contract_state_tree_leaf_hash_f(
        &self,
        checkpoint_id: F,
        user_id: F,
        contract_id: F,
        height: u8,
        leaf_id: F,
    ) -> RpcResult<QHashOut<F>> {
        Ok(self
            .ctx
            .store_reader
            .get_user_contract_state_tree_leaf_hash_f(
                checkpoint_id,
                user_id,
                contract_id,
                height,
                leaf_id,
            )
            .await
            .map_err(RpcError::Anyhow)?)
    }

    async fn get_user_contract_state_tree_merkle_proof(
        &self,
        checkpoint_id: u64,
        user_id: u64,
        contract_id: u32,
        height: u8,
        leaf_id: u64,
    ) -> RpcResult<MerkleProofCore<QHashOut<F>>> {
        Ok(self
            .ctx
            .store_reader
            .get_user_contract_state_tree_merkle_proof(
                checkpoint_id,
                user_id,
                contract_id,
                height,
                leaf_id,
            )
            .await
            .map_err(RpcError::Anyhow)?)
    }

    async fn get_user_contract_state_tree_merkle_proof_f(
        &self,
        checkpoint_id: F,
        user_id: F,
        contract_id: F,
        height: u8,
        leaf_id: F,
    ) -> RpcResult<MerkleProofCore<QHashOut<F>>> {
        Ok(self
            .ctx
            .store_reader
            .get_user_contract_state_tree_merkle_proof_f(
                checkpoint_id,
                user_id,
                contract_id,
                height,
                leaf_id,
            )
            .await
            .map_err(RpcError::Anyhow)?)
    }

    async fn get_user_contract_tree_root(
        &self,
        checkpoint_id: u64,
        user_id: u64,
    ) -> RpcResult<QHashOut<F>> {
        Ok(self
            .ctx
            .store_reader
            .get_user_contract_tree_root(checkpoint_id, user_id)
            .await
            .map_err(RpcError::Anyhow)?)
    }

    async fn get_user_contract_tree_root_f(
        &self,
        checkpoint_id: F,
        user_id: F,
    ) -> RpcResult<QHashOut<F>> {
        Ok(self
            .ctx
            .store_reader
            .get_user_contract_tree_root(
                checkpoint_id.to_canonical_u64(),
                user_id.to_canonical_u64(),
            )
            .await
            .map_err(RpcError::Anyhow)?)
    }

    async fn get_user_contract_tree_leaf_hash(
        &self,
        checkpoint_id: u64,
        user_id: u64,
        contract_id: u32,
    ) -> RpcResult<QHashOut<F>> {
        Ok(self
            .ctx
            .store_reader
            .get_user_contract_tree_leaf_hash(checkpoint_id, user_id, contract_id)
            .await
            .map_err(RpcError::Anyhow)?)
    }

    async fn get_user_contract_tree_leaf_hash_f(
        &self,
        checkpoint_id: F,
        user_id: F,
        contract_id: F,
    ) -> RpcResult<QHashOut<F>> {
        Ok(self
            .ctx
            .store_reader
            .get_user_contract_tree_leaf_hash_f(checkpoint_id, user_id, contract_id)
            .await
            .map_err(RpcError::Anyhow)?)
    }

    async fn get_user_contract_tree_merkle_proof(
        &self,
        checkpoint_id: u64,
        user_id: u64,
        contract_id: u32,
    ) -> RpcResult<MerkleProofCore<QHashOut<F>>> {
        Ok(self
            .ctx
            .store_reader
            .get_user_contract_tree_merkle_proof(checkpoint_id, user_id, contract_id)
            .await
            .map_err(RpcError::Anyhow)?)
    }

    async fn get_user_contract_tree_merkle_proof_f(
        &self,
        checkpoint_id: F,
        user_id: F,
        contract_id: F,
    ) -> RpcResult<MerkleProofCore<QHashOut<F>>> {
        Ok(self
            .ctx
            .store_reader
            .get_user_contract_tree_merkle_proof_f(checkpoint_id, user_id, contract_id)
            .await
            .map_err(RpcError::Anyhow)?)
    }

    async fn get_user_tree_root(&self, checkpoint_id: u64) -> RpcResult<QHashOut<F>> {
        Ok(self
            .ctx
            .store_reader
            .get_user_tree_root(checkpoint_id)
            .await
            .map_err(RpcError::Anyhow)?)
    }

    async fn get_user_tree_root_f(&self, checkpoint_id: F) -> RpcResult<QHashOut<F>> {
        Ok(self
            .ctx
            .store_reader
            .get_user_tree_root(checkpoint_id.to_canonical_u64())
            .await
            .map_err(RpcError::Anyhow)?)
    }

    async fn get_user_tree_leaf_hash(
        &self,
        checkpoint_id: u64,
        user_id: u64,
    ) -> RpcResult<QHashOut<F>> {
        Ok(self
            .ctx
            .store_reader
            .get_user_tree_leaf_hash(checkpoint_id, user_id)
            .await
            .map_err(RpcError::Anyhow)?)
    }

    async fn get_user_tree_leaf_hash_f(
        &self,
        checkpoint_id: F,
        user_id: F,
    ) -> RpcResult<QHashOut<F>> {
        Ok(self
            .ctx
            .store_reader
            .get_user_tree_leaf_hash(checkpoint_id.to_canonical_u64(), user_id.to_canonical_u64())
            .await
            .map_err(RpcError::Anyhow)?)
    }

    async fn get_user_bottom_tree_merkle_proof(
        &self,
        root_level: u8,
        checkpoint_id: u64,
        user_id: u64,
    ) -> RpcResult<MerkleProofCore<QHashOut<F>>> {
        Ok(self
            .ctx
            .store_reader
            .get_user_bottom_tree_merkle_proof(root_level, checkpoint_id, user_id)
            .await
            .map_err(RpcError::Anyhow)?)
    }

    async fn get_user_bottom_tree_merkle_proof_f(
        &self,
        root_level: u8,
        checkpoint_id: F,
        user_id: F,
    ) -> RpcResult<MerkleProofCore<QHashOut<F>>> {
        Ok(self
            .ctx
            .store_reader
            .get_user_bottom_tree_merkle_proof(
                root_level,
                checkpoint_id.to_canonical_u64(),
                user_id.to_canonical_u64(),
            )
            .await
            .map_err(RpcError::Anyhow)?)
    }

    async fn get_user_sub_tree_merkle_proof(
        &self,
        checkpoint_id: u64,
        root_level: u8,
        leaf_level: u8,
        leaf_index: u64,
    ) -> RpcResult<MerkleProofCore<QHashOut<F>>> {
        Ok(self
            .ctx
            .store_reader
            .get_user_sub_tree_merkle_proof(checkpoint_id, root_level, leaf_level, leaf_index)
            .await
            .map_err(RpcError::Anyhow)?)
    }

    async fn get_user_sub_tree_merkle_proof_f(
        &self,
        checkpoint_id: F,
        root_level: u8,
        leaf_level: u8,
        leaf_index: F,
    ) -> RpcResult<MerkleProofCore<QHashOut<F>>> {
        Ok(self
            .ctx
            .store_reader
            .get_user_sub_tree_merkle_proof(
                checkpoint_id.to_canonical_u64(),
                root_level,
                leaf_level,
                leaf_index.to_canonical_u64(),
            )
            .await
            .map_err(RpcError::Anyhow)?)
    }

    async fn get_user_tree_merkle_proof(
        &self,
        checkpoint_id: u64,
        user_id: u64,
    ) -> RpcResult<MerkleProofCore<QHashOut<F>>> {
        tracing::info!(
            "get_user_tree_merkle_proof: checkpoint_id={}, user_id={}",
            checkpoint_id,
            user_id
        );
        Ok(self
            .ctx
            .store_reader
            .get_user_tree_merkle_proof(checkpoint_id, user_id)
            .await
            .map_err(RpcError::Anyhow)?)
    }
}

#[async_trait]
impl<SR, DQ, PS> JobSchedulerRpcServer for RealmEdgeHandler<SR, DQ, PS>
where
    SR: QEDRealmStoreReaderAsync<F> + Sync + Send + 'static,
    DQ: CheckpointDrainQueueEmitterAsyncImm + Sync + Send + 'static,
    PS: QProofStoreAsyncImm + Sync + Send + 'static,
{
    async fn get_pending_job(&self) -> RpcResult<Option<QJob>> {
        let j = match self.job_task_store.claim_job_from_current_layer().await {
            Ok(job) => job,
            Err(e) => {
                error!("Error claiming job from current task: {:?}", e);
                return Err(crate::coordinator::edge::error::RpcError::Anyhow(e.into()));
            }
        };
        match j {
            Some(job) => {
                debug!("Pending job from current task: {:?}", job);
                Ok(Some(job))
            },
            None => {
                debug!("No pending job from current task");
                Ok(None)
            }
        }
    }

    async fn get_proof_by_id(&self, job_id: QProvingJobDataID) -> RpcResult<Vec<u8>> {
        let proof: ConcreteProofWithPublicInputs = self
            .ctx
            .proof_store
            .get_proof_by_id(job_id)
            .await
            .map_err(|e| RpcError::Anyhow(e.into()))?;
        let bytes = bincode::serialize(&proof).map_err(|e| RpcError::Anyhow(e.into()))?;
        Ok(bytes)
    }

    async fn get_bytes_by_id(&self, job_id: QProvingJobDataID) -> RpcResult<Vec<u8>> {
        let bytes = self
            .ctx
            .proof_store
            .get_bytes_by_id(job_id)
            .await
            .map_err(RpcError::Anyhow)?;
        Ok(bytes)
    }

    async fn set_proof_by_id(
        &self,
        job: QJob,
        proof: Option<ConcreteProofWithPublicInputs>,
    ) -> RpcResult<()> {
        let job_id = job.job_id;

        // CRITICAL: Validate job ownership before processing proof
        let validation_status = self.job_task_store.validate_job_ownership(&job).await
            .map_err(|e| crate::coordinator::edge::error::RpcError::Anyhow(anyhow!("Failed to validate job: {}", e)))?;

        match validation_status {
            JobValidationStatus::Valid => {
                info!("✅ Job {:?} validated successfully, proceeding with proof", job_id);
            }
            JobValidationStatus::NoActiveLayer => {
                error!("⚠️ No active layer when submitting proof for job {:?}", job_id);
                return Err(crate::coordinator::edge::error::RpcError::Anyhow(anyhow!(
                    "System error: no active layer"
                )));
            }
            JobValidationStatus::WrongLayer { expected, provided } => {
                error!(
                "⚠️ Worker submitted job {:?} for wrong layer: expected {}, got {}",
                job_id, expected, provided
            );
                self.log_suspicious_activity(&job, "wrong_layer").await;
                return Err(crate::coordinator::edge::error::RpcError::Anyhow(anyhow!(
                    "Invalid submission: wrong layer (expected {}, got {})",
                    expected, provided
                )));
            }
            JobValidationStatus::MessageNotFound => {
                error!(
                    "⚠️ Worker submitted proof for non-existent job {:?}, msg_id: {}",
                    job_id, job.msg_id
                );
                self.log_suspicious_activity(&job, "message_not_found").await;
                return Err(crate::coordinator::edge::error::RpcError::Anyhow(anyhow!(
                    "Invalid submission: job not found"
                )));
            }
            JobValidationStatus::MessageNotHidden => {
                error!(
                    "⚠️ Worker submitted proof for non-hidden job {:?}, msg_id: {}",
                    job_id, job.msg_id
                );
                self.log_suspicious_activity(&job, "message_not_hidden").await;
                return Err(crate::coordinator::edge::error::RpcError::Anyhow(anyhow!(
                    "Invalid submission: job not being processed"
                )));
            }
        }

        if let Some(proof) = proof {
            info!("Setting proof by id: {:?}", job_id);
            self.ctx.proof_verifier.verify_proof_of_type(job_id.circuit_type, &proof)
                .map_err(|e| RpcError::Anyhow(e.into()))?;
            let output_id = job_id.get_output_id();
            self.ctx
                .proof_store
                .set_proof_by_id(output_id, &proof)
                .await
                .map_err(RpcError::Anyhow)?;
        }

        // remove the job from the current task, no matter if proof is None or Some
        match self.job_task_store.acknowledge_job_completion(&job).await {
            Ok(_) => {
                info!("Job completed successfully: {:?}", job_id);
            },
            Err(e) => {
                error!("Error acknowledging job completion: {:?}", e);
                return Err(ErrorObject::owned(
                    ErrorCode::InternalError.code(),
                    format!("Failed to acknowledge job completion: {}", e),
                    None::<()>,
                ));
            }
        }
        if job_id.topic == QJobTopic::NotifyOrchestratorComplete
            || job_id.circuit_type == ProvingJobCircuitType::NotifyRealmComplete
        {
            info!("Notifying core goal completed: {:?}", job_id);
            self.job_notify_queue
                .notify_core_goal_completed_imm(job_id)
                .await
                .map_err(RpcError::Anyhow)?;
        }
        Ok(())
    }
}

pub async fn spawn_realm_job_update_task<
    SR: QEDRealmStoreReaderAsync<F> + Sync + Send + 'static,
    DQ: CheckpointDrainQueueEmitterAsyncImm + Sync + Send + 'static,
    PS: QProofStoreAsyncImm + Sync + Send + 'static,
>(
    proof_store: Arc<ProofStoreRedisAsync>,
    realm_id: u64,
    coordinator_addr: String,
    ctx: Arc<RealmEdgeContext<SR, DQ, PS>>,
    retry_config: Option<RetryConfig>,
) -> anyhow::Result<()> {
    info!("realm job listener spawned");
    
    // Create RealmProofSender instance once
    let proof_sender = Arc::new(RealmProofSender::new(realm_id, coordinator_addr, retry_config)?);
    
    tokio::spawn(async move {
        let mut last_checkpoint = match ctx.get_checkpoint_id_async().await {
            Ok(checkpoint) => {
                let next_checkpoint = checkpoint + 1;
                info!("Starting realm job update task from checkpoint: {} (latest local: {})", next_checkpoint, checkpoint);
                next_checkpoint
            },
            Err(e) => {
                warn!("Failed to get latest local checkpoint, starting from 0: {}", e);
                0u64
            }
        };
        loop {
            // Listen for new proof job IDs from the history queue
            match proof_store
                .wait_for_next_item_imm::<ProvingJobDataId>(
                    REALM_PROOF_SYNC_CHANNEL,
                    last_checkpoint,
                )
                .await
            {
                Ok(job_id) => {
                    info!(?job_id, "Received proof from realm processor");
                    last_checkpoint = job_id.checkpoint_id + 1;
                    
                    // Use the RealmProofSender instance
                    if let Err(err) = proof_sender.send_proof(ctx.proof_store.clone(), job_id).await {
                        error!("Failed to send realm proof: {:?}", err);
                    }
                }
                Err(err) => {
                    error!("Error getting job_id from history queue: {:?}", err);
                    // Avoid busy waiting on error
                    tokio::time::sleep(Duration::from_millis(500)).await;
                }
            }
        }
    });
    Ok(())
}

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

/// Handles sending realm proofs to coordinator with optimized HTTP client reuse and retry mechanisms
pub struct RealmProofSender {
    realm_id: u64,
    http_client: HttpClient,
    retry_config: RetryConfig,
}

impl RealmProofSender {
    /// Create a new RealmProofSender instance
    pub fn new(realm_id: u64, coordinator_addr: String, retry_config: Option<RetryConfig>) -> anyhow::Result<Self> {
        let secret = env::var("JWT_SECRET").expect("JWT_SECRET must be set in .env");
        let jwt_token = generate_jwt_token(&secret, realm_id)?;
        let bearer_token_value = format!("Bearer {}", jwt_token);
        let header_value = HeaderValue::from_str(&bearer_token_value)?;

        let mut headers = HeaderMap::new();
        headers.insert("Authorization", header_value);

        let http_client = HttpClientBuilder::default()
            .set_headers(headers)
            .build(&coordinator_addr)?;

        Ok(Self {
            realm_id,
            http_client,
            retry_config: retry_config.unwrap_or_default(),
        })
    }

    /// Generic retry function for any async operation
    async fn retry_with_backoff<T, F, Fut, E>(&self, operation_name: &str, mut operation: F) -> anyhow::Result<T>
    where
        F: FnMut() -> Fut,
        Fut: std::future::Future<Output = Result<T, E>>,
        E: std::fmt::Debug,
    {
        for attempt in 0..self.retry_config.max_retries {
            match operation().await {
                Ok(result) => return Ok(result),
                Err(err) => {
                    error!("{} failed: {:?}, attempt {}/{}", operation_name, err, attempt + 1, self.retry_config.max_retries);

                    if attempt < self.retry_config.max_retries - 1 {
                        let delay = if self.retry_config.exponential_backoff {
                            Duration::from_millis(self.retry_config.base_delay_ms * 2_u64.pow(attempt))
                        } else {
                            Duration::from_millis(self.retry_config.base_delay_ms)
                        };
                        tokio::time::sleep(delay).await;
                    }
                }
            }
        }

        Err(anyhow!("{} failed after {} attempts", operation_name, self.retry_config.max_retries))
    }

    /// Send realm proof to coordinator with unified retry mechanism
    pub async fn send_proof<PS: QProofStoreAsyncImm>(
        &self,
        proof_store: Arc<PS>,
        job_info: ProvingJobDataId,
    ) -> anyhow::Result<()> {
        info!(?job_info.job_id, "send_realm_proof start");

        // Get bytes with retry
        let bytes = self.get_bytes_with_retry(proof_store.clone(), job_info.job_id).await?;

        let preview_len = bytes.len().min(100);
        let hex_preview = hex::encode(&bytes[..preview_len]);
        debug!(
            "The bytes from job_info.job_id: len = {}, head[0..{}] = {}",
            bytes.len(),
            preview_len,
            hex_preview
        );

        // Deserialize realm result
        let realm_result: GUTARealmCheckpointResult<QEDFelt> = bincode::deserialize(&bytes)?;

        // Get proof with retry
        let proof = self.get_proof_with_retry(proof_store, realm_result.proof_id.get_output_id()).await?;

        eprintln!(
            "DEBUGPRINT[686]: context.rs:885: proof={}",
            serde_json::to_string_pretty(&proof.public_inputs).unwrap()
        );

        let input = SubmitGUTARealmResultAPINoProofInput::<QEDFelt> {
            realm_id: self.realm_id,
            checkpoint_id: realm_result.checkpoint_id,
            guta_stats: realm_result.guta_stats,
            top_line_proof: realm_result.top_line_proof,
            checkpoint_tree_root: realm_result.checkpoint_tree_root,
            circuit_type: realm_result.proof_id.circuit_type,
        };

        // Submit with retry
        self.submit_with_retry(input, proof).await?;

        Ok(())
    }

    /// Get bytes from proof store with retry mechanism
    async fn get_bytes_with_retry<PS: QProofStoreAsyncImm>(
        &self,
        proof_store: Arc<PS>,
        job_id: QProvingJobDataID,
    ) -> anyhow::Result<Vec<u8>> {
        self.retry_with_backoff("get_bytes_by_id", || async {
            match proof_store.get_bytes_by_id(job_id).await {
                Ok(bytes) if !bytes.is_empty() => Ok(bytes),
                Ok(_) => {
                    Err(anyhow!("empty bytes"))
                }
                Err(err) => Err(err),
            }
        }).await
    }

    /// Get proof from proof store with retry mechanism
    async fn get_proof_with_retry<PS: QProofStoreAsyncImm>(
        &self,
        proof_store: Arc<PS>,
        proof_id: QProvingJobDataID,
    ) -> anyhow::Result<ProofWithPublicInputs<QEDFelt, PoseidonGoldilocksConfig, 2>> {
        self.retry_with_backoff("get_proof_by_id", || async {
            proof_store.get_proof_by_id(proof_id).await
        }).await
    }

    /// Submit request to coordinator with retry mechanism
    async fn submit_with_retry(
        &self,
        input: SubmitGUTARealmResultAPINoProofInput<QEDFelt>,
        proof: ProofWithPublicInputs<QEDFelt, PoseidonGoldilocksConfig, 2>,
    ) -> anyhow::Result<()> {
        self.retry_with_backoff("submit_guta_proof", || async {
            info!("Sending job to coordinator");
            let params = rpc_params![input.clone(), proof.clone()];
            match self.http_client.request::<String, _>("qed_submit_guta", params).await {
                Ok(result) => {
                    info!("Successfully submitted job to coordinator, result: {}", result);
                    Ok(())
                }
                Err(err) => Err(err),
            }
        }).await
    }
}
