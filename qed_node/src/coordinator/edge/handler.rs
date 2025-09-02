use std::collections::HashMap;
// std
use std::sync::Arc;
use std::time::Duration;

use anyhow::{anyhow, bail};
use chrono::Utc;
use rand::RngCore;
use tokio::sync::{mpsc, Mutex};
use tracing::{debug, error, info, warn};

use kvq::traits::KVQSerializable;
use plonky2::field::types::Field;
use plonky2::plonk::config::PoseidonGoldilocksConfig;
use plonky2::plonk::proof::ProofWithPublicInputs;

use jsonrpsee::types::ErrorObject;
use qed_core::data::qhashout::QHashOut;
use qed_core::job::drain_queue::{
    CheckpointDrainQueueConsumerAsyncImm, CheckpointDrainQueueEmitterAsyncImm,
    DrainQueueMetadataTagged, WithDrainQueueMetadata,
};
use qed_core::job::id::{
    JobProof, JobProofSibling, ProvingJobCircuitType, QJobTopic, QProvingJobDataID,
};
use qed_core::job::traits::{QProofStoreReaderAsync, QProofStoreWriterAsyncImm};

use qed_crypto::hash::merkle::core::MerkleProofCore;
use qed_crypto::signature::zk::data::ZKPublicKeyInfo;

use qed_data::guta::api::{
    SubmitGUTARealmResultAPINoProofInput, SubmitGUTARealmResultAPIQueueItem,
};
use qed_data::qblock::cmds::deploy_contract::QBCDeployContract;
use qed_data::qdata::checkpoint::{
    QEDCheckpointGlobalStateRoots, QEDCheckpointLeaf, QEDL2BlockState,
};
use qed_data::qdata::contract::{ContractCodeDefinition, QEDContractLeaf};
use qed_data::qdata::user::QEDUserLeaf;
use qed_data::qsync::coordinator::{QEDCheckpointSyncInfo, QEDCheckpointSyncInfoCompact};

use crate::common::jobs::{JobSchedulerRpcServer, MESSAGE_CLAIM_JOB};
use crate::common::verifier::get_cached_generic_verifier;
use crate::coordinator::args::CoordinatorEdgeArgs;
use crate::coordinator::edge::{DrainQueue, ProofStore, StoreReader};
use crate::coordinator::error::CoordinatorError;
use crate::coordinator::state::edge::CoordinatorEdgeContext;
use qed_core::job::history_queue::CheckpointHistoryQueueEmitterAsyncImm;
use qed_core::job::worker_queue::WorkerEventReceiverAsyncImm;
use qed_data::config::store_config::{
    QCheckpointSyncInfoCompact, QEDFelt, QEDHash, QEDHasher, QEDProof,
};
use qed_data::qdata::checkpoint::CheckpointSyncInfo;
use qed_data::traits::qdatastore::qmetadata::QMetaDataStoreReaderSync;
use qed_data::traits::qdatastore::qtreedata::QTreeDataStoreReaderSync;
use qed_rollup_circuit::verify_witness::verify_witness_and_proof;
use qed_store::node::coordinator::QEDCoordinatorStoreReaderAsync;
use qed_store::queue::new_redis_async_pool;
use qed_store::queue::rsmq_queue::CEQueueNotification;
use qed_store::queue::ProofStoreRedisAsync;
use qed_store::store::{Backend, QEDStore};

use qed_crypto::hash::traits::qhashable::QFieldHashable;

type F = QEDFelt;
type C = PoseidonGoldilocksConfig;
const D: usize = 2;

#[derive(Clone)]
pub struct CoordinatorEdgeHandler {
    history_queue: Arc<ProofStore>,
    proof_store: Arc<ProofStore>,
    ctx: CoordinatorEdgeContext<StoreReader, DrainQueue, ProofStore>,
    store: Arc<StoreReader>,
    task_store: Arc<QProvingTaskStoreImpl>,
    white_list: Arc<WhiteList>,
}

impl CoordinatorEdgeHandler {
    pub async fn new(args: CoordinatorEdgeArgs) -> anyhow::Result<Self> {
        info!("🚀 Initializing coordinator edge handler...");

        // Create QED store reader from backend configuration
        info!("🗄️ Initializing storage backend...");
        let qed_store = QEDStore::from_backend(args.backend.to_backend()).await?;
        let store_reader = Arc::new(qed_store);
        let redis_pool = new_redis_async_pool(&args.redis_uri, args.redis_pool_size).await?;
        let task_store = QProvingTaskStoreImpl::new(&args.redis_uri, args.redis_pool_size).await?;
        let qe_args = &args.queue_args;

        let proof_store = Arc::new(
            ProofStoreRedisAsync::new(redis_pool.clone(), qe_args.queue_biz_key.clone()).await?,
        );

        // init verifier
        let verifier = Arc::new(get_cached_generic_verifier::<_, 2>());

        let edge_config = crate::coordinator::state::processor::CoordinatorConfig::get_standard(0);

        // init context
        let ctx = CoordinatorEdgeContext::new(
            edge_config,
            store_reader.clone(),
            Arc::clone(&proof_store),
            Arc::clone(&proof_store),
            verifier,
        )
        .await?;

        let whitelist = WhiteList::from_file(&args.config_path)?;

        Ok(Self {
            history_queue: Arc::clone(&proof_store),
            proof_store: Arc::clone(&proof_store),
            ctx,
            store: store_reader,
            task_store: Arc::new(task_store),
            white_list: Arc::new(whitelist),
        })
    }

    async fn get_latest_checkpoint_id(&self) -> anyhow::Result<u64> {
        let block_state =
            QEDCoordinatorStoreReaderAsync::get_latest_l2_block_state(&*self.store).await?;
        Ok(block_state.checkpoint_id)
    }

    pub async fn register_user(
        &self,
        zk_user_info: ZKPublicKeyInfo<QEDFelt>,
    ) -> Result<(), CoordinatorError> {
        let public_key_hash = zk_user_info.qfhash::<QEDHasher>();

        // Check if user is already registered using the store reader
        if let Ok(user_id) = self.store.get_first_user_id(public_key_hash).await {
            info!("🛑 User already registered, user_id = {}", user_id);
            return Ok(()); // Already registered is not an error
        }

        info!("🆕 User not found. Starting new registration.");
        tracing::info!(
            "✅ register user: {}",
            serde_json::to_string_pretty(&zk_user_info).unwrap()
        );
        self.ctx
            .checkpoint_queue
            .cdq_push_imm(zk_user_info)
            .await
            .map_err(|e| CoordinatorError::QueueError(e.to_string()))?;
        info!("✅ User pushed to checkpoint queue.");
        Ok(())
    }

    pub async fn get_user_id(
        &self,
        public_key: QHashOut<QEDFelt>,
    ) -> Result<u64, CoordinatorError> {
        self.store.get_first_user_id(public_key).await.map_err(|_| {
            warn!("❌ User not found for public key: {:?}", public_key);
            CoordinatorError::UserNotFound { public_key }
        })
    }

    pub async fn deploy_contract(
        &self,
        contract: QBCDeployContract<QEDFelt>,
    ) -> anyhow::Result<()> {
        let latest = self.get_latest_checkpoint_id().await?;
        let next_checkpoint_id = latest + 1;

        let with_root = contract.into_with_whitelist_root::<QEDHasher>()?;

        let cd_for_queue = WithDrainQueueMetadata::new_params(
            self.ctx.coordinator_config.deploy_contract_channel_id,
            next_checkpoint_id,
            rand::thread_rng().next_u64(),
            with_root,
        );

        self.ctx.checkpoint_queue.cdq_push_imm(cd_for_queue).await?;
        Ok(())
    }

