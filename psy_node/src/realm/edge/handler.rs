use std::{str::FromStr, sync::Arc, time::Duration};

use anyhow::{anyhow, bail, ensure};
use async_trait::async_trait;
use jsonrpsee::{
    core::{client::ClientT, RpcResult},
    rpc_params,
    types::{ErrorCode, ErrorObject},
};
use plonky2::{
    field::{
        goldilocks_field::GoldilocksField,
        types::{Field, PrimeField64},
    },
    plonk::{config::PoseidonGoldilocksConfig, proof::ProofWithPublicInputs},
};
use psy_common::{
    data::qhashout::QHashOut,
    job::{
        drain_queue::CheckpointDrainQueueEmitterAsyncImm,
        history_queue::CheckpointHistoryQueueConsumerAsyncImm,
        id::{ProvingJobCircuitType, ProvingJobDataType, QJobTopic, QProvingJobDataID, VariableHeightRewardMerkleProof},
        traits::QProofStoreAsyncImm,
        worker_queue::WorkerEventReceiverAsyncImm,
    },
};
use psy_config::network_constants::{GLOBAL_CONTRACT_TREE_HEIGHT, GLOBAL_USER_TREE_HEIGHT};
use psy_crypto::hash::{
    merkle::core::{compute_historical_and_current_merkle_roots_core_gt, DeltaMerkleProofCore, MerkleProofCore},
    traits::{
        hasher::{MerkleHasher, MerkleZeroHasher, PoseidonHasher},
        qhashable::QFieldHashable,
    },
};
use psy_data::{
    config::store_config::{PsyFelt, PsyHash, PsyHasher, PsyProof},
    dpn::event::PsyUserEventRecord,
    guta::{
        api::{PsyContractStateUpdateHistory, SimpleContractHeightCache, UserEndCapNonProofCoreInputQueueItem},
        end_cap_input::SubmitUserEndCapNonProofInput,
        proof_input::VerifyEndCapSimpleStandardInput,
    },
    qdata::{
        checkpoint::{PsyBlockState, PsyCheckpointGlobalStateRoots, PsyCheckpointLeaf},
        user::PsyUserLeaf,
        user_endcap_metadata::UserEndCapMetaData,
        uuid::UserEndCapUUID,
    },
};
use psy_network_circuit::verify_witness::verify_witness_and_proof;
use psy_provider::{session::TxStatus, wallet::secp_sign::SignedRequest};
use psy_store::{
    node::realm::PsyRealmStoreReaderAsync,
    queue::{
        task_queue::{current_timestamp_millis, JobValidationStatus, QJob, QProvingTaskStore, QProvingTaskStoreImpl},
        ProofStoreRedis,
    },
};
use tracing::{debug, error, info, warn};

use super::{error::RpcError, rpc::RealmEdgeRpcServer};
use crate::{
    common::{
        jobs::{JobSchedulerRpcServer, MESSAGE_CLAIM_JOB},
        traits::realm::CoordinatorClient,
        utils::current_datetime,
        whitelist::{WhiteList, WhiteListCache},
    },
    coordinator::edge::ProofStore,
    realm::{client::ConcreteCoordinatorClient, state::edge::RealmEdgeContext, C, D, F, H},
    watcher::{
        events::{JobCompletedEvent, JobStartedEvent, UserEndcapSubmissionEvent, UserEndcapSubmissionMetadata, WatcherMessage},
        watcher_client::WatcherClient,
        WatcherSourceNodeType,
    },
};

#[derive(Clone)]
pub struct RealmEdgeHandler<SR: PsyRealmStoreReaderAsync<F> + Sync, DQ: CheckpointDrainQueueEmitterAsyncImm, PS: QProofStoreAsyncImm> {
    ctx: RealmEdgeContext<SR, DQ, PS>,
    job_notify_queue: Arc<ProofStoreRedis>,
    task_store: Arc<QProvingTaskStoreImpl>,
    whitelist_cache: WhiteListCache,
    watcher_client: Arc<WatcherClient>,
    coordinator_client: Arc<ConcreteCoordinatorClient>,
}

