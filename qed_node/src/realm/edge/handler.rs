use super::error::RpcError;
use super::rpc::RealmEdgeRpcServer;
use crate::common::jobs::{JobSchedulerRpcServer};
use crate::common::ConcreteProofWithPublicInputs;
use crate::realm::state::edge::RealmEdgeContext;
use crate::realm::{C, D, F, H};
use async_trait::async_trait;
use jsonrpsee::core::{client::ClientT, RpcResult};
use jsonrpsee::http_client::{HeaderMap, HeaderValue};
use jsonrpsee::rpc_params;
use plonky2::plonk::config::PoseidonGoldilocksConfig;
use plonky2::{field::types::PrimeField64, plonk::proof::ProofWithPublicInputs};
use plonky2::field::types::Field;
use qed_core::config::network_constants::REALM_PROOF_SYNC_CHANNEL;
use qed_core::job::history_queue::CheckpointHistoryQueueConsumerAsyncImm;
use qed_core::job::id::ProvingJobDataId;
use qed_core::data::qhashout::QHashOut;
use qed_core::job::worker_queue::WorkerEventReceiverAsyncImm;
use qed_core::job::{
    drain_queue::CheckpointDrainQueueEmitterAsyncImm,
    id::{ProvingJobCircuitType, QJobTopic, QProvingJobDataID, JobProof},
    traits::QProofStoreAsyncImm,
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
use tokio::sync::Mutex;
use tracing::{debug, error, info, warn};
use qed_store::queue::task_queue::{QProvingTaskStore, QProvingTaskStoreImpl, JobValidationStatus, QJob};

#[derive(Clone)]
pub struct RealmEdgeHandler<
    SR: QEDRealmStoreReaderAsync<F> + Sync,
    DQ: CheckpointDrainQueueEmitterAsyncImm,
    PS: QProofStoreAsyncImm,
> {
    ctx: RealmEdgeContext<SR, DQ, PS>,
    job_notify_queue: Arc<ProofStoreRedisAsync>,
    task_store: Arc<QProvingTaskStoreImpl>,

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
        task_store: Arc<QProvingTaskStoreImpl>,
    ) -> Self {
        Self {
            ctx,
            job_notify_queue,
            task_store,
        }
    }
    async fn log_suspicious_activity(&self, job: &QJob, reason: &str) {
        //todo! add some operation to log suspicious activity or ban user
        error!(
            "🚨 SECURITY ALERT: Invalid job submission - Reason: {}, Job: {:?}, Layer: {}, MsgId: {}",
            reason, job.job_id, job.task_id, job.msg_id
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


    async fn generate_batch_proofs(
        &self,
        checkpoint_id: u64,
        job_ids: Vec<QProvingJobDataID>,
    ) -> RpcResult<Vec<JobProof>> {
        use jsonrpsee::types::ErrorObject;

        for job_id in &job_ids {
            if job_id.goal_id != checkpoint_id {
                return Err(ErrorObject::owned(
                    jsonrpsee::types::ErrorCode::InvalidParams.code(),
                    format!("Job ID {:?} does not belong to checkpoint {}", job_id, checkpoint_id),
                    None::<()>,
                ));
            }
        }

        let checkpoint_leaf = self.ctx.store_reader
            .get_checkpoint_leaf_data(checkpoint_id)
            .await
            .map_err(|e| ErrorObject::owned(
                jsonrpsee::types::ErrorCode::InternalError.code(),
                format!("Failed to get checkpoint data: {}", e),
                None::<()>,
            ))?;

        let graph = self.task_store
            .load_job_dependency_graph(checkpoint_id)
            .await
            .map_err(|e| ErrorObject::owned(
                jsonrpsee::types::ErrorCode::InternalError.code(),
                format!("Failed to load job dependency graph for checkpoint {}: {}", checkpoint_id, e),
                None::<()>,
            ))?;

        let mut proofs = Vec::new();

        for job_id in job_ids {
            let expected_root = match job_id.circuit_type {
                ProvingJobCircuitType::GUTARegisterUsers | ProvingJobCircuitType::GUTAOnlyRegisterUsers => {
                    checkpoint_leaf.stats.pm_rewards_commitment.register_users_root
                }
                ProvingJobCircuitType::GUTATwoGUTA | ProvingJobCircuitType::GUTANoChange => {
                    checkpoint_leaf.stats.pm_rewards_commitment.gutas_root
                }
                ProvingJobCircuitType::BatchDeployContracts => {
                    checkpoint_leaf.stats.pm_rewards_commitment.deploy_contracts_root
                }
                _ => {
                    return Err(ErrorObject::owned(
                        jsonrpsee::types::ErrorCode::InvalidParams.code(),
                        format!("Job type {:?} not supported for proof generation", job_id.circuit_type),
                        None::<()>,
                    ));
                }
            };

            match graph.generate_proof(job_id, &*self.ctx.proof_store).await {
                Ok(job_proof) => {
                    if job_proof.root != expected_root {
                        tracing::warn!(
                            "Root mismatch for job {:?}: expected {:?}, got {:?}",
                            job_id, expected_root, job_proof.root
                        );
                    }

                    proofs.push(job_proof);
                }
                Err(e) => {
                    error!("Failed to generate proof for job {:?}: {}", job_id, e);
                    return Err(ErrorObject::owned(
                        jsonrpsee::types::ErrorCode::InternalError.code(),
                        format!("Failed to generate proof for job {:?}: {}", job_id, e),
                        None::<()>,
                    ));
                }
            }
        }

        Ok(proofs)
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
        let j = match self.task_store.claim_job_from_current_layer().await {
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
        let validation_status = self.task_store.validate_job_ownership(&job).await
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
            
            crate::common::log_proof_details("Realm", job_id, &proof);
            
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
        match self.task_store.acknowledge_job_completion(&job).await {
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
        if job_id.is_notify_complete() {
            info!("Notifying core goal completed: {:?}", job_id);
            self.job_notify_queue
                .notify_core_goal_completed_imm(job_id)
                .await
                .map_err(RpcError::Anyhow)?;
        }
        Ok(())
    }
}

pub async fn spawn_realm_job_update_task(
    proof_store: Arc<ProofStoreRedisAsync>,
    realm_id: u64,
    coordinator_addr: String,
) -> anyhow::Result<()> {
    info!("realm job listener spawned");
    tokio::spawn(async move {
        let mut last_checkpoint = 0u64;
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
                    // if job_id.job_id.circuit_type != GUTANoChange {
                    send_realm_proof(proof_store.clone(), job_id, realm_id, &coordinator_addr)
                        .await;
                    // }
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

async fn send_realm_proof<PS: QProofStoreAsyncImm>(
    proof_store: Arc<PS>,
    job_info: ProvingJobDataId,
    realm_id: u64,
    coordinator_addr: &str,
) {
    let mut retries_count = 0;

    info!(?job_info.job_id, "send_realm_proof start");
    let bytes = loop {
        match proof_store.get_bytes_by_id(job_info.job_id).await {
            Ok(bytes) if !bytes.is_empty() => break bytes,
            Ok(bytes) => {
                warn!("bytes is empty");
            }
            Err(err) => {
                error!("Failed to get bytes by job_id: {:?}", err);
            }
        };
        retries_count += 1;
        if (retries_count == 5) {
            error!("Failed to get bytes by job_jd");
            return;
        }
        tokio::time::sleep(Duration::from_millis(3000)).await;
    };
    let preview_len = bytes.len().min(100);
    let hex_preview = hex::encode(&bytes[..preview_len]);
    debug!(
        "The bytes from job_info.job_id: len = {}, head[0..{}] = {}",
        bytes.len(),
        preview_len,
        hex_preview
    );
    let realm_result: GUTARealmCheckpointResult<QEDFelt> = match bincode::deserialize(&bytes) {
        Ok(result) => result,
        Err(err) => {
            error!("Failed to deserialize realm_result: {:?}", err);
            return;
        }
    };
    let proof: ProofWithPublicInputs<QEDFelt, PoseidonGoldilocksConfig, 2> = match proof_store
        .get_proof_by_id(realm_result.proof_id.get_output_id())
        .await
    {
        Ok(proof) => {
            eprintln!(
                "DEBUGPRINT[686]: context.rs:885: proof={}",
                serde_json::to_string_pretty(&proof.public_inputs).unwrap()
            );
            proof
        }
        Err(err) => {
            error!("Failed to get proof_by_id: {:?}", err);
            return;
        }
    };

    let input = SubmitGUTARealmResultAPINoProofInput {
        realm_id,
        checkpoint_id: realm_result.checkpoint_id,
        guta_stats: realm_result.guta_stats,
        top_line_proof: realm_result.top_line_proof,
        checkpoint_tree_root: realm_result.checkpoint_tree_root,
        circuit_type: realm_result.proof_id.circuit_type,
    };
    let mut retry_count = 0;
    let secret = env::var("JWT_SECRET").expect("JWT_SECRET must be set in .env");

    let jwt_token = generate_jwt_token(&secret, realm_id).expect("Failed to generate JWT token");
    let bearer_token_value = format!("Bearer {}", jwt_token);
    let header_value =
        HeaderValue::from_str(&bearer_token_value).expect("Failed to create header value");
    let mut headers = HeaderMap::new();
    headers.insert("Authorization", header_value);

    while retry_count < 5 {
        info!("Sending job to coordinator, retry_count = {}", retry_count);
        let client = jsonrpsee::http_client::HttpClientBuilder::default()
            .set_headers(headers.clone())
            .build(coordinator_addr);

        match client {
            Ok(client) => {
                let params = rpc_params![input.clone(), proof.clone()];
                match client.request::<String, _>("qed_submit_guta", params).await {
                    Ok(result) => {
                        info!(
                            "Successfully submitted job to coordinator, result: {}",
                            result
                        );
                        return;
                    }
                    Err(err) => {
                        error!("Failed to call coordinator API: {:?}", err);
                    }
                }
            }
            Err(err) => {
                error!("Failed to create RPC client: {:?}", err);
            }
        }
        retry_count += 1;
        tokio::time::sleep(Duration::from_secs(1u64.pow(retry_count as u32))).await;
    }
}