    pub async fn submit_guta(
        &self,
        input: SubmitGUTARealmResultAPINoProofInput<QEDFelt>,
        proof: ProofWithPublicInputs<QEDFelt, PoseidonGoldilocksConfig, 2>,
    ) -> anyhow::Result<()> {
        debug!(
            "submit_guta input: {}",
            serde_json::to_string_pretty(&input).unwrap()
        );
        let checkpoint_queue = self.ctx.checkpoint_queue.clone();
        let proof_store = self.ctx.proof_store.clone();
        let config = self.ctx.coordinator_config.clone();
        let verifier = self.ctx.proof_verifier.clone();
        // verify top line proof
        if !input.top_line_proof.verify::<QEDHasher>() {
            anyhow::bail!("invalid top line proof from realm");
        }

        if input.top_line_proof.new_root != input.top_line_proof.new_value {
            anyhow::bail!("top line not currently supported for guta proofs");
        }

        // verify proof
        verifier.verify_proof_of_type(input.circuit_type, &proof)?;

        //if circuit type is GUTANoChange, disable the proof
        if input.circuit_type == ProvingJobCircuitType::GUTANoChange {
            info!("⚠️ GUTANoChange proof, disabling it");
            return Ok(());
        }
        tracing::info!(
            "✅ verified guta result proof public input: {:?} ",
            proof.public_inputs
        );

        // verify state consistency
        let old_root = match self
            .store
            .get_user_latest_top_tree_cap_root(config.realm_root_level, input.realm_id)
            .await
        {
            Ok(root) => root,
            Err(e) => {
                error!("❌ Failed to get old root: {:?}", e);

                let mut source = e.source();
                while let Some(err) = source {
                    error!("⛓ Caused by: {}", err);
                    source = err.source();
                }

                return Err(anyhow::anyhow!("Failed to get old root"));
            }
        };

        info!(
            "old root from db: {:?}, hex = {:?}",
            old_root,
            hex::encode(old_root.to_bytes()?)
        );
        info!("old root from realm: {:?}", input.top_line_proof.old_root);
        if old_root != input.top_line_proof.old_root && old_root != input.top_line_proof.new_root {
            // anyhow::bail!("invalid top line proof old value from realm");
            tracing::warn!("invalid top line proof old value from realm");
        }

        // build queue item
        let queue_item: SubmitGUTARealmResultAPIQueueItem<GoldilocksField> =
            input.to_queue_item(config.guta_channel_id, config.realm_root_level as u32);
        let proof_id = queue_item.proof_id;
        info!(
            "🚀 Pushing GUTA result to drain queue, realm_id = {}",
            proof_id.task_index
        );

        // write to proof store
        proof_store.set_proof_by_id(proof_id, &proof).await?;
        info!("✅ wrote guta result to proof store");
        checkpoint_queue.cdq_push_imm(queue_item.clone()).await?;
        info!("✅ wrote guta result to proof store end");
        let metadata = queue_item.get_dq_metadata();
        let items: Vec<SubmitGUTARealmResultAPIQueueItem<GoldilocksField>> =
            checkpoint_queue.cdq_peek_imm(metadata.channel_id).await?;
        debug!(
            "Retrieved GUTA queue items: {} items, metadata: {:#?}",
            items.len(),
            metadata
        );

        Ok(())
    }

    pub async fn build_block(&self) -> anyhow::Result<()> {
        let latest = self.get_latest_checkpoint_id().await?;
        let next_checkpoint = latest + 1;

        // Use CheckpointHistoryQueue instead of RedisQueue
        self.history_queue
            .produce_item(CEQueueNotification::StartProduceBlock { next_checkpoint })
            .await?;

        info!("☎️ build block {} cmd have send to CP", next_checkpoint);
        Ok(())
    }

