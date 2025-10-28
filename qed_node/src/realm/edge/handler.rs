use super::error::RpcError;
use super::rpc::RealmEdgeRpcServer;
use crate::common::jobs::{JobSchedulerRpcServer, MESSAGE_CLAIM_JOB};
use crate::realm::state::edge::RealmEdgeContext;
use crate::realm::{C, D, F, H};
use crate::watcher::current_timestamp_mills;
use async_trait::async_trait;
use jsonrpsee::core::{client::ClientT, RpcResult};
use jsonrpsee::http_client::{HttpClient, HttpClientBuilder};
use jsonrpsee::rpc_params;
use plonky2::plonk::config::PoseidonGoldilocksConfig;
use plonky2::{field::types::PrimeField64, plonk::proof::ProofWithPublicInputs};
use plonky2::field::types::Field;
use qed_core::job::history_queue::CheckpointHistoryQueueConsumerAsyncImm;
use qed_core::data::qhashout::QHashOut;
use qed_core::job::worker_queue::WorkerEventReceiverAsyncImm;
use qed_core::job::{
    drain_queue::CheckpointDrainQueueEmitterAsyncImm,
    id::{ProvingJobCircuitType, QJobTopic, QProvingJobDataID, VariableHeightRewardMerkleProof},
    traits::QProofStoreAsyncImm,
};
use qed_crypto::hash::merkle::core::{DeltaMerkleProofCore, MerkleProofCore, compute_historical_and_current_merkle_roots_core_gt};
use qed_data::config::store_config::{QEDFelt, QEDHash, QEDHasher, QEDProof};
use qed_data::guta::end_cap_input::SubmitUserEndCapNonProofInput;
use qed_data::qdata::checkpoint::{
    QEDCheckpointGlobalStateRoots, QEDCheckpointLeaf, QEDL2BlockState,
};
use qed_data::qdata::user::QEDUserLeaf;
use qed_store::node::realm::QEDRealmStoreReaderAsync;
use qed_store::queue::ProofStoreRedisAsync;
use std::sync::Arc;
use std::time::Duration;
use anyhow::{anyhow, bail, ensure};
use jsonrpsee::types::{ErrorCode, ErrorObject};
use plonky2::field::goldilocks_field::GoldilocksField;
use tracing::{debug, error, info, warn};
use qed_core::config::network_constants::{GLOBAL_CONTRACT_TREE_HEIGHT, GLOBAL_USER_TREE_HEIGHT};
use qed_core::job::id::ProvingJobDataType;
use qed_crypto::hash::traits::hasher::{MerkleHasher, MerkleZeroHasher, PoseidonHasher};
use qed_crypto::hash::traits::qhashable::QFieldHashable;
use qed_data::guta::api::{QEDContractStateUpdateHistory, SimpleContractHeightCache, UserEndCapNonProofCoreInputQueueItem};
use qed_data::guta::proof_input::VerifyEndCapSimpleStandardInput;
use qed_prover::wallet::secp_sign::SignedRequest;
use qed_store::queue::task_queue::{QProvingTaskStore, QProvingTaskStoreImpl, JobValidationStatus, QJob, current_timestamp_millis};
use crate::coordinator::edge::ProofStore;
use qed_rollup_circuit::verify_witness::verify_witness_and_proof;
use qed_store::queue::tx_pool::TxPoolAsyncImm;
use crate::common::whitelist::{WhiteList, WhiteListCache};
use crate::common_v2::traits::realm::{RealmEdgeContractStateTreeUpdate, RealmEdgeStateHelper, RealmEdgeUserContractTreeUpdate, RealmEdgeUserUpdateSubmission, SimpleTreeUpdateBuilder, UniqueQueueId};
use crate::realm::state::edge_queue_helper::RealmEdgeQueueHelper;
use crate::watcher::events::{JobCompletedEvent, JobStartedEvent, WatcherMessage};
use crate::watcher::watcher_client::WatcherClient;

#[derive(Clone)]
pub struct RealmEdgeHandler<
    SR: QEDRealmStoreReaderAsync<F> + Sync,
    DQ: TxPoolAsyncImm + CheckpointDrainQueueEmitterAsyncImm,
    PS: TxPoolAsyncImm + QProofStoreAsyncImm,