impl<SR, DQ, PS> RealmEdgeHandler<SR, DQ, PS>
where
    SR: PsyRealmStoreReaderAsync<F> + Sync,
    DQ: CheckpointDrainQueueEmitterAsyncImm,
    PS: QProofStoreAsyncImm,
{
    pub fn new(
        ctx: RealmEdgeContext<SR, DQ, PS>,
        job_notify_queue: Arc<ProofStoreRedis>,
        task_store: Arc<QProvingTaskStoreImpl>,
        whitelist_cache: WhiteListCache,
        watcher_client: Arc<WatcherClient>,
    ) -> Result<Self, anyhow::Error> {
        let coordinator_client = ctx.coordinator_client.clone();
        Ok(Self {
            ctx,
            job_notify_queue,
            task_store,
            whitelist_cache,
            watcher_client,
            coordinator_client,
        })
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
    SR: PsyRealmStoreReaderAsync<F> + Sync + Send + 'static,
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
    ) -> RpcResult<UserEndCapUUID> {
        let checkpoint_id = self
            .ctx
            .store_reader
            .get_latest_block_state()
            .await
            .map_err(RpcError::Anyhow)?
            .checkpoint_id;
        let slot_updates = user_ec_input.get_slot_updates().map_err(RpcError::Anyhow)?;
        let user_endcap_uuid = self
            .ctx
            .handle_recv_end_cap_from_user(user_ec_input.clone(), &proof)
            .await
            .map_err(RpcError::Anyhow)?;

        let endcap_event = UserEndcapSubmissionEvent {
            realm_id: self.ctx.realm_config.realm_id as u64,
            user_id: user_ec_input.core.state_transition.user_id.to_canonical_u64(),
            metadata: UserEndcapSubmissionMetadata {
                checkpoint_id,
                user_endcap_uuid,
                state_transition: user_ec_input.core.state_transition,
                new_user_leaf: user_ec_input.core.new_user_leaf,
                endcap_proof_public_inputs: proof.public_inputs.clone(),
                node_id: self.watcher_client.node_id.clone().unwrap_or_default(),
                node_type: WatcherSourceNodeType::Realm.to_string(),
                slot_updates,
            },
            timestamp: current_datetime(),
        };

        if let Err(e) = self.watcher_client.send_event(WatcherMessage::EndcapSubmission(endcap_event)).await {
            warn!("⚠️ Failed to report endcap submission event to watcher: {}", e);
        }

        Ok(user_endcap_uuid)
    }

    async fn get_tx_status(&self, user_id: u64, nonce: u64) -> RpcResult<TxStatus> {
        Ok(self.ctx.get_tx_status(user_id, nonce).await.map_err(RpcError::Anyhow)?)
    }

    async fn get_checkpoint_leaf_data(&self, checkpoint_id: u64) -> RpcResult<PsyCheckpointLeaf<F>> {
        Ok(self
            .ctx
            .store_reader
            .get_checkpoint_leaf_data(checkpoint_id)
            .await
            .map_err(RpcError::Anyhow)?)
    }

    async fn get_latest_block_state(&self) -> RpcResult<PsyBlockState> {
        Ok(self.ctx.store_reader.get_latest_block_state().await.map_err(RpcError::Anyhow)?)
    }

    async fn get_block_state(&self, checkpoint_id: u64) -> RpcResult<PsyBlockState> {
        Ok(self.ctx.store_reader.get_block_state(checkpoint_id).await.map_err(RpcError::Anyhow)?)
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
        Ok(self.ctx.store_reader.get_latest_checkpoint_tree_root().await.map_err(RpcError::Anyhow)?)
    }

    async fn get_checkpoint_tree_root(&self, checkpoint_id: u64) -> RpcResult<QHashOut<F>> {
        Ok(self
            .ctx
            .store_reader
            .get_checkpoint_tree_root(checkpoint_id)
            .await
            .map_err(RpcError::Anyhow)?)
    }

    async fn get_checkpoint_tree_leaf_hash(&self, checkpoint_id: u64, leaf_checkpoint_id: u64) -> RpcResult<QHashOut<F>> {
        Ok(self
            .ctx
            .store_reader
            .get_checkpoint_tree_leaf_hash(checkpoint_id, leaf_checkpoint_id)
            .await
            .map_err(RpcError::Anyhow)?)
    }

    async fn get_checkpoint_tree_merkle_proof(&self, checkpoint_id: u64, leaf_checkpoint_id: u64) -> RpcResult<MerkleProofCore<QHashOut<F>>> {
        Ok(self
            .ctx
            .store_reader
            .get_checkpoint_tree_merkle_proof(checkpoint_id, leaf_checkpoint_id)
            .await
            .map_err(RpcError::Anyhow)?)
    }

    async fn get_checkpoint_global_state_roots(&self, checkpoint_id: u64) -> RpcResult<PsyCheckpointGlobalStateRoots<F>> {
        Ok(self
            .ctx
            .store_reader
            .get_checkpoint_global_state_roots(checkpoint_id)
            .await
            .map_err(RpcError::Anyhow)?)
    }

    async fn get_user_leaf_data(&self, checkpoint_id: u64, user_id: u64) -> RpcResult<PsyUserLeaf<F>> {
        Ok(self
            .ctx
            .store_reader
            .get_user_leaf_data(checkpoint_id, user_id)
            .await
            .map_err(RpcError::Anyhow)?)
    }

    async fn get_user_contract_state_tree_root(&self, checkpoint_id: u64, user_id: u64, contract_id: u32) -> RpcResult<QHashOut<F>> {
        Ok(self
            .ctx
            .store_reader
            .get_user_contract_state_tree_root(checkpoint_id, user_id, contract_id)
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
            .get_user_contract_state_tree_leaf_hash(checkpoint_id, user_id, contract_id, height, leaf_id)
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
            .get_user_contract_state_tree_merkle_proof(checkpoint_id, user_id, contract_id, height, leaf_id)
            .await
            .map_err(RpcError::Anyhow)?)
    }

    async fn get_user_contract_tree_root(&self, checkpoint_id: u64, user_id: u64) -> RpcResult<QHashOut<F>> {
        Ok(self
            .ctx
            .store_reader
            .get_user_contract_tree_root(checkpoint_id, user_id)
            .await
            .map_err(RpcError::Anyhow)?)
    }

    async fn get_user_contract_tree_leaf_hash(&self, checkpoint_id: u64, user_id: u64, contract_id: u32) -> RpcResult<QHashOut<F>> {
        Ok(self
            .ctx
            .store_reader
            .get_user_contract_tree_leaf_hash(checkpoint_id, user_id, contract_id)
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

    async fn get_user_tree_root(&self, checkpoint_id: u64) -> RpcResult<QHashOut<F>> {
        Ok(self.ctx.store_reader.get_user_tree_root(checkpoint_id).await.map_err(RpcError::Anyhow)?)
    }

    async fn get_user_tree_leaf_hash(&self, checkpoint_id: u64, user_id: u64) -> RpcResult<QHashOut<F>> {
        Ok(self
            .ctx
            .store_reader
            .get_user_tree_leaf_hash(checkpoint_id, user_id)
            .await
            .map_err(RpcError::Anyhow)?)
    }

    async fn get_user_bottom_tree_merkle_proof(&self, root_level: u8, checkpoint_id: u64, user_id: u64) -> RpcResult<MerkleProofCore<QHashOut<F>>> {
        Ok(self
            .ctx
            .store_reader
            .get_user_bottom_tree_merkle_proof(root_level, checkpoint_id, user_id)
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

    async fn get_user_tree_merkle_proof(&self, checkpoint_id: u64, user_id: u64) -> RpcResult<MerkleProofCore<QHashOut<F>>> {
        tracing::info!("get_user_tree_merkle_proof: checkpoint_id={}, user_id={}", checkpoint_id, user_id);
        Ok(self
            .ctx
            .store_reader
            .get_user_tree_merkle_proof(checkpoint_id, user_id)
            .await
            .map_err(RpcError::Anyhow)?)
    }

    async fn generate_batch_variable_height_reward_proofs(
        &self,
        checkpoint_id: u64,
        job_ids: Vec<QProvingJobDataID>,
    ) -> RpcResult<Vec<(VariableHeightRewardMerkleProof, QProvingJobDataID)>> {
        use jsonrpsee::types::ErrorObject;

        let mut actual_checkpoint_id = checkpoint_id;
        let mut job_graph = None;

        for offset in 0..5 {
            let candidate_checkpoint_id = checkpoint_id + offset;

            if let Ok(graph) = self.task_store.load_job_dependency_graph(candidate_checkpoint_id).await {
                let all_jobs_found = job_ids.iter().all(|job_id| {
                    if job_id.circuit_type.is_user_registration_job() {
                        graph.user_registrations_graph.has_node(job_id)
                    } else if job_id.circuit_type.is_deploy_contracts_job() {
                        graph.deploy_contracts_graph.has_node(job_id)
                    } else if job_id.circuit_type.is_guta_job() {
                        graph.guta_graph.has_node(job_id)
                    } else {
                        false
                    }
                });
                if all_jobs_found {
                    actual_checkpoint_id = candidate_checkpoint_id;
                    job_graph = Some(graph);
                    break;
                }
            }
        }

        let graph = job_graph.ok_or_else(|| {
            ErrorObject::owned(
                jsonrpsee::types::ErrorCode::InvalidParams.code(),
                format!("Jobs not found in checkpoints {} to {}", checkpoint_id, checkpoint_id + 4),
                None::<()>,
            )
        })?;

        let mut proofs = Vec::new();

        for job_id in job_ids {
            debug!("job_id: {}", job_id.to_hex_string());
            match graph
                .generate_variable_height_reward_proof(job_id, self.ctx.realm_config.realm_id, &*self.ctx.proof_store)
                .await
            {
                Ok((realm_proof, root_job_id)) => {
                    debug!(
                        "realm proof: {}, root_job_id: {}",
                        serde_json::to_string_pretty(&realm_proof).unwrap(),
                        root_job_id.to_hex_string()
                    );
                    let coordinator_proofs = self
                        .coordinator_client
                        .rpc_client
                        .request::<Vec<(VariableHeightRewardMerkleProof, QProvingJobDataID)>, _>(
                            "psy_generate_batch_variable_height_reward_proofs",
                            jsonrpsee::rpc_params![checkpoint_id, vec![root_job_id]],
                        )
                        .await
                        .map_err(|e| {
                            ErrorObject::owned(
                                jsonrpsee::types::ErrorCode::InternalError.code(),
                                format!("Failed to get coordinator proof: {}", e),
                                None::<()>,
                            )
                        })?;

                    if coordinator_proofs.is_empty() {
                        return Err(ErrorObject::owned(
                            jsonrpsee::types::ErrorCode::InternalError.code(),
                            "No coordinator proof returned".to_string(),
                            None::<()>,
                        ));
                    }

                    let (coordinator_proof, root_job_id) = coordinator_proofs.into_iter().next().unwrap();
                    debug!(
                        "coordinator proof: {}, root_job_id: {}",
                        serde_json::to_string_pretty(&coordinator_proof).unwrap(),
                        root_job_id.to_hex_string()
                    );
                    let combined_proof = realm_proof.combine_with(coordinator_proof);
                    debug!("combined proof: {}", serde_json::to_string_pretty(&combined_proof).unwrap());

                    let (computed_root, _) = combined_proof.compute_root_and_nullifier_index();

                    let checkpoint_leaf = self.ctx.store_reader.get_checkpoint_leaf_data(root_job_id.goal_id).await.map_err(|e| {
                        ErrorObject::owned(
                            jsonrpsee::types::ErrorCode::InternalError.code(),
                            format!("Failed to get checkpoint data for {}: {}", root_job_id.goal_id, e),
                            None::<()>,
                        )
                    })?;
                    let expected_root = if job_id.circuit_type.is_user_registration_job() {
                        checkpoint_leaf.stats.pm_rewards_commitment.register_users_root
                    } else if job_id.circuit_type.is_guta_job() {
                        checkpoint_leaf.stats.pm_rewards_commitment.gutas_root
                    } else if job_id.circuit_type.is_deploy_contracts_job() {
                        checkpoint_leaf.stats.pm_rewards_commitment.deploy_contracts_root
                    } else {
                        return Err(ErrorObject::owned(
                            jsonrpsee::types::ErrorCode::InvalidParams.code(),
                            format!("Job type {:?} not supported for proof generation", job_id.circuit_type),
                            None::<()>,
                        ));
                    };

                    if computed_root != expected_root {
                        warn!(
                            "Root mismatch for job({}): expected {}, got {}",
                            job_id.to_hex_string(),
                            expected_root,
                            computed_root
                        );
                    }

                    proofs.push((combined_proof, root_job_id));
                }
                Err(e) => {
                    error!("Failed to generate proof for job {}: {}", job_id.to_hex_string(), e);
                    return Err(ErrorObject::owned(
                        jsonrpsee::types::ErrorCode::InternalError.code(),
                        format!("Failed to generate proof for job {}: {}", job_id.to_hex_string(), e),
                        None::<()>,
                    ));
                }
            }
        }

        Ok(proofs)
    }

    async fn get_graphviz(&self, checkpoint_id: u64) -> RpcResult<String> {
        let graph = self.task_store.load_job_dependency_graph(checkpoint_id).await.map_err(|e| {
            ErrorObject::owned(
                jsonrpsee::types::ErrorCode::InternalError.code(),
                format!("Failed to load job dependency graph: {}", e),
                None::<()>,
            )
        })?;
        let graphviz_content = graph.get_graphviz();
        Ok(graphviz_content)
    }

    async fn get_user_event_tree_root(&self, checkpoint_id: u64, user_id: u64) -> RpcResult<QHashOut<F>> {
        let user_event_tree_root = self
            .ctx
            .store_reader
            .get_user_event_tree_root(checkpoint_id, user_id)
            .await
            .map_err(RpcError::Anyhow)?;
        Ok(user_event_tree_root)
    }

    async fn get_user_event_tree_leaf_hash(&self, checkpoint_id: u64, user_id: u64, event_index: u64) -> RpcResult<QHashOut<F>> {
        let user_event_tree_leaf_hash = self
            .ctx
            .store_reader
            .get_user_event_tree_leaf_hash(checkpoint_id, user_id, event_index)
            .await
            .map_err(RpcError::Anyhow)?;
        Ok(user_event_tree_leaf_hash)
    }

    async fn get_user_event_tree_merkle_proof(&self, checkpoint_id: u64, user_id: u64, event_index: u64) -> RpcResult<MerkleProofCore<QHashOut<F>>> {
        let user_event_tree_merkle_proof = self
            .ctx
            .store_reader
            .get_user_event_tree_merkle_proof(checkpoint_id, user_id, event_index)
            .await
            .map_err(RpcError::Anyhow)?;
        Ok(user_event_tree_merkle_proof)
    }

    async fn get_user_event_data(&self, checkpoint_id: u64, user_id: u64, event_index: u64) -> RpcResult<PsyUserEventRecord<F>> {
        let user_event = self
            .ctx
            .store_reader
            .get_user_event_data(checkpoint_id, user_id, event_index)
            .await
            .map_err(RpcError::Anyhow)?;
        Ok(user_event)
    }

    async fn get_user_endcap_metadata(&self, user_endcap_uuid: String) -> RpcResult<UserEndCapMetaData<F>> {
        let user_endcap_uuid = UserEndCapUUID::from_str(&user_endcap_uuid).map_err(|e| {
            ErrorObject::owned(
                jsonrpsee::types::ErrorCode::InvalidParams.code(),
                format!("Failed to parse user endcap uuid: {}", e),
                None::<()>,
            )
        })?;
        tracing::debug!("get_user_endcap_metadata: {:?}", user_endcap_uuid);
        let user_endcap_metadata: UserEndCapMetaData<GoldilocksField> = self
            .ctx
            .store_reader
            .get_user_endcap_metadata(user_endcap_uuid)
            .await
            .map_err(RpcError::Anyhow)?;
        Ok(user_endcap_metadata)
    }
}

#[async_trait]
impl<SR, DQ, PS> JobSchedulerRpcServer for RealmEdgeHandler<SR, DQ, PS>
where
    SR: PsyRealmStoreReaderAsync<F> + Sync + Send + 'static,
    DQ: CheckpointDrainQueueEmitterAsyncImm + Sync + Send + 'static,
    PS: QProofStoreAsyncImm + Sync + Send + 'static,
{
    async fn get_pending_job(&self, signed: SignedRequest<PsyHash>) -> RpcResult<Option<QJob>> {
        self.whitelist_cache
            .verify_request(&signed, &MESSAGE_CLAIM_JOB.to_string(), Some(Duration::from_secs(30)))
            .map_err(|e| RpcError::Anyhow(e.into()))?;

        let worker_id = signed.worker_public_key.to_string();
        let j = match self.task_store.acquire_job(&worker_id).await {
            Ok(job) => job,
            Err(e) => {
                error!("Error claiming job from current task: {:?}", e);
                return Err(crate::coordinator::edge::error::RpcError::Anyhow(e.into()));
            }
        };
        match j {
            Some(job) if !job.job_id.is_provable() => {
                self.acknowledge_job_completion(&job, &worker_id).await.map_err(RpcError::Anyhow)?;
                Ok(None)
            }
            Some(job) if self.ctx.proof_store.contains_id(job.job_id.get_input_witness_id()).await.is_ok_and(|x| x) => {
                debug!("Pending job from current task: {:?}", job);

                // Report job started event to watcher
                let start_event = JobStartedEvent {
                    job_id: job.job_id,
                    worker_id,
                    start_time: current_timestamp_millis(),
                    layer_id: job.layer_id,
                };

                // Send to watcher queue
                let message = WatcherMessage::JobStarted(start_event);
                if let Err(e) = self.watcher_client.send_event(message).await {
                    warn!("⚠️ Failed to report job started to watcher: {}", e);
                }

                Ok(Some(job))
            }
            _ => Ok(None),
        }
    }

    async fn get_proof_by_id(&self, job_id: QProvingJobDataID) -> RpcResult<Vec<u8>> {
        let proof: PsyProof = self
            .ctx
            .proof_store
            .get_proof_by_id(job_id)
            .await
            .map_err(|e| RpcError::Anyhow(e.into()))?;
        let bytes = bincode::serialize(&proof).map_err(|e| RpcError::Anyhow(e.into()))?;
        Ok(bytes)
    }

    async fn get_bytes_by_id(&self, job_id: QProvingJobDataID) -> RpcResult<Vec<u8>> {
        let bytes = self.ctx.proof_store.get_bytes_by_id(job_id).await.map_err(RpcError::Anyhow)?;
        Ok(bytes)
    }

    async fn set_proof_by_id(&self, job: QJob, proof: PsyProof, signed: SignedRequest<PsyHash>) -> RpcResult<()> {
        // Verify signature and whitelist
        self.whitelist_cache
            .verify_request(&signed, &proof, Some(Duration::from_secs(300)))
            .map_err(|e| RpcError::Anyhow(e.into()))?;

        let job_id = job.job_id;
        // CRITICAL: Validate job ownership before processing proof
        let validation_status = self
            .task_store
            .validate_and_extend_job(&job)
            .await
            .map_err(|e| RpcError::Anyhow(anyhow!("Failed to validate job: {}", e)))?;

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
                    expected,
                    provided
                )));
            }
            JobValidationStatus::MessageNotFound => {
                error!("⚠️ Worker submitted proof for non-existent job {:?}, msg_id: {}", job_id, job.msg_id);
                self.log_suspicious_activity(&job, "message_not_found").await;
                return Err(crate::coordinator::edge::error::RpcError::Anyhow(anyhow!(
                    "Invalid submission: job not found"
                )));
            }
            JobValidationStatus::MessageNotHidden => {
                error!("⚠️ Worker submitted proof for non-hidden job {:?}, msg_id: {}", job_id, job.msg_id);
                self.log_suspicious_activity(&job, "message_not_hidden").await;
                return Err(crate::coordinator::edge::error::RpcError::Anyhow(anyhow!(
                    "Invalid submission: job not being processed"
                )));
            }
        }

        info!("Setting proof by id: {:?}", job_id);

        crate::common::log_proof_details("Realm", job_id, &proof);

        verify_witness_and_proof(&self.ctx.proof_verifier, job_id, self.ctx.proof_store.as_ref(), &proof)
            .await
            .map_err(|e| RpcError::Anyhow(e.into()))?;

        let output_id = job_id.get_output_id();
        self.ctx.proof_store.set_proof_by_id(output_id, &proof).await.map_err(RpcError::Anyhow)?;

        // remove the job from the current task, no matter if proof is None or Some
        let worker_id = signed.worker_public_key.to_string();
        self.acknowledge_job_completion(&job, worker_id).await.map_err(RpcError::Anyhow)?;
        Ok(())
    }
}