    pub async fn get_checkpoint_sync_info(
        &self,
        realm_id: Option<u32>,
        request_checkpoint_id: u64,
    ) -> anyhow::Result<CheckpointSyncInfo<F>> {
        let latest = self.get_latest_checkpoint_id().await?;

        if request_checkpoint_id > latest {
            bail!(
                "Requested checkpoint_id {} exceeds latest local checkpoint_id {}",
                request_checkpoint_id,
                latest
            );
        }

        let compact = self
            .store
            .get_checkpoint_sync_info_compact(request_checkpoint_id)
            .await?;

        let is_pack_guta = if let Some(realm_id) = realm_id {
                Some(self.verify_realm_guta_production(realm_id as u64, request_checkpoint_id).await?)
            } else {
                None
            };

        // Convert compact to full sync info with all required fields
        let sync_info = CheckpointSyncInfo {
            latest_checkpoint_id: latest,
            description: None,
            source_coordinator_edge_id: None,
            sync_timestamp: Utc::now().timestamp() as u64,
            compact,
            is_pack_guta,
        };
        Ok(sync_info)
    }
    // async fn get_contract_leaf_data(&self, contract_id: u64) -> anyhow::Result<QEDContractLeaf<F>>;
    pub async fn get_contract_leaf_data(
        &self,
        contract_id: u64,
    ) -> anyhow::Result<QEDContractLeaf<QEDFelt>> {
        QEDCoordinatorStoreReaderAsync::get_contract_leaf_data(&*self.store, contract_id).await
    }
    // async fn get_contract_leaf_data_f(&self, contract_id: F) -> anyhow::Result<QEDContractLeaf<F>>;
    pub async fn get_contract_leaf_data_f(
        &self,
        contract_id: F,
    ) -> anyhow::Result<QEDContractLeaf<F>> {
        QEDCoordinatorStoreReaderAsync::get_contract_leaf_data_f(&*self.store, contract_id).await
    }
    // async fn get_checkpoint_leaf_data(&self, checkpoint_id: u64) -> anyhow::Result<QEDCheckpointLeaf<F>>;
    pub async fn get_checkpoint_leaf_data(
        &self,
        checkpoint_id: u64,
    ) -> anyhow::Result<QEDCheckpointLeaf<QEDFelt>> {
        QEDCoordinatorStoreReaderAsync::get_checkpoint_leaf_data(&*self.store, checkpoint_id).await
    }
    // async fn get_checkpoint_leaf_data_f(&self, checkpoint_id: F) -> anyhow::Result<QEDCheckpointLeaf<F>>;
    pub async fn get_checkpoint_leaf_data_f(
        &self,
        checkpoint_id: F,
    ) -> anyhow::Result<QEDCheckpointLeaf<F>> {
        QEDCoordinatorStoreReaderAsync::get_checkpoint_leaf_data_f(&*self.store, checkpoint_id)
            .await
    }
    // async fn get_contract_code_definition(&self, contract_id: u64) -> anyhow::Result<ContractCodeDefinition>;
    pub async fn get_contract_code_definition(
        &self,
        contract_id: u64,
    ) -> anyhow::Result<ContractCodeDefinition> {
        QEDCoordinatorStoreReaderAsync::get_contract_code_definition(&*self.store, contract_id)
            .await
    }
    // async fn get_contract_code_definition_f(&self, contract_id: F) -> anyhow::Result<ContractCodeDefinition>;
    pub async fn get_contract_code_definition_f(
        &self,
        contract_id: F,
    ) -> anyhow::Result<ContractCodeDefinition> {
        QEDCoordinatorStoreReaderAsync::get_contract_code_definition_f(&*self.store, contract_id)
            .await
    }
    // async fn get_latest_l2_block_state(&self) -> anyhow::Result<QEDL2BlockState>;
    pub async fn get_latest_l2_block_state(&self) -> anyhow::Result<QEDL2BlockState> {
        QEDCoordinatorStoreReaderAsync::get_latest_l2_block_state(&*self.store).await
    }
    // async fn get_l2_block_state(&self, checkpoint_id: u64) -> anyhow::Result<QEDL2BlockState>;
    pub async fn get_l2_block_state(&self, checkpoint_id: u64) -> anyhow::Result<QEDL2BlockState> {
        QEDCoordinatorStoreReaderAsync::get_l2_block_state(&*self.store, checkpoint_id).await
    }
    // async fn get_l2_block_state_f(&self, checkpoint_id: F) -> anyhow::Result<QEDL2BlockState>;
    pub async fn get_l2_block_state_f(&self, checkpoint_id: F) -> anyhow::Result<QEDL2BlockState> {
        QEDCoordinatorStoreReaderAsync::get_l2_block_state_f(&*self.store, checkpoint_id).await
    }
    // async fn get_user_registration_tree_root(&self, checkpoint_id: u64) -> anyhow::Result<QHashOut<F>>;
    pub async fn get_user_registration_tree_root(
        &self,
        checkpoint_id: u64,
    ) -> anyhow::Result<QHashOut<F>> {
        QEDCoordinatorStoreReaderAsync::get_user_registration_tree_root(&*self.store, checkpoint_id)
            .await
    }
    // async fn get_user_registration_tree_root_f(&self, checkpoint_id: F) -> anyhow::Result<QHashOut<F>>;
    pub async fn get_user_registration_tree_root_f(
        &self,
        checkpoint_id: F,
    ) -> anyhow::Result<QHashOut<F>> {
        QEDCoordinatorStoreReaderAsync::get_user_registration_tree_root_f(
            &*self.store,
            checkpoint_id,
        )
        .await
    }
    // async fn get_user_registration_tree_leaf_hash(&self, checkpoint_id: u64, leaf_index: u64) -> anyhow::Result<QHashOut<F>>;
    pub async fn get_user_registration_tree_leaf_hash(
        &self,
        checkpoint_id: u64,
        leaf_index: u64,
    ) -> anyhow::Result<QHashOut<F>> {
        QEDCoordinatorStoreReaderAsync::get_user_registration_tree_leaf_hash(
            &*self.store,
            checkpoint_id,
            leaf_index,
        )
        .await
    }
    // async fn get_user_registration_tree_leaf_hash_f(&self, checkpoint_id: F, leaf_index: F) -> anyhow::Result<QHashOut<F>>;
    pub async fn get_user_registration_tree_leaf_hash_f(
        &self,
        checkpoint_id: F,
        leaf_index: F,
    ) -> anyhow::Result<QHashOut<F>> {
        QEDCoordinatorStoreReaderAsync::get_user_registration_tree_leaf_hash_f(
            &*self.store,
            checkpoint_id,
            leaf_index,
        )
        .await
    }
    // async fn get_user_registration_tree_merkle_proof(&self, checkpoint_id: u64, leaf_index: u64) -> anyhow::Result<MerkleProofCore<QHashOut<F>>>;
    pub async fn get_user_registration_tree_merkle_proof(
        &self,
        checkpoint_id: u64,
        leaf_index: u64,
    ) -> anyhow::Result<MerkleProofCore<QHashOut<F>>> {
        QEDCoordinatorStoreReaderAsync::get_user_registration_tree_merkle_proof(
            &*self.store,
            checkpoint_id,
            leaf_index,
        )
        .await
    }
    // async fn get_user_registration_tree_merkle_proof_f(&self, checkpoint_id: F, leaf_index: F) -> anyhow::Result<MerkleProofCore<QHashOut<F>>>;
    pub async fn get_user_registration_tree_merkle_proof_f(
        &self,
        checkpoint_id: F,
        leaf_index: F,
    ) -> anyhow::Result<MerkleProofCore<QHashOut<F>>> {
        QEDCoordinatorStoreReaderAsync::get_user_registration_tree_merkle_proof_f(
            &*self.store,
            checkpoint_id,
            leaf_index,
        )
        .await
    }
    // async fn get_user_tree_root(&self, checkpoint_id: u64) -> anyhow::Result<QHashOut<F>>;
    pub async fn get_user_tree_root(&self, checkpoint_id: u64) -> anyhow::Result<QHashOut<F>> {
        QEDCoordinatorStoreReaderAsync::get_user_tree_root(&*self.store, checkpoint_id).await
    }
    // async fn get_user_tree_root_f(&self, checkpoint_id: F) -> anyhow::Result<QHashOut<F>>;
    pub async fn get_user_tree_root_f(&self, checkpoint_id: F) -> anyhow::Result<QHashOut<F>> {
        QEDCoordinatorStoreReaderAsync::get_user_tree_root_f(&*self.store, checkpoint_id).await
    }
    // async fn get_user_sub_tree_merkle_proof(&self, checkpoint_id: u64, root_level: u8, leaf_level: u8, leaf_index: u64) -> anyhow::Result<MerkleProofCore<QHashOut<F>>>;
    pub async fn get_user_sub_tree_merkle_proof(
        &self,
        checkpoint_id: u64,
        root_level: u8,
        leaf_level: u8,
        leaf_index: u64,
    ) -> anyhow::Result<MerkleProofCore<QHashOut<F>>> {
        QEDCoordinatorStoreReaderAsync::get_user_sub_tree_merkle_proof(
            &*self.store,
            checkpoint_id,
            root_level,
            leaf_level,
            leaf_index,
        )
        .await
    }
    // async fn get_user_top_tree_merkle_proof(&self, checkpoint_id: u64, leaf_level: u8, leaf_index: u64) -> anyhow::Result<MerkleProofCore<QHashOut<F>>>;
    pub async fn get_user_top_tree_merkle_proof(
        &self,
        checkpoint_id: u64,
        leaf_level: u8,
        leaf_index: u64,
    ) -> anyhow::Result<MerkleProofCore<QHashOut<F>>> {
        QEDCoordinatorStoreReaderAsync::get_user_top_tree_merkle_proof(
            &*self.store,
            checkpoint_id,
            leaf_level,
            leaf_index,
        )
        .await
    }
    // async fn get_user_top_tree_cap_root(&self, checkpoint_id: u64, cap_level: u8, cap_index: u64) -> anyhow::Result<QHashOut<F>>;
    pub async fn get_user_top_tree_cap_root(
        &self,
        checkpoint_id: u64,
        cap_level: u8,
        cap_index: u64,
    ) -> anyhow::Result<QHashOut<F>> {
        QEDCoordinatorStoreReaderAsync::get_user_top_tree_cap_root(
            &*self.store,
            checkpoint_id,
            cap_level,
            cap_index,
        )
        .await
    }
    // async fn get_user_latest_top_tree_cap_root(&self, cap_level: u8, cap_index: u64) -> anyhow::Result<QHashOut<F>>;
    pub async fn get_user_latest_top_tree_cap_root(
        &self,
        cap_level: u8,
        cap_index: u64,
    ) -> anyhow::Result<QHashOut<F>> {
        QEDCoordinatorStoreReaderAsync::get_user_latest_top_tree_cap_root(
            &*self.store,
            cap_level,
            cap_index,
        )
        .await
    }
    // async fn get_contract_function_tree_root(&self, checkpoint_id: u64, contract_id: u32) -> anyhow::Result<QHashOut<F>>;
    pub async fn get_contract_function_tree_root(
        &self,
        checkpoint_id: u64,
        contract_id: u32,
    ) -> anyhow::Result<QHashOut<F>> {
        QEDCoordinatorStoreReaderAsync::get_contract_function_tree_root(
            &*self.store,
            checkpoint_id,
            contract_id,
        )
        .await
    }
    // async fn get_contract_function_tree_root_f(&self, checkpoint_id: F, contract_id: F) -> anyhow::Result<QHashOut<F>>;
    pub async fn get_contract_function_tree_root_f(
        &self,
        checkpoint_id: F,
        contract_id: F,
    ) -> anyhow::Result<QHashOut<F>> {
        QEDCoordinatorStoreReaderAsync::get_contract_function_tree_root_f(
            &*self.store,
            checkpoint_id,
            contract_id,
        )
        .await
    }
    // async fn get_contract_function_tree_leaf_hash(&self, checkpoint_id: u64, contract_id: u32, function_id: u32) -> anyhow::Result<QHashOut<F>>;
    pub async fn get_contract_function_tree_leaf_hash(
        &self,
        checkpoint_id: u64,
        contract_id: u32,
        function_id: u32,
    ) -> anyhow::Result<QHashOut<F>> {
        QEDCoordinatorStoreReaderAsync::get_contract_function_tree_leaf_hash(
            &*self.store,
            checkpoint_id,
            contract_id,
            function_id,
        )
        .await
    }
    // async fn get_contract_function_tree_leaf_hash_f(&self, checkpoint_id: F, contract_id: F, function_id: F) -> anyhow::Result<QHashOut<F>>;
    pub async fn get_contract_function_tree_leaf_hash_f(
        &self,
        checkpoint_id: F,
        contract_id: F,
        function_id: F,
    ) -> anyhow::Result<QHashOut<F>> {
        QEDCoordinatorStoreReaderAsync::get_contract_function_tree_leaf_hash_f(
            &*self.store,
            checkpoint_id,
            contract_id,
            function_id,
        )
        .await
    }
    // async fn get_contract_function_tree_merkle_proof(&self, checkpoint_id: u64, contract_id: u32, function_id: u32) -> anyhow::Result<MerkleProofCore<QHashOut<F>>>;
    pub async fn get_contract_function_tree_merkle_proof(
        &self,
        checkpoint_id: u64,
        contract_id: u32,
        function_id: u32,
    ) -> anyhow::Result<MerkleProofCore<QHashOut<F>>> {
        QEDCoordinatorStoreReaderAsync::get_contract_function_tree_merkle_proof(
            &*self.store,
            checkpoint_id,
            contract_id,
            function_id,
        )
        .await
    }
    // async fn get_contract_function_tree_merkle_proof_f(&self, checkpoint_id: F, contract_id: F, function_id: F) -> anyhow::Result<MerkleProofCore<QHashOut<F>>>;
    pub async fn get_contract_function_tree_merkle_proof_f(
        &self,
        checkpoint_id: F,
        contract_id: F,
        function_id: F,
    ) -> anyhow::Result<MerkleProofCore<QHashOut<F>>> {
        QEDCoordinatorStoreReaderAsync::get_contract_function_tree_merkle_proof_f(
            &*self.store,
            checkpoint_id,
            contract_id,
            function_id,
        )
        .await
    }
    // async fn get_contract_tree_root(&self, checkpoint_id: u64) -> anyhow::Result<QHashOut<F>>;
    pub async fn get_contract_tree_root(&self, checkpoint_id: u64) -> anyhow::Result<QHashOut<F>> {
        QEDCoordinatorStoreReaderAsync::get_contract_tree_root(&*self.store, checkpoint_id).await
    }
    // async fn get_contract_tree_root_f(&self, checkpoint_id: F) -> anyhow::Result<QHashOut<F>>;
    pub async fn get_contract_tree_root_f(&self, checkpoint_id: F) -> anyhow::Result<QHashOut<F>> {
        QEDCoordinatorStoreReaderAsync::get_contract_tree_root_f(&*self.store, checkpoint_id).await
    }
    // async fn get_contract_tree_leaf_hash(&self, checkpoint_id: u64, contract_id: u32) -> anyhow::Result<QHashOut<F>>;
    pub async fn get_contract_tree_leaf_hash(
        &self,
        checkpoint_id: u64,
        contract_id: u32,
    ) -> anyhow::Result<QHashOut<F>> {
        QEDCoordinatorStoreReaderAsync::get_contract_tree_leaf_hash(
            &*self.store,
            checkpoint_id,
            contract_id,
        )
        .await
    }
    // async fn get_contract_tree_leaf_hash_f(&self, checkpoint_id: F, contract_id: F) -> anyhow::Result<QHashOut<F>>;
    pub async fn get_contract_tree_leaf_hash_f(
        &self,
        checkpoint_id: F,
        contract_id: F,
    ) -> anyhow::Result<QHashOut<F>> {
        QEDCoordinatorStoreReaderAsync::get_contract_tree_leaf_hash_f(
            &*self.store,
            checkpoint_id,
            contract_id,
        )
        .await
    }
    // async fn get_contract_tree_merkle_proof(&self, checkpoint_id: u64, contract_id: u32) -> anyhow::Result<MerkleProofCore<QHashOut<F>>>;
    pub async fn get_contract_tree_merkle_proof(
        &self,
        checkpoint_id: u64,
        contract_id: u32,
    ) -> anyhow::Result<MerkleProofCore<QHashOut<F>>> {
        QEDCoordinatorStoreReaderAsync::get_contract_tree_merkle_proof(
            &*self.store,
            checkpoint_id,
            contract_id,
        )
        .await
    }
    // async fn get_contract_tree_merkle_proof_f(&self, checkpoint_id: F, contract_id: F) -> anyhow::Result<MerkleProofCore<QHashOut<F>>>;
    pub async fn get_contract_tree_merkle_proof_f(
        &self,
        checkpoint_id: F,
        contract_id: F,
    ) -> anyhow::Result<MerkleProofCore<QHashOut<F>>> {
        QEDCoordinatorStoreReaderAsync::get_contract_tree_merkle_proof_f(
            &*self.store,
            checkpoint_id,
            contract_id,
        )
        .await
    }
    // async fn get_deposit_tree_root(&self, checkpoint_id: u64) -> anyhow::Result<QHashOut<F>>;
    pub async fn get_deposit_tree_root(&self, checkpoint_id: u64) -> anyhow::Result<QHashOut<F>> {
        QEDCoordinatorStoreReaderAsync::get_deposit_tree_root(&*self.store, checkpoint_id).await
    }
    // async fn get_deposit_tree_root_f(&self, checkpoint_id: F) -> anyhow::Result<QHashOut<F>>;
    pub async fn get_deposit_tree_root_f(&self, checkpoint_id: F) -> anyhow::Result<QHashOut<F>> {
        QEDCoordinatorStoreReaderAsync::get_deposit_tree_root_f(&*self.store, checkpoint_id).await
    }
    // async fn get_deposit_tree_leaf_hash(&self, checkpoint_id: u64, deposit_id: u32) -> anyhow::Result<QHashOut<F>>;
    pub async fn get_deposit_tree_leaf_hash(
        &self,
        checkpoint_id: u64,
        deposit_id: u32,
    ) -> anyhow::Result<QHashOut<F>> {
        QEDCoordinatorStoreReaderAsync::get_deposit_tree_leaf_hash(
            &*self.store,
            checkpoint_id,
            deposit_id,
        )
        .await
    }
    // async fn get_deposit_tree_leaf_hash_f(&self, checkpoint_id: F, deposit_id: F) -> anyhow::Result<QHashOut<F>>;
    pub async fn get_deposit_tree_leaf_hash_f(
        &self,
        checkpoint_id: F,
        deposit_id: F,
    ) -> anyhow::Result<QHashOut<F>> {
        QEDCoordinatorStoreReaderAsync::get_deposit_tree_leaf_hash_f(
            &*self.store,
            checkpoint_id,
            deposit_id,
        )
        .await
    }
    // async fn get_deposit_tree_merkle_proof(&self, checkpoint_id: u64, deposit_id: u32) -> anyhow::Result<MerkleProofCore<QHashOut<F>>>;
    pub async fn get_deposit_tree_merkle_proof(
        &self,
        checkpoint_id: u64,
        deposit_id: u32,
    ) -> anyhow::Result<MerkleProofCore<QHashOut<F>>> {
        QEDCoordinatorStoreReaderAsync::get_deposit_tree_merkle_proof(
            &*self.store,
            checkpoint_id,
            deposit_id,
        )
        .await
    }
    // async fn get_deposit_tree_merkle_proof_f(&self, checkpoint_id: F, deposit_id: F) -> anyhow::Result<MerkleProofCore<QHashOut<F>>>;
    pub async fn get_deposit_tree_merkle_proof_f(
        &self,
        checkpoint_id: F,
        deposit_id: F,
    ) -> anyhow::Result<MerkleProofCore<QHashOut<F>>> {
        QEDCoordinatorStoreReaderAsync::get_deposit_tree_merkle_proof_f(
            &*self.store,
            checkpoint_id,
            deposit_id,
        )
        .await
    }
    // async fn get_withdrawal_tree_root(&self, checkpoint_id: u64) -> anyhow::Result<QHashOut<F>>;
    pub async fn get_withdrawal_tree_root(
        &self,
        checkpoint_id: u64,
    ) -> anyhow::Result<QHashOut<F>> {
        QEDCoordinatorStoreReaderAsync::get_withdrawal_tree_root(&*self.store, checkpoint_id).await
    }
    // async fn get_withdrawal_tree_root_f(&self, checkpoint_id: F) -> anyhow::Result<QHashOut<F>>;
    pub async fn get_withdrawal_tree_root_f(
        &self,
        checkpoint_id: F,
    ) -> anyhow::Result<QHashOut<F>> {
        QEDCoordinatorStoreReaderAsync::get_withdrawal_tree_root_f(&*self.store, checkpoint_id)
            .await
    }
    // async fn get_withdrawal_tree_leaf_hash(&self, checkpoint_id: u64, withdrawal_id: u32) -> anyhow::Result<QHashOut<F>>;
    pub async fn get_withdrawal_tree_leaf_hash(
        &self,
        checkpoint_id: u64,
        withdrawal_id: u32,
    ) -> anyhow::Result<QHashOut<F>> {
        QEDCoordinatorStoreReaderAsync::get_withdrawal_tree_leaf_hash(
            &*self.store,
            checkpoint_id,
            withdrawal_id,
        )
        .await
    }
    // async fn get_withdrawal_tree_leaf_hash_f(&self, checkpoint_id: F, withdrawal_id: F) -> anyhow::Result<QHashOut<F>>;
    pub async fn get_withdrawal_tree_leaf_hash_f(
        &self,
        checkpoint_id: F,
        withdrawal_id: F,
    ) -> anyhow::Result<QHashOut<F>> {
        QEDCoordinatorStoreReaderAsync::get_withdrawal_tree_leaf_hash_f(
            &*self.store,
            checkpoint_id,
            withdrawal_id,
        )
        .await
    }
    // async fn get_withdrawal_tree_merkle_proof(&self, checkpoint_id: u64, withdrawal_id: u32) -> anyhow::Result<MerkleProofCore<QHashOut<F>>>;
    pub async fn get_withdrawal_tree_merkle_proof(
        &self,
        checkpoint_id: u64,
        withdrawal_id: u32,
    ) -> anyhow::Result<MerkleProofCore<QHashOut<F>>> {
        QEDCoordinatorStoreReaderAsync::get_withdrawal_tree_merkle_proof(
            &*self.store,
            checkpoint_id,
            withdrawal_id,
        )
        .await
    }
    // async fn get_withdrawal_tree_merkle_proof_f(&self, checkpoint_id: F, withdrawal_id: F) -> anyhow::Result<MerkleProofCore<QHashOut<F>>>;
    pub async fn get_withdrawal_tree_merkle_proof_f(
        &self,
        checkpoint_id: F,
        withdrawal_id: F,
    ) -> anyhow::Result<MerkleProofCore<QHashOut<F>>> {
        QEDCoordinatorStoreReaderAsync::get_withdrawal_tree_merkle_proof_f(
            &*self.store,
            checkpoint_id,
            withdrawal_id,
        )
        .await
    }
    // async fn get_latest_checkpoint_tree_root(&self) -> anyhow::Result<QHashOut<F>>;
    pub async fn get_latest_checkpoint_tree_root(&self) -> anyhow::Result<QHashOut<F>> {
        QEDCoordinatorStoreReaderAsync::get_latest_checkpoint_tree_root(&*self.store).await
    }
    // async fn get_checkpoint_tree_root(&self, checkpoint_id: u64) -> anyhow::Result<QHashOut<F>>;
    pub async fn get_checkpoint_tree_root(
        &self,
        checkpoint_id: u64,
    ) -> anyhow::Result<QHashOut<F>> {
        QEDCoordinatorStoreReaderAsync::get_checkpoint_tree_root(&*self.store, checkpoint_id).await
    }
    // async fn get_checkpoint_tree_root_f(&self, checkpoint_id: F) -> anyhow::Result<QHashOut<F>>;
    pub async fn get_checkpoint_tree_root_f(
        &self,
        checkpoint_id: F,
    ) -> anyhow::Result<QHashOut<F>> {
        QEDCoordinatorStoreReaderAsync::get_checkpoint_tree_root_f(&*self.store, checkpoint_id)
            .await
    }
    // async fn get_checkpoint_tree_leaf_hash(&self, checkpoint_id: u64, leaf_checkpoint_id: u64) -> anyhow::Result<QHashOut<F>>;
    pub async fn get_checkpoint_tree_leaf_hash(
        &self,
        checkpoint_id: u64,
        leaf_checkpoint_id: u64,
    ) -> anyhow::Result<QHashOut<F>> {
        QEDCoordinatorStoreReaderAsync::get_checkpoint_tree_leaf_hash(
            &*self.store,
            checkpoint_id,
            leaf_checkpoint_id,
        )
        .await
    }
    // async fn get_checkpoint_tree_leaf_hash_f(&self, checkpoint_id: F, leaf_checkpoint_id: F) -> anyhow::Result<QHashOut<F>>;
    pub async fn get_checkpoint_tree_leaf_hash_f(
        &self,
        checkpoint_id: F,
        leaf_checkpoint_id: F,
    ) -> anyhow::Result<QHashOut<F>> {
        QEDCoordinatorStoreReaderAsync::get_checkpoint_tree_leaf_hash_f(
            &*self.store,
            checkpoint_id,
            leaf_checkpoint_id,
        )
        .await
    }
    // async fn get_checkpoint_tree_merkle_proof(&self, checkpoint_id: u64, leaf_checkpoint_id: u64) -> anyhow::Result<MerkleProofCore<QHashOut<F>>>;
    pub async fn get_checkpoint_tree_merkle_proof(
        &self,
        checkpoint_id: u64,
        leaf_checkpoint_id: u64,
    ) -> anyhow::Result<MerkleProofCore<QHashOut<F>>> {
        QEDCoordinatorStoreReaderAsync::get_checkpoint_tree_merkle_proof(
            &*self.store,
            checkpoint_id,
            leaf_checkpoint_id,
        )
        .await
    }
    // async fn get_checkpoint_tree_merkle_proof_f(&self, checkpoint_id: F, leaf_checkpoint_id: F) -> anyhow::Result<MerkleProofCore<QHashOut<F>>>;
    pub async fn get_checkpoint_tree_merkle_proof_f(
        &self,
        checkpoint_id: F,
        leaf_checkpoint_id: F,
    ) -> anyhow::Result<MerkleProofCore<QHashOut<F>>> {
        QEDCoordinatorStoreReaderAsync::get_checkpoint_tree_merkle_proof_f(
            &*self.store,
            checkpoint_id,
            leaf_checkpoint_id,
        )
        .await
    }
    // async fn get_checkpoint_global_state_roots(&self, checkpoint_id: u64) -> anyhow::Result<QEDCheckpointGlobalStateRoots<F>>;
    pub async fn get_checkpoint_global_state_roots(
        &self,
        checkpoint_id: u64,
    ) -> anyhow::Result<QEDCheckpointGlobalStateRoots<QEDFelt>> {
        QEDCoordinatorStoreReaderAsync::get_checkpoint_global_state_roots(
            &*self.store,
            checkpoint_id,
        )
        .await
    }
    // async fn get_checkpoint_sync_info_compact(&self, checkpoint_id: u64) -> anyhow::Result<QEDCheckpointSyncInfoCompact<F>>;
    pub async fn get_checkpoint_sync_info_compact(
        &self,
        checkpoint_id: u64,
    ) -> anyhow::Result<QEDCheckpointSyncInfoCompact<QEDFelt>> {
        QEDCoordinatorStoreReaderAsync::get_checkpoint_sync_info_compact(
            &*self.store,
            checkpoint_id,
        )
        .await
    }