> {
    ctx: RealmEdgeContext<SR, DQ, PS>,
    job_notify_queue: Arc<ProofStoreRedisAsync>,
    task_store: Arc<QProvingTaskStoreImpl>,
    whitelist_cache: WhiteListCache,
    watcher_client: Arc<WatcherClient>,
    coordinator_client: HttpClient,
    queue_helper: Arc<RealmEdgeQueueHelper<F>>,
}

impl<SR, DQ, PS> RealmEdgeHandler<SR, DQ, PS>
where
    SR: QEDRealmStoreReaderAsync<F> + Sync,
    DQ: TxPoolAsyncImm + CheckpointDrainQueueEmitterAsyncImm,
    PS: TxPoolAsyncImm + QProofStoreAsyncImm,
{
    pub fn new(
        ctx: RealmEdgeContext<SR, DQ, PS>,
        job_notify_queue: Arc<ProofStoreRedisAsync>,
        task_store: Arc<QProvingTaskStoreImpl>,
        whitelist_cache: WhiteListCache,
        watcher_client: Arc<WatcherClient>,
        coordinator_addr: &str,
        edge_queue_helper: Arc<RealmEdgeQueueHelper<F>>,
    ) -> Result<Self, anyhow::Error> {
        let coordinator_client = HttpClientBuilder::default()
            .build(coordinator_addr)?;
        Ok(Self {
            ctx,
            job_notify_queue,
            task_store,
            whitelist_cache,
            watcher_client,
            coordinator_client,
            queue_helper: edge_queue_helper,
        })
    }

    async fn log_suspicious_activity(&self, job: &QJob, reason: &str) {
        //todo! add some operation to log suspicious activity or ban user
        error!(
            "🚨 SECURITY ALERT: Invalid job submission - Reason: {}, Job: {:?}, Layer: {}, MsgId: {}",
            reason, job.job_id, job.layer_id, job.msg_id
        );
    }

    fn validate_contract_updates(
        &self,
        contract_updates: &[QEDContractStateUpdateHistory<F>],
        old_user_leaf: &QEDUserLeaf<F>,
        new_user_leaf: &QEDUserLeaf<F>,
    ) -> anyhow::Result<()> {
        if contract_updates.is_empty() {
            return Ok(());
        }

        let first = &contract_updates[0].user_contract_tree_update_proof;
        let last = &contract_updates[contract_updates.len() - 1].user_contract_tree_update_proof;

        ensure!(
            first.old_root == old_user_leaf.user_state_tree_root,
            "Contract update chain broken: first old_root mismatch"
        );

        ensure!(
            last.new_root == new_user_leaf.user_state_tree_root,
            "Contract update chain broken: last new_root mismatch"
        );

        contract_updates.windows(2).enumerate().try_for_each(|(i, pair)| {
            ensure!(
                pair[0].user_contract_tree_update_proof.new_root ==
                pair[1].user_contract_tree_update_proof.old_root,
                "Contract update chain broken at index {}: discontinuous roots", i + 1
            );
            Ok(())
        })
    }

    async fn build_submission_from_end_cap(
        &self,
        user_ec_input: &SubmitUserEndCapNonProofInput<F>,
        checkpoint_id: u64,
        user_id: u64
    ) -> anyhow::Result<RealmEdgeUserUpdateSubmission<F>> {
        // ensure!(
        //     user_ec_input.core.checkpoint_id.to_canonical_u64() == checkpoint_id,
        //     "Checkpoint mismatch: {} != {}",
        //     user_ec_input.core.checkpoint_id.to_canonical_u64(),
        //     checkpoint_id
        // );
        let endcap_checkpoint_id = user_ec_input.core.checkpoint_id.to_canonical_u64();
        if endcap_checkpoint_id != checkpoint_id {
            tracing::warn!("user cap checkpoint is behind current checkpoint: {} != {}", endcap_checkpoint_id, checkpoint_id);
        }

        let old_user_leaf = self.ctx.store_reader
            .get_user_leaf_data(checkpoint_id, user_id)
            .await?;

        let old_leaf_hash = old_user_leaf.qfhash::<QEDHasher>();
        ensure!(
            old_leaf_hash == user_ec_input.core.state_transition.start_user_leaf_hash,
            "Start leaf hash mismatch"
        );

        let new_user_leaf = user_ec_input.core.new_user_leaf;
        let new_leaf_hash = new_user_leaf.qfhash::<QEDHasher>();

        ensure!(
            new_leaf_hash == user_ec_input.core.state_transition.end_user_leaf_hash,
            "End leaf hash mismatch"
        );

        ensure!(
            new_user_leaf.user_id == user_ec_input.core.state_transition.user_id,
            "User ID mismatch in new leaf"
        );

        //note: The checkpoint_id here is not used by the outside world and can be passed in any value
        let cst_update = user_ec_input
            .verify_and_generate_cst_updates::<QEDHasher>(
                checkpoint_id,
                old_user_leaf.user_state_tree_root
            )?;

        let contract_state_updates = cst_update.updates
            .iter()
            .map(|delta| RealmEdgeContractStateTreeUpdate {
                user_id,
                contract_id: delta.key.contract_id,
                index: delta.key.index,
                level: delta.key.level,
                new_value: delta.value,
            })
            .collect();

        let user_contract_updates = cst_update.uct_updates
            .iter()
            .map(|update| RealmEdgeUserContractTreeUpdate {
                user_id,
                index: update.key.index as u32,
                level: update.key.level,
                new_value: update.value,
            })
            .collect();

        let checkpoint_proof = self.ctx.store_reader
            .get_checkpoint_tree_merkle_proof(checkpoint_id, endcap_checkpoint_id)
            .await?;

        let (historical_root, current_root) = compute_historical_and_current_merkle_roots_core_gt::<QHashOut<F>, QEDHasher>(&checkpoint_proof);
        ensure!(current_root == checkpoint_proof.root);
        ensure!(historical_root == user_ec_input.core.state_transition.checkpoint_tree_root_hash);

        if checkpoint_proof.root != user_ec_input.core.state_transition.checkpoint_tree_root_hash {
            tracing::warn!(
                "ensure checkpoint_proof: {} == user_ec_input.core.state_transition.checkpoint_tree_root_hash {}",
                checkpoint_proof.root,
                user_ec_input.core.state_transition.checkpoint_tree_root_hash,
            );
            // anyhow::bail!("invalid checkpoint_root_hash");
        }

        Ok(RealmEdgeUserUpdateSubmission {
            proof_id: QProvingJobDataID::new(
                QJobTopic::GenerateStandardProof,
                checkpoint_id,
                0, // slot_id - default value since this is for proof tracking
                self.ctx.realm_config.realm_id as u32,
                GLOBAL_USER_TREE_HEIGHT as u32,
                user_id as u32,
                ProvingJobCircuitType::UserEndCap,
                ProvingJobDataType::OutputProof,
                0,
            ),
            contract_state_tree_updates: contract_state_updates,
            user_contract_tree_updates: user_contract_updates,
            old_user_leaf,
            new_user_leaf,
            misc_data: VerifyEndCapSimpleStandardInput {
                guta_stats: user_ec_input.core.stats,
                checkpoint_root: historical_root,
                checkpoint_historical_merkle_proof: checkpoint_proof,
            },
        })
    }

    async fn handle_end_cap_with_queue(
        &self,
        user_ec_input: SubmitUserEndCapNonProofInput<F>,
        proof: ProofWithPublicInputs<F, C, D>,
    ) -> anyhow::Result<String> {
        let user_id = user_ec_input.core.state_transition.user_id.to_canonical_u64();

        //Step1: The realm edge fetches the unique shared checkpoint id which includes a checkpoint_id AND a 128bit uuid
        let unique_checkpoint = self.get_shared_checkpoint_id().await?;
        debug!("checkpoint={:?} user={}", unique_checkpoint, user_id);

        //Step2: The realm edge checks if the user has submitted a proof for this UNIQUE checkpoint id before,
        // if not, it stores a random number for the UNIQUE checkpoint id in redis or similar
        ensure!(
            !self.queue_helper.has_user_submitted(unique_checkpoint, user_id).await?,
            "User {} already submitted for checkpoint {:?}", user_id, unique_checkpoint
        );

        let random_lock = rand::random::<u128>();
        self.put_submitted_end_cap_for_checkpoint(random_lock, user_id).await?;

        //Step3: The realm edge checks the proof, and validity of inputs,
        // generating the RealmEdgeUserUpdateSubmission ALONG THE WAY
        // (no need to waste compute on processor that has already been completed)
        let validation_result = (|| async {
            ensure!(proof.public_inputs.len() == 4, "Invalid proof inputs");
            ensure!(!user_ec_input.contract_state_updates.is_empty(), "No contract updates");
            ensure!(self.ctx.includes_user_id(user_id), "User not in realm");

            let mut contracts = SimpleContractHeightCache::<F>::new();
            for (id, height) in user_ec_input.get_needed_contract_zero_hashes() {
                contracts.add_contract(id, height as u8, PoseidonHasher::get_zero_hash(height));
            }

            let proof_hash = QHashOut::from_felt_slice(&proof.public_inputs);
            user_ec_input.ensure_simple_self_consistent::<QEDHasher>(proof_hash, &contracts)?;

            let current_checkpoint = self.ctx.get_checkpoint_id_async().await?;
            let end_cap_checkpoint = user_ec_input.core.checkpoint_id.to_canonical_u64();
            ensure!(
                end_cap_checkpoint <= current_checkpoint,
                "Future checkpoint: {} > {}", end_cap_checkpoint, current_checkpoint
            );

            let user_leaf = self.ctx.store_reader
                .get_user_leaf_data(current_checkpoint, user_id)
                .await?;

            ensure!(
                user_leaf.qfhash::<QEDHasher>() == user_ec_input.core.state_transition.start_user_leaf_hash,
                "User state mismatch"
            );

            ensure!(
                user_leaf.last_checkpoint_id.to_canonical_u64() <= end_cap_checkpoint &&
                user_leaf.nonce.to_canonical_u64() <= user_ec_input.core.new_user_leaf.nonce.to_canonical_u64(),
                "Invalid state progression"
            );

            self.ctx.verify_proof_of_type(ProvingJobCircuitType::UserEndCap, &proof)?;

            let submission = self.build_submission_from_end_cap(
                &user_ec_input,
                current_checkpoint,
                user_id,
            ).await?;

            self.validate_contract_updates(
                &user_ec_input.contract_state_updates,
                &submission.old_user_leaf,
                &submission.new_user_leaf,
            )?;

            Ok::<_, anyhow::Error>((submission, end_cap_checkpoint, current_checkpoint))
        })().await;

        let (submission, end_cap_checkpoint, current_checkpoint) = validation_result?;

        //Step4: The realm edge fetches the random number for the UNIQUE checkpoint id and user,
        // and makes sure it equals the previous random number generated (to ensure no weird race conditions with multiple submissions)
        ensure!(
            self.has_submitted_end_cap_for_checkpoint(random_lock, user_id).await?,
            "Random lock verification failed"
        );

        //Step5: The realm edge stores the proof in proof store
        let proof_id = submission.proof_id;
        self.put_proof_id(proof_id, proof.into()).await?;

        //Step6: The realm edge pushes the RealmEdgeUserUpdateSubmission to a queue or similar that is tied to the UniqueCheckpointId (both checkpoint_id and uuid)
        self.queue_helper
            .enqueue_submission(unique_checkpoint, submission)
            .await
            .map_err(|e| {
                anyhow::anyhow!("Failed to enqueue submission: {}", e)
            })?;

        self.queue_helper
            .mark_user_submitted(unique_checkpoint, user_id)
            .await?;

        info!("✅ User {} proof accepted for checkpoint {:?}", user_id, unique_checkpoint);

        Ok(format!(
            "Proof submitted successfully for checkpoint {:?} (user: {}, lock: {})",
            unique_checkpoint, user_id, random_lock
        ))
    }

}