impl<SR, DQ, PS> RealmEdgeHandler<SR, DQ, PS>
where
    SR: PsyRealmStoreReaderAsync<F> + Sync,
    DQ: CheckpointDrainQueueEmitterAsyncImm,
    PS: QProofStoreAsyncImm,
{
    async fn acknowledge_job_completion(&self, job: &QJob, worker_id: impl ToString) -> anyhow::Result<()> {
        let job_id = job.job_id;
        let worker_id = worker_id.to_string();

        let job_status = match self.task_store.mark_job_completed(&job, &worker_id).await {
            Ok(status) => {
                info!("Job completed successfully: {:?}", job_id);
                status
            }
            Err(e) => {
                error!("Error acknowledging job completion: {:?}", e);
                return Err(e.into());
            }
        };

        // Send job completion event to watcher (external to task_store)
        if let Some(duration_ms) = job_status.duration_ms() {
            let completed_event = JobCompletedEvent {
                job_id: job_id.clone(),
                worker_id: Some(worker_id.clone()),
                start_time: job_status.start_time,
                end_time: job_status.end_time.unwrap_or_else(current_timestamp_millis),
                duration_ms,
            };

            // Send to watcher but don't fail the job completion if it fails
            if let Err(e) = self.watcher_client.send_event(WatcherMessage::JobCompleted(completed_event)).await {
                warn!("Failed to send job completion event to watcher: {}", e);
            } else {
                info!("📊 Job completion reported to watcher for job {:?}", job_id);
            }
        }

        if job_id.is_notify_complete() {
            info!("Notifying core goal completed: {:?}", job_id);
            self.job_notify_queue.notify_core_goal_completed_imm(job_id).await?;
        }

        Ok(())
    }
}