    pub async fn get_user_leaf_data(
        &self,
        checkpoint_id: u64,
        user_id: u64,
    ) -> anyhow::Result<QEDUserLeaf<QEDFelt>> {
        self.store.get_user_leaf_data(checkpoint_id, user_id)
    }
    pub async fn get_user_tree_merkle_proof(
        &self,
        checkpoint_id: u64,
        user_id: u64,
    ) -> anyhow::Result<MerkleProofCore<QHashOut<QEDFelt>>> {
        self.store
            .get_user_tree_merkle_proof(checkpoint_id, user_id)
    }

    pub async fn get_user_tree_merkle_proof_f(
        &self,
        checkpoint_id: F,
        user_id: F,
    ) -> anyhow::Result<MerkleProofCore<QHashOut<QEDFelt>>> {
        self.store
            .get_user_tree_merkle_proof_f(checkpoint_id, user_id)
    }

    async fn log_suspicious_activity(&self, job: &QJob, reason: &str) {
        //todo! add some operation to log suspicious activity or ban user
        error!(
            "🚨 SECURITY ALERT: Invalid job submission - Reason: {}, Job: {:?}, Layer: {}, MsgId: {}",
            reason, job.job_id, job.layer_id, job.msg_id
        );
    }

    /// Verify if a realm produced GUTA for a specific checkpoint
    pub async fn verify_realm_guta_production(
        &self,
        realm_id: u64,
        checkpoint_id: u64,
    ) -> anyhow::Result<bool> {
        let realm_merkle_proof = self
            .get_user_top_tree_merkle_proof(
                checkpoint_id,
                self.ctx.coordinator_config.realm_root_level,
                realm_id,
            )
            .await?;
        if !realm_merkle_proof.verify::<QEDHasher>() {
            return Ok(false);
        }
        let has_guta = realm_merkle_proof.value != QHashOut::<QEDFelt>::ZERO;
        Ok(has_guta)
    }
}