#[async_trait]
impl<SR, DQ, PS> RealmEdgeRpcServer for RealmEdgeHandler<SR, DQ, PS>
where
    SR: QEDRealmStoreReaderAsync<F> + Sync + Send + 'static,
    DQ: TxPoolAsyncImm + CheckpointDrainQueueEmitterAsyncImm + Sync + Send + 'static,
    PS: TxPoolAsyncImm + QProofStoreAsyncImm + Sync + Send + 'static,
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
                    match job_id.circuit_type {
                        ProvingJobCircuitType::AppendUserRegistrationTree |
                        ProvingJobCircuitType::AppendUserRegistrationTreeAggregate |
                        ProvingJobCircuitType::DummyAppendUserRegistrationTreeAggregate => {
                            graph.user_registrations_graph.has_node(job_id)
                        }
                        ProvingJobCircuitType::BatchDeployContracts |
                        ProvingJobCircuitType::BatchDeployContractsAggregate |
                        ProvingJobCircuitType::DummyBatchDeployContractsAggregate => {
                            graph.deploy_contracts_graph.has_node(job_id)
                        }
                        ProvingJobCircuitType::GUTARegisterUsers |
                        ProvingJobCircuitType::GUTAOnlyRegisterUsers |
                        ProvingJobCircuitType::GUTATwoGUTA | ProvingJobCircuitType::GUTANoChange |
                        ProvingJobCircuitType::GUTASingleEndCap | ProvingJobCircuitType::GUTATwoEndCap |
                        ProvingJobCircuitType::GUTALeftEndCapRightGUTA | ProvingJobCircuitType::GUTALeftGUTARightEndCap |
                        ProvingJobCircuitType::GUTATwoGUTAWithCheckpointUpgrade  |
                        ProvingJobCircuitType::GUTAVerifyToCapWithCheckpointUpgrade |
                        ProvingJobCircuitType::GUTAVerifyToCap => {
                            graph.guta_graph.has_node(job_id)
                        }
                        _ => false
                    }
                });
                if all_jobs_found {
                    actual_checkpoint_id = candidate_checkpoint_id;
                    job_graph = Some(graph);
                    break;
                }
            }
        }

        let graph = job_graph.ok_or_else(|| ErrorObject::owned(
            jsonrpsee::types::ErrorCode::InvalidParams.code(),
            format!("Jobs not found in checkpoints {} to {}", checkpoint_id, checkpoint_id + 4),
            None::<()>,
        ))?;


        let mut proofs = Vec::new();

        for job_id in job_ids {
            debug!("job_id: {}", job_id.to_hex_string());
            match graph.generate_variable_height_reward_proof(job_id, self.ctx.realm_config.realm_id, &*self.ctx.proof_store).await {
                Ok((realm_proof, root_job_id)) => {
                    debug!("realm proof: {}, root_job_id: {}", serde_json::to_string_pretty(&realm_proof).unwrap(), root_job_id.to_hex_string());
                    let coordinator_proofs = self.coordinator_client
                        .request::<Vec<(VariableHeightRewardMerkleProof, QProvingJobDataID)>, _>(
                            "qed_generate_batch_variable_height_reward_proofs",
                            jsonrpsee::rpc_params![checkpoint_id, vec![root_job_id]]
                        ).await.map_err(|e| ErrorObject::owned(
                            jsonrpsee::types::ErrorCode::InternalError.code(),
                            format!("Failed to get coordinator proof: {}", e),
                            None::<()>,
                        ))?;

                    if coordinator_proofs.is_empty() {
                        return Err(ErrorObject::owned(
                            jsonrpsee::types::ErrorCode::InternalError.code(),
                            "No coordinator proof returned".to_string(),
                            None::<()>,
                        ));
                    }

                    let (coordinator_proof, root_job_id) = coordinator_proofs.into_iter().next().unwrap();
                    debug!("coordinator proof: {}, root_job_id: {}", serde_json::to_string_pretty(&coordinator_proof).unwrap(), root_job_id.to_hex_string());
                    let combined_proof = realm_proof.combine_with(coordinator_proof);
                    debug!("combined proof: {}", serde_json::to_string_pretty(&combined_proof).unwrap());

                    let (computed_root, _) = combined_proof.compute_root_and_nullifier_index();

                    let checkpoint_leaf = self.ctx.store_reader
                        .get_checkpoint_leaf_data(root_job_id.goal_id)
                        .await
                        .map_err(|e| ErrorObject::owned(
                            jsonrpsee::types::ErrorCode::InternalError.code(),
                            format!("Failed to get checkpoint data for {}: {}", root_job_id.goal_id, e),
                            None::<()>,
                        ))?;
                    let expected_root = match job_id.circuit_type {
                        ProvingJobCircuitType::AppendUserRegistrationTree |
                        ProvingJobCircuitType::AppendUserRegistrationTreeAggregate |
                        ProvingJobCircuitType::DummyAppendUserRegistrationTreeAggregate => {
                            checkpoint_leaf.stats.pm_rewards_commitment.register_users_root
                        }
                        ProvingJobCircuitType::GUTARegisterUsers |
                        ProvingJobCircuitType::GUTAOnlyRegisterUsers |
                        ProvingJobCircuitType::GUTATwoGUTA | ProvingJobCircuitType::GUTANoChange | ProvingJobCircuitType::GUTASingleEndCap |
                        ProvingJobCircuitType::GUTATwoEndCap | ProvingJobCircuitType::GUTALeftEndCapRightGUTA |
                        ProvingJobCircuitType::GUTATwoGUTAWithCheckpointUpgrade | ProvingJobCircuitType::GUTAVerifyToCapWithCheckpointUpgrade |
                        ProvingJobCircuitType::GUTALeftGUTARightEndCap | ProvingJobCircuitType::GUTAVerifyToCap => {
                            checkpoint_leaf.stats.pm_rewards_commitment.gutas_root
                        }
                        ProvingJobCircuitType::BatchDeployContracts |
                        ProvingJobCircuitType::BatchDeployContractsAggregate |
                        ProvingJobCircuitType::DummyBatchDeployContractsAggregate => {
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

                    if computed_root != expected_root {
                        tracing::warn!(
                            "Root mismatch for job({}): expected {}, got {}",
                            job_id.to_hex_string(), expected_root, computed_root
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
        let graph = self.task_store.load_job_dependency_graph(checkpoint_id).await
            .map_err(|e| ErrorObject::owned(
                jsonrpsee::types::ErrorCode::InternalError.code(),
                format!("Failed to load job dependency graph: {}", e),
                None::<()>,
            ))?;
        let graphviz_content = graph.get_graphviz();
        Ok(graphviz_content)
    }
}

#[async_trait]
impl<SR, DQ, PS> JobSchedulerRpcServer for RealmEdgeHandler<SR, DQ, PS>
where
    SR: QEDRealmStoreReaderAsync<F> + Sync + Send + 'static,
    DQ: TxPoolAsyncImm + CheckpointDrainQueueEmitterAsyncImm + Sync + Send + 'static,
    PS: TxPoolAsyncImm + QProofStoreAsyncImm + Sync + Send + 'static,
{
    async fn get_pending_job(&self, signed: SignedRequest<QEDHash>) -> RpcResult<Option<QJob>> {
        self.whitelist_cache.verify_request(&signed, &MESSAGE_CLAIM_JOB.to_string(), Some(Duration::from_secs(30))).map_err(|e|
            RpcError::Anyhow(e.into())
        )?;

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
                    start_time: current_timestamp_mills(),
                    layer_id: job.layer_id,
                };

                // Send to watcher queue
                let message = WatcherMessage::JobStarted(start_event);
                if let Err(e) = self.watcher_client.send_event(message).await {
                    warn!("⚠️ Failed to report job started to watcher: {}", e);
                }

                Ok(Some(job))
            },
            _ => {
                Ok(None)
            }
        }
    }

    async fn get_proof_by_id(&self, job_id: QProvingJobDataID) -> RpcResult<Vec<u8>> {
        let proof: QEDProof = self
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
        proof: QEDProof,
        signed: SignedRequest<QEDHash>,
    ) -> RpcResult<()> {
        // Verify signature and whitelist
        self.whitelist_cache.verify_request(&signed, &proof, Some(Duration::from_secs(300))).map_err(|e|
            RpcError::Anyhow(e.into())
        )?;

        let job_id = job.job_id;
        // CRITICAL: Validate job ownership before processing proof
        let validation_status = self.task_store.validate_and_extend_job(&job).await
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

        info!("Setting proof by id: {:?}", job_id);

        crate::common::log_proof_details("Realm", job_id, &proof);

        verify_witness_and_proof(
            &self.ctx.proof_verifier,
            job_id,
            self.ctx.proof_store.as_ref(),
            &proof,
        ).await.map_err(|e| RpcError::Anyhow(e.into()))?;

        let output_id = job_id.get_output_id();
        self.ctx
            .proof_store
            .set_proof_by_id(output_id, &proof)
            .await
            .map_err(RpcError::Anyhow)?;

        // remove the job from the current task, no matter if proof is None or Some
        let worker_id = signed.worker_public_key.to_string();
        self.acknowledge_job_completion(&job, worker_id).await.map_err(RpcError::Anyhow)?;
        Ok(())
    }
}


impl<SR, DQ, PS> RealmEdgeHandler<SR, DQ, PS>
where
    SR: QEDRealmStoreReaderAsync<F> + Sync,
    DQ: TxPoolAsyncImm + CheckpointDrainQueueEmitterAsyncImm,
    PS: TxPoolAsyncImm + QProofStoreAsyncImm,
{
    async fn acknowledge_job_completion(&self, job: &QJob, worker_id: impl ToString) -> anyhow::Result<()> {
        let job_id = job.job_id;
        let worker_id = worker_id.to_string();

        let job_status = match self.task_store.mark_job_completed(&job, &worker_id).await {
            Ok(status) => {
                info!("Job completed successfully: {:?}", job_id);
                status
            },
            Err(e) => {
                error!("Error acknowledging job completion: {:?}", e);
                return Err(e.into())
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
            self.job_notify_queue
                .notify_core_goal_completed_imm(job_id)
                .await?;
        }

        Ok(())
    }
}

impl<SR, DQ, PS> RealmEdgeStateHelper for RealmEdgeHandler<SR, DQ, PS>
where
    SR: QEDRealmStoreReaderAsync<F> + Sync,
    DQ: TxPoolAsyncImm + CheckpointDrainQueueEmitterAsyncImm,
    PS: TxPoolAsyncImm + QProofStoreAsyncImm,
{
    async fn get_shared_checkpoint_id(&self) -> anyhow::Result<UniqueQueueId> {
        self.queue_helper.get_shared_checkpoint_id().await
    }

    async fn has_submitted_end_cap_for_checkpoint(&self, queue_uuid: u128, user_id: u64) -> anyhow::Result<bool> {
        let shared_checkpoint = self.get_shared_checkpoint_id().await?;
        let key = format!("{}_{}", shared_checkpoint.uuid, user_id);
        let value = self.queue_helper.get_key(&key).await;
        if let Ok(stored_queue_uuid) = value {
            return if stored_queue_uuid == queue_uuid {
                Ok(true)
            } else {
                Ok(false)
            }
        }

        Ok(false)
    }

    async fn put_submitted_end_cap_for_checkpoint(&self, queue_uuid: u128, user_id: u64) -> anyhow::Result<()> {
        let shared_checkpoint = self.get_shared_checkpoint_id().await?;
        let key = format!("{}_{}", shared_checkpoint.uuid, user_id);
        self.queue_helper.set_key(&key, queue_uuid).await?;

        Ok(())
    }

    async fn put_proof_id(&self, job_id: QProvingJobDataID, proof: QEDProof) -> anyhow::Result<()> {
        self.ctx.proof_store.set_proof_by_id(job_id, &proof).await?;

        debug!("Stored proof for job {} in realm {}",
            job_id.to_hex_string(),
            self.ctx.realm_config.realm_id
        );

        Ok(())
    }
}