use super::error::RpcError;
use super::rpc::CoordinatorEdgeRpcServer;
use super::types::LatestCheckpointResponse;
use crate::common::whitelist::WhiteList;
use async_trait::async_trait;
use jsonrpsee::core::RpcResult;
use plonky2::field::goldilocks_field::GoldilocksField;
use qed_prover::local::request::{QDeployContractRPCRequest, QRegisterUserRPCRequest};
use qed_prover::wallet::secp_sign::SignedRequest;
use qed_store::queue::redis_queue::NotificationQueue;
use qed_store::queue::task_queue::{
    JobValidationStatus, QJob, QProvingTaskStore, QProvingTaskStoreImpl,
};

#[async_trait]
impl CoordinatorEdgeRpcServer for CoordinatorEdgeHandler {
    async fn register_user(&self, public_key: ZKPublicKeyInfo<F>) -> RpcResult<String> {
        self.register_user(public_key)
            .await
            .map(|_| "ok".to_string())
            .map_err(|e| RpcError::Anyhow(e.into()))
    }

    async fn get_user_id(&self, public_key: QHashOut<F>) -> RpcResult<u64> {
        self.get_user_id(public_key)
            .await
            .map_err(|e| RpcError::Anyhow(e.into()))
    }

    async fn deploy_contract(&self, deploy_contract: QBCDeployContract<F>) -> RpcResult<String> {
        self.deploy_contract(deploy_contract)
            .await
            .map(|_| "ok".to_string())
            .map_err(RpcError::Anyhow)
    }

    async fn build_block(&self) -> RpcResult<String> {
        self.build_block()
            .await
            .map(|_| "ok".to_string())
            .map_err(RpcError::Anyhow)
    }

    async fn submit_guta(
        &self,
        input: SubmitGUTARealmResultAPINoProofInput<F>,
        proof: ProofWithPublicInputs<F, C, D>,
    ) -> RpcResult<String> {
        self.submit_guta(input, proof)
            .await
            .map(|_| "ok".to_string())
            .map_err(RpcError::Anyhow)
    }

    async fn get_latest_checkpoint(&self) -> RpcResult<LatestCheckpointResponse> {
        let checkpoint_id = self
            .get_latest_checkpoint_id()
            .await
            .map_err(RpcError::Anyhow)?;
        Ok(LatestCheckpointResponse { checkpoint_id })
    }

    async fn latest_checkpoint(&self) -> RpcResult<u64> {
        self.get_latest_checkpoint_id()
            .await
            .map_err(RpcError::Anyhow)
    }

    async fn get_latest_checkpoint_id(&self) -> RpcResult<u64> {
        self.get_latest_checkpoint_id()
            .await
            .map_err(RpcError::Anyhow)
    }

    async fn get_checkpoint_sync_info(
        &self,
        realm_id: Option<u32>,
        checkpoint_id: u64,
    ) -> RpcResult<CheckpointSyncInfo<F>> {
        self.get_checkpoint_sync_info(realm_id, checkpoint_id)
            .await
            .map_err(RpcError::Anyhow)
    }

    async fn get_checkpoint_sync_info_compact(
        &self,
        checkpoint_id: u64,
    ) -> RpcResult<QCheckpointSyncInfoCompact> {
        self.get_checkpoint_sync_info_compact(checkpoint_id)
            .await
            .map_err(RpcError::Anyhow)
    }

    async fn get_contract_leaf_data(&self, contract_id: u64) -> RpcResult<QEDContractLeaf<F>> {
        self.get_contract_leaf_data(contract_id)
            .await
            .map_err(RpcError::Anyhow)
    }

    async fn get_contract_leaf_data_f(&self, contract_id: F) -> RpcResult<QEDContractLeaf<F>> {
        self.get_contract_leaf_data_f(contract_id)
            .await
            .map_err(RpcError::Anyhow)
    }

    async fn get_contract_code_definition(
        &self,
        contract_id: u64,
    ) -> RpcResult<ContractCodeDefinition> {
        self.get_contract_code_definition(contract_id)
            .await
            .map_err(RpcError::Anyhow)
    }

    async fn get_contract_code_definition_f(
        &self,
        contract_id: F,
    ) -> RpcResult<ContractCodeDefinition> {
        self.get_contract_code_definition_f(contract_id)
            .await
            .map_err(RpcError::Anyhow)
    }

    async fn get_checkpoint_leaf_data(
        &self,
        checkpoint_id: u64,
    ) -> RpcResult<QEDCheckpointLeaf<F>> {
        self.get_checkpoint_leaf_data(checkpoint_id)
            .await
            .map_err(RpcError::Anyhow)
    }

    async fn get_checkpoint_leaf_data_f(
        &self,
        checkpoint_id: F,
    ) -> RpcResult<QEDCheckpointLeaf<F>> {
        self.get_checkpoint_leaf_data_f(checkpoint_id)
            .await
            .map_err(RpcError::Anyhow)
    }

    async fn get_checkpoint_global_state_roots(
        &self,
        checkpoint_id: u64,
    ) -> RpcResult<QEDCheckpointGlobalStateRoots<F>> {
        self.get_checkpoint_global_state_roots(checkpoint_id)
            .await
            .map_err(RpcError::Anyhow)
    }

    async fn get_latest_l2_block_state(&self) -> RpcResult<QEDL2BlockState> {
        self.get_latest_l2_block_state()
            .await
            .map_err(RpcError::Anyhow)
    }

    async fn get_l2_block_state(&self, checkpoint_id: u64) -> RpcResult<QEDL2BlockState> {
        self.get_l2_block_state(checkpoint_id)
            .await
            .map_err(RpcError::Anyhow)
    }

    async fn get_l2_block_state_f(&self, checkpoint_id: F) -> RpcResult<QEDL2BlockState> {
        self.get_l2_block_state_f(checkpoint_id)
            .await
            .map_err(RpcError::Anyhow)
    }

    async fn get_user_registration_tree_root(&self, checkpoint_id: u64) -> RpcResult<QHashOut<F>> {
        self.get_user_registration_tree_root(checkpoint_id)
            .await
            .map_err(RpcError::Anyhow)
    }

    async fn get_user_registration_tree_root_f(&self, checkpoint_id: F) -> RpcResult<QHashOut<F>> {
        self.get_user_registration_tree_root_f(checkpoint_id)
            .await
            .map_err(RpcError::Anyhow)
    }

    async fn get_user_registration_tree_leaf_hash(
        &self,
        checkpoint_id: u64,
        leaf_index: u64,
    ) -> RpcResult<QHashOut<F>> {
        self.get_user_registration_tree_leaf_hash(checkpoint_id, leaf_index)
            .await
            .map_err(RpcError::Anyhow)
    }

    async fn get_user_registration_tree_leaf_hash_f(
        &self,
        checkpoint_id: F,
        leaf_index: F,
    ) -> RpcResult<QHashOut<F>> {
        self.get_user_registration_tree_leaf_hash_f(checkpoint_id, leaf_index)
            .await
            .map_err(RpcError::Anyhow)
    }

    async fn get_user_registration_tree_merkle_proof(
        &self,
        checkpoint_id: u64,
        leaf_index: u64,
    ) -> RpcResult<MerkleProofCore<QHashOut<F>>> {
        self.get_user_registration_tree_merkle_proof(checkpoint_id, leaf_index)
            .await
            .map_err(RpcError::Anyhow)
    }

    async fn get_user_registration_tree_merkle_proof_f(
        &self,
        checkpoint_id: F,
        leaf_index: F,
    ) -> RpcResult<MerkleProofCore<QHashOut<F>>> {
        self.get_user_registration_tree_merkle_proof_f(checkpoint_id, leaf_index)
            .await
            .map_err(RpcError::Anyhow)
    }

    async fn get_user_tree_root(&self, checkpoint_id: u64) -> RpcResult<QHashOut<F>> {
        self.get_user_tree_root(checkpoint_id)
            .await
            .map_err(RpcError::Anyhow)
    }

    async fn get_user_tree_root_f(&self, checkpoint_id: F) -> RpcResult<QHashOut<F>> {
        self.get_user_tree_root_f(checkpoint_id)
            .await
            .map_err(RpcError::Anyhow)
    }

    async fn get_user_sub_tree_merkle_proof(
        &self,
        checkpoint_id: u64,
        root_level: u8,
        leaf_level: u8,
        leaf_index: u64,
    ) -> RpcResult<MerkleProofCore<QHashOut<F>>> {
        self.get_user_sub_tree_merkle_proof(checkpoint_id, root_level, leaf_level, leaf_index)
            .await
            .map_err(RpcError::Anyhow)
    }

    async fn get_user_top_tree_merkle_proof(
        &self,
        checkpoint_id: u64,
        leaf_level: u8,
        leaf_index: u64,
    ) -> RpcResult<MerkleProofCore<QHashOut<F>>> {
        self.get_user_top_tree_merkle_proof(checkpoint_id, leaf_level, leaf_index)
            .await
            .map_err(RpcError::Anyhow)
    }

    async fn get_user_top_tree_cap_root(
        &self,
        checkpoint_id: u64,
        cap_level: u8,
        cap_index: u64,
    ) -> RpcResult<QHashOut<F>> {
        self.get_user_top_tree_cap_root(checkpoint_id, cap_level, cap_index)
            .await
            .map_err(RpcError::Anyhow)
    }

    async fn get_user_latest_top_tree_cap_root(
        &self,
        cap_level: u8,
        cap_index: u64,
    ) -> RpcResult<QHashOut<F>> {
        self.get_user_latest_top_tree_cap_root(cap_level, cap_index)
            .await
            .map_err(RpcError::Anyhow)
    }

    async fn get_user_leaf_data(
        &self,
        checkpoint_id: u64,
        user_id: u64,
    ) -> RpcResult<QEDUserLeaf<F>> {
        self.get_user_leaf_data(checkpoint_id, user_id)
            .await
            .map_err(RpcError::Anyhow)
    }

    async fn get_user_tree_merkle_proof(
        &self,
        checkpoint_id: u64,
        user_id: u64,
    ) -> RpcResult<MerkleProofCore<QHashOut<F>>> {
        self.get_user_tree_merkle_proof(checkpoint_id, user_id)
            .await
            .map_err(RpcError::Anyhow)
    }

    async fn get_user_tree_merkle_proof_f(
        &self,
        checkpoint_id: F,
        user_id: F,
    ) -> RpcResult<MerkleProofCore<QHashOut<F>>> {
        self.get_user_tree_merkle_proof_f(checkpoint_id, user_id)
            .await
            .map_err(RpcError::Anyhow)
    }

    async fn get_contract_function_tree_root(
        &self,
        checkpoint_id: u64,
        contract_id: u32,
    ) -> RpcResult<QHashOut<F>> {
        self.get_contract_function_tree_root(checkpoint_id, contract_id)
            .await
            .map_err(RpcError::Anyhow)
    }

    async fn get_contract_function_tree_root_f(
        &self,
        checkpoint_id: F,
        contract_id: F,
    ) -> RpcResult<QHashOut<F>> {
        self.get_contract_function_tree_root_f(checkpoint_id, contract_id)
            .await
            .map_err(RpcError::Anyhow)
    }

    async fn get_contract_function_tree_leaf_hash(
        &self,
        checkpoint_id: u64,
        contract_id: u32,
        function_id: u32,
    ) -> RpcResult<QHashOut<F>> {
        self.get_contract_function_tree_leaf_hash(checkpoint_id, contract_id, function_id)
            .await
            .map_err(RpcError::Anyhow)
    }

    async fn get_contract_function_tree_leaf_hash_f(
        &self,
        checkpoint_id: F,
        contract_id: F,
        function_id: F,
    ) -> RpcResult<QHashOut<F>> {
        self.get_contract_function_tree_leaf_hash_f(checkpoint_id, contract_id, function_id)
            .await
            .map_err(RpcError::Anyhow)
    }

    async fn get_contract_function_tree_merkle_proof(
        &self,
        checkpoint_id: u64,
        contract_id: u32,
        function_id: u32,
    ) -> RpcResult<MerkleProofCore<QHashOut<F>>> {
        self.get_contract_function_tree_merkle_proof(checkpoint_id, contract_id, function_id)
            .await
            .map_err(RpcError::Anyhow)
    }

    async fn get_contract_function_tree_merkle_proof_f(
        &self,
        checkpoint_id: F,
        contract_id: F,
        function_id: F,
    ) -> RpcResult<MerkleProofCore<QHashOut<F>>> {
        self.get_contract_function_tree_merkle_proof_f(checkpoint_id, contract_id, function_id)
            .await
            .map_err(RpcError::Anyhow)
    }

    async fn get_contract_tree_root(&self, checkpoint_id: u64) -> RpcResult<QHashOut<F>> {
        self.get_contract_tree_root(checkpoint_id)
            .await
            .map_err(RpcError::Anyhow)
    }

    async fn get_contract_tree_root_f(&self, checkpoint_id: F) -> RpcResult<QHashOut<F>> {
        self.get_contract_tree_root_f(checkpoint_id)
            .await
            .map_err(RpcError::Anyhow)
    }

    async fn get_contract_tree_leaf_hash(
        &self,
        checkpoint_id: u64,
        contract_id: u32,
    ) -> RpcResult<QHashOut<F>> {
        self.get_contract_tree_leaf_hash(checkpoint_id, contract_id)
            .await
            .map_err(RpcError::Anyhow)
    }

    async fn get_contract_tree_leaf_hash_f(
        &self,
        checkpoint_id: F,
        contract_id: F,
    ) -> RpcResult<QHashOut<F>> {
        self.get_contract_tree_leaf_hash_f(checkpoint_id, contract_id)
            .await
            .map_err(RpcError::Anyhow)
    }

    async fn get_contract_tree_merkle_proof(
        &self,
        checkpoint_id: u64,
        contract_id: u32,
    ) -> RpcResult<MerkleProofCore<QHashOut<F>>> {
        self.get_contract_tree_merkle_proof(checkpoint_id, contract_id)
            .await
            .map_err(RpcError::Anyhow)
    }

    async fn get_contract_tree_merkle_proof_f(
        &self,
        checkpoint_id: F,
        contract_id: F,
    ) -> RpcResult<MerkleProofCore<QHashOut<F>>> {
        self.get_contract_tree_merkle_proof_f(checkpoint_id, contract_id)
            .await
            .map_err(RpcError::Anyhow)
    }

    async fn get_deposit_tree_root(&self, checkpoint_id: u64) -> RpcResult<QHashOut<F>> {
        self.get_deposit_tree_root(checkpoint_id)
            .await
            .map_err(RpcError::Anyhow)
    }

    async fn get_deposit_tree_root_f(&self, checkpoint_id: F) -> RpcResult<QHashOut<F>> {
        self.get_deposit_tree_root_f(checkpoint_id)
            .await
            .map_err(RpcError::Anyhow)
    }

    async fn get_deposit_tree_leaf_hash(
        &self,
        checkpoint_id: u64,
        deposit_id: u32,
    ) -> RpcResult<QHashOut<F>> {
        self.get_deposit_tree_leaf_hash(checkpoint_id, deposit_id)
            .await
            .map_err(RpcError::Anyhow)
    }

    async fn get_deposit_tree_leaf_hash_f(
        &self,
        checkpoint_id: F,
        deposit_id: F,
    ) -> RpcResult<QHashOut<F>> {
        self.get_deposit_tree_leaf_hash_f(checkpoint_id, deposit_id)
            .await
            .map_err(RpcError::Anyhow)
    }

    async fn get_deposit_tree_merkle_proof(
        &self,
        checkpoint_id: u64,
        deposit_id: u32,
    ) -> RpcResult<MerkleProofCore<QHashOut<F>>> {
        self.get_deposit_tree_merkle_proof(checkpoint_id, deposit_id)
            .await
            .map_err(RpcError::Anyhow)
    }

    async fn get_deposit_tree_merkle_proof_f(
        &self,
        checkpoint_id: F,
        deposit_id: F,
    ) -> RpcResult<MerkleProofCore<QHashOut<F>>> {
        self.get_deposit_tree_merkle_proof_f(checkpoint_id, deposit_id)
            .await
            .map_err(RpcError::Anyhow)
    }

    async fn get_withdrawal_tree_root(&self, checkpoint_id: u64) -> RpcResult<QHashOut<F>> {
        self.get_withdrawal_tree_root(checkpoint_id)
            .await
            .map_err(RpcError::Anyhow)
    }

    async fn get_withdrawal_tree_root_f(&self, checkpoint_id: F) -> RpcResult<QHashOut<F>> {
        self.get_withdrawal_tree_root_f(checkpoint_id)
            .await
            .map_err(RpcError::Anyhow)
    }

    async fn get_withdrawal_tree_leaf_hash(
        &self,
        checkpoint_id: u64,
        withdrawal_id: u32,
    ) -> RpcResult<QHashOut<F>> {
        self.get_withdrawal_tree_leaf_hash(checkpoint_id, withdrawal_id)
            .await
            .map_err(RpcError::Anyhow)
    }

    async fn get_withdrawal_tree_leaf_hash_f(
        &self,
        checkpoint_id: F,
        withdrawal_id: F,
    ) -> RpcResult<QHashOut<F>> {
        self.get_withdrawal_tree_leaf_hash_f(checkpoint_id, withdrawal_id)
            .await
            .map_err(RpcError::Anyhow)
    }

    async fn get_withdrawal_tree_merkle_proof(
        &self,
        checkpoint_id: u64,
        withdrawal_id: u32,
    ) -> RpcResult<MerkleProofCore<QHashOut<F>>> {
        self.get_withdrawal_tree_merkle_proof(checkpoint_id, withdrawal_id)
            .await
            .map_err(RpcError::Anyhow)
    }

    async fn get_withdrawal_tree_merkle_proof_f(
        &self,
        checkpoint_id: F,
        withdrawal_id: F,
    ) -> RpcResult<MerkleProofCore<QHashOut<F>>> {
        self.get_withdrawal_tree_merkle_proof_f(checkpoint_id, withdrawal_id)
            .await
            .map_err(RpcError::Anyhow)
    }

    async fn get_latest_checkpoint_tree_root(&self) -> RpcResult<QHashOut<F>> {
        self.get_latest_checkpoint_tree_root()
            .await
            .map_err(RpcError::Anyhow)
    }

    async fn get_checkpoint_tree_root(&self, checkpoint_id: u64) -> RpcResult<QHashOut<F>> {
        self.get_checkpoint_tree_root(checkpoint_id)
            .await
            .map_err(RpcError::Anyhow)
    }

    async fn get_checkpoint_tree_root_f(&self, checkpoint_id: F) -> RpcResult<QHashOut<F>> {
        self.get_checkpoint_tree_root_f(checkpoint_id)
            .await
            .map_err(RpcError::Anyhow)
    }

    async fn get_checkpoint_tree_leaf_hash(
        &self,
        checkpoint_id: u64,
        leaf_checkpoint_id: u64,
    ) -> RpcResult<QHashOut<F>> {
        self.get_checkpoint_tree_leaf_hash(checkpoint_id, leaf_checkpoint_id)
            .await
            .map_err(RpcError::Anyhow)
    }

    async fn get_checkpoint_tree_leaf_hash_f(
        &self,
        checkpoint_id: F,
        leaf_checkpoint_id: F,
    ) -> RpcResult<QHashOut<F>> {
        self.get_checkpoint_tree_leaf_hash_f(checkpoint_id, leaf_checkpoint_id)
            .await
            .map_err(RpcError::Anyhow)
    }

    async fn get_checkpoint_tree_merkle_proof(
        &self,
        checkpoint_id: u64,
        leaf_checkpoint_id: u64,
    ) -> RpcResult<MerkleProofCore<QHashOut<F>>> {
        self.get_checkpoint_tree_merkle_proof(checkpoint_id, leaf_checkpoint_id)
            .await
            .map_err(RpcError::Anyhow)
    }

    async fn get_checkpoint_tree_merkle_proof_f(
        &self,
        checkpoint_id: F,
        leaf_checkpoint_id: F,
    ) -> RpcResult<MerkleProofCore<QHashOut<F>>> {
        self.get_checkpoint_tree_merkle_proof_f(checkpoint_id, leaf_checkpoint_id)
            .await
            .map_err(RpcError::Anyhow)
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
                    format!(
                        "Job ID {:?} does not belong to checkpoint {}",
                        job_id, checkpoint_id
                    ),
                    None::<()>,
                ));
            }
        }

        let checkpoint_leaf = self
            .get_checkpoint_leaf_data(checkpoint_id)
            .await
            .map_err(|e| {
                ErrorObject::owned(
                    jsonrpsee::types::ErrorCode::InternalError.code(),
                    format!("Failed to get checkpoint data: {}", e),
                    None::<()>,
                )
            })?;

        let graph = self
            .task_store
            .load_job_dependency_graph(checkpoint_id)
            .await
            .map_err(|e| {
                ErrorObject::owned(
                    jsonrpsee::types::ErrorCode::InternalError.code(),
                    format!(
                        "Failed to load job dependency graph for checkpoint {}: {}",
                        checkpoint_id, e
                    ),
                    None::<()>,
                )
            })?;

        let mut proofs = Vec::new();

        for job_id in job_ids {
            let expected_root = match job_id.circuit_type {
                ProvingJobCircuitType::AppendUserRegistrationTree
                | ProvingJobCircuitType::AppendUserRegistrationTreeAggregate
                | ProvingJobCircuitType::DummyAppendUserRegistrationTreeAggregate => {
                    checkpoint_leaf
                        .stats
                        .pm_rewards_commitment
                        .register_users_root
                }
                ProvingJobCircuitType::GUTARegisterUsers
                | ProvingJobCircuitType::GUTAOnlyRegisterUsers
                | ProvingJobCircuitType::GUTATwoGUTA
                | ProvingJobCircuitType::GUTANoChange
                | ProvingJobCircuitType::GUTASingleEndCap
                | ProvingJobCircuitType::GUTATwoEndCap
                | ProvingJobCircuitType::GUTALeftEndCapRightGUTA
                | ProvingJobCircuitType::GUTALeftGUTARightEndCap
                | ProvingJobCircuitType::GUTAVerifyToCap => {
                    checkpoint_leaf.stats.pm_rewards_commitment.gutas_root
                }
                ProvingJobCircuitType::BatchDeployContracts
                | ProvingJobCircuitType::BatchDeployContractsAggregate
                | ProvingJobCircuitType::DummyBatchDeployContractsAggregate => {
                    checkpoint_leaf
                        .stats
                        .pm_rewards_commitment
                        .deploy_contracts_root
                }
                _ => {
                    return Err(ErrorObject::owned(
                        jsonrpsee::types::ErrorCode::InvalidParams.code(),
                        format!(
                            "Job type {:?} not supported for proof generation",
                            job_id.circuit_type
                        ),
                        None::<()>,
                    ));
                }
            };

            match graph.generate_proof(job_id, &*self.proof_store).await {
                Ok(job_proof) => {
                    if job_proof.root != expected_root {
                        tracing::warn!(
                            "Root mismatch for job {:?}: expected {:?}, got {:?}",
                            job_id,
                            expected_root,
                            job_proof.root
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
impl JobSchedulerRpcServer for CoordinatorEdgeHandler {
    async fn get_pending_job(&self, signed: SignedRequest<QEDHash>) -> RpcResult<Option<QJob>> {
        self.white_list
            .verify_request(
                &signed,
                &MESSAGE_CLAIM_JOB.to_string(),
                Some(Duration::from_secs(30)),
            )
            .map_err(|e| RpcError::Anyhow(e.into()))?;

        let j = match self.task_store.claim_job_from_current_layer().await {
            Ok(job) => job,
            Err(e) => {
                error!("Error claiming job from current task: {:?}", e);
                return Err(RpcError::Anyhow(e.into()));
            }
        };
        match j {
            Some(job) => {
                debug!("Pending job from current task: {:?}", job);
                Ok(Some(job))
            }
            None => {
                debug!("No pending job from current task");
                Ok(None)
            }
        }
    }

    async fn get_proof_by_id(&self, job_id: QProvingJobDataID) -> RpcResult<Vec<u8>> {
        let proof: QEDProof = self
            .proof_store
            .get_proof_by_id(job_id)
            .await
            .map_err(|e| RpcError::Anyhow(e.into()))?;
        let bytes = bincode::serialize(&proof).map_err(|e| RpcError::Anyhow(e.into()))?;
        Ok(bytes)
    }

    async fn get_bytes_by_id(&self, job_id: QProvingJobDataID) -> RpcResult<Vec<u8>> {
        self.proof_store
            .get_bytes_by_id(job_id)
            .await
            .map_err(|e| RpcError::Anyhow(e.into()))
    }

    async fn set_proof_by_id(
        &self,
        job: QJob,
        proof: Option<QEDProof>,
        signed: SignedRequest<QEDHash>,
    ) -> RpcResult<()> {
        // Verify signature and whitelist
        self.white_list
            .verify_request(&signed, &proof, Some(Duration::from_secs(300)))
            .map_err(|e| RpcError::Anyhow(e.into()))?;

        let job_id = job.job_id;

        // CRITICAL: Validate job ownership before processing proof
        let validation_status = self
            .task_store
            .validate_job_ownership(&job)
            .await
            .map_err(|e| RpcError::Anyhow(anyhow!("Failed to validate job: {}", e)))?;

        match validation_status {
            JobValidationStatus::Valid => {
                info!(
                    "✅ Job {:?} validated successfully, proceeding with proof",
                    job_id
                );
            }
            JobValidationStatus::NoActiveLayer => {
                error!(
                    "⚠️ No active layer when submitting proof for job {:?}",
                    job_id
                );
                return Err(RpcError::Anyhow(anyhow!("System error: no active layer")));
            }
            JobValidationStatus::WrongLayer { expected, provided } => {
                error!(
                    "⚠️ Worker submitted job {:?} for wrong layer: expected {}, got {}",
                    job_id, expected, provided
                );
                self.log_suspicious_activity(&job, "wrong_layer").await;
                return Err(RpcError::Anyhow(anyhow!(
                    "Invalid submission: wrong layer (expected {}, got {})",
                    expected,
                    provided
                )));
            }
            JobValidationStatus::MessageNotFound => {
                error!(
                    "⚠️ Worker submitted proof for non-existent job {:?}, msg_id: {}",
                    job_id, job.msg_id
                );
                self.log_suspicious_activity(&job, "message_not_found")
                    .await;
                return Err(RpcError::Anyhow(anyhow!(
                    "Invalid submission: job not found"
                )));
            }
            JobValidationStatus::MessageNotHidden => {
                error!(
                    "⚠️ Worker submitted proof for non-hidden job {:?}, msg_id: {}",
                    job_id, job.msg_id
                );
                self.log_suspicious_activity(&job, "message_not_hidden")
                    .await;
                return Err(RpcError::Anyhow(anyhow!(
                    "Invalid submission: job not being processed"
                )));
            }
        }

        if let Some(proof) = proof {
            info!("Setting proof by id: {:?}", job_id);

            crate::common::log_proof_details("Coordinator", job_id, &proof);

            verify_witness_and_proof(
                &self.ctx.proof_verifier,
                job_id,
                self.ctx.proof_store.as_ref(),
                &proof,
            )
            .await
            .map_err(|e| RpcError::Anyhow(e.into()))?;

            let output_id = job_id.get_output_id();
            self.proof_store
                .set_proof_by_id(output_id, &proof)
                .await
                .map_err(RpcError::Anyhow)?;
            info!("✅ Proof stored successfully for job {:?}", job_id);
        }
        // remove the job from the current task, no matter if proof is None or Some
        match self.task_store.acknowledge_job_completion(&job).await {
            Ok(_) => {
                info!("Job completed successfully: {:?}", job_id);
            }
            Err(e) => {
                error!("Error acknowledging job completion: {:?}", e);
                return Err(RpcError::Anyhow(e.into()));
            }
        }

        if job_id.is_notify_complete() {
            info!("Notifying core goal completed: {:?}", job_id);
            self.history_queue
                .notify_core_goal_completed_imm(job_id)
                .await
                .map_err(RpcError::Anyhow)?;
        }
        Ok(())
    }
}
