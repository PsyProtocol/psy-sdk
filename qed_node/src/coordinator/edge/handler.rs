use std::collections::HashMap;
// std
use std::sync::Arc;
use std::time::Duration;

use anyhow::{anyhow, bail};
use chrono::Utc;
use qed_core::traits::to_qfelts::ToQFelts;
use qed_crypto::hash::merkle::treeprover::subtree::SubTreeNodeStateTransition;
use qed_data::guta::header::GlobalUserTreeAggregatorHeader;
use rand::RngCore;
use tokio::sync::{mpsc, Mutex};
use tracing::{debug, error, info, warn, trace};

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
    VariableHeightRewardMerkleProof, ProvingJobCircuitType, QJobTopic, QProvingJobDataID,
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
    whitelist_cache: WhiteListCache,
    watcher_client: Arc<WatcherClient>,
}

impl CoordinatorEdgeHandler {
    pub async fn new(args: CoordinatorEdgeArgs) -> anyhow::Result<Self> {
        info!("🚀 Initializing coordinator edge handler...");

        // Create QED store reader from backend configuration
        info!("🗄️ Initializing storage backend...");
        let qed_store = QEDStore::from_backend(args.backend.to_backend()).await?;
        let store_reader = Arc::new(qed_store);
        let task_store = QProvingTaskStoreImpl::new(&args.redis_uri, args.redis_pool_size).await?;
        let qe_args = &args.queue_args;

        let proof_store = Arc::new(
            ProofStoreRedisAsync::new(&args.redis_uri, qe_args.queue_biz_key.clone()).await?,
        );

        // init verifier
        let verifier = Arc::new(get_cached_generic_verifier::<_, 2>());

        let edge_config = crate::coordinator::state::processor::CoordinatorConfig::get_standard();

        // init context
        let ctx = CoordinatorEdgeContext::new(
            edge_config,
            store_reader.clone(),
            Arc::clone(&proof_store),
            Arc::clone(&proof_store),
            verifier,
        )
        .await?;

        let whitelist_cache = WhiteListCache::new(&args.config_path)?;

        // Initialize watcher
        info!("📡 Initializing watcher client...");
        let watcher_client = Arc::new(
            WatcherClient::new(&args.redis_uri).await?
        );
        info!("✅ Watcher client initialized successfully");

        Ok(Self {
            history_queue: Arc::clone(&proof_store),
            proof_store: Arc::clone(&proof_store),
            ctx,
            store: store_reader,
            task_store: Arc::new(task_store),
            whitelist_cache,
            watcher_client,
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
        info!(
            "✅ register user: {}",
            serde_json::to_string_pretty(&zk_user_info).unwrap()
        );
        self.ctx
            .checkpoint_queue
            .cdq_push_imm(zk_user_info)
            .await
            .map_err(|e| CoordinatorError::QueueError(e.to_string()))?;
        info!("✅ User pushed to checkpoint queue.");


        // Convert public key to string representation
        let public_key_str = format!("{}", public_key_hash.to_string_le());

        // Report to watcher
        if let Err(e) = self.watcher_client.register_user(&public_key_str).await
        {
            // Log the error but don't fail the registration
            warn!("❌ Failed to report user registration to watcher: {}", e);
        } else {
            info!("📊 User registration reported to watcher: {}", public_key_str);
        }

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

        // Store contract details for reporting (before converting to with_root)
        let deployer_str = format!("{}", contract.deployer.to_string_le());
        let state_tree_height = contract.code_definition.state_tree_height;
        let function_count = contract.code_definition.functions.len();

        let with_root = contract.into_with_whitelist_root::<QEDHasher>()?;
        let function_whitelist_root_str = format!("{}", with_root.function_whitelist_root.to_string_le());

        let cd_for_queue = WithDrainQueueMetadata::new_params(
            self.ctx.coordinator_config.deploy_contract_channel_id,
            next_checkpoint_id,
            rand::thread_rng().next_u64(),
            with_root,
        );

        self.ctx.checkpoint_queue.cdq_push_imm(cd_for_queue).await?;

        // Report contract deployment to watcher
        let metadata = UserDeployContractMetadata {
            state_tree_height,
            function_count,
            function_whitelist_root: function_whitelist_root_str,
            node_id: self.watcher_client.get_node_id().await.unwrap_or_default(),
            node_type: "coordinator".to_string(),
        };

        if let Err(e) = self.watcher_client
            .deploy_contract(&deployer_str, metadata)
            .await
        {
            // Log the error but don't fail the contract deployment
            warn!("❌ Failed to report contract deployment to watcher: {}", e);
        } else {
            info!("📊 Contract deployment reported to watcher for deployer: {}", deployer_str);
        }

        Ok(())
    }

    pub async fn submit_guta(
        &self,
        input: SubmitGUTARealmResultAPINoProofInput<QEDFelt>,
        proof: ProofWithPublicInputs<QEDFelt, PoseidonGoldilocksConfig, 2>,
        realm_id: u64,
    ) -> anyhow::Result<()> {
        debug!(
            "submit_guta input: {}",
            serde_json::to_string_pretty(&input).unwrap()
        );
        let checkpoint_id = self.get_latest_checkpoint_id().await?;
        if input.checkpoint_id.saturating_sub(1) < checkpoint_id {
            warn!("⚠️ got guta at old checkpoint {}, expected {}", input.checkpoint_id.saturating_sub(1), checkpoint_id);
        }

        let expected_realm_checkpoint_tree_root = QEDCoordinatorStoreReaderAsync::get_checkpoint_tree_root(&self.store, input.checkpoint_id.saturating_sub(1)).await?;
        if expected_realm_checkpoint_tree_root != input.checkpoint_tree_root {
            anyhow::bail!("invalid checkpoint tree root {} from realm, expected {}", input.checkpoint_tree_root, expected_realm_checkpoint_tree_root);
        }

        let checkpoint_queue = self.ctx.checkpoint_queue.clone();
        let proof_store = self.ctx.proof_store.clone();
        let config = self.ctx.coordinator_config.clone();
        let verifier = self.ctx.proof_verifier.clone();
        // verify top line proof
        if !input.top_line_proof.verify::<QEDHasher>() {
            anyhow::bail!("invalid top line proof from realm");
        }

        if input.top_line_proof.old_root != input.top_line_proof.old_value || input.top_line_proof.new_root != input.top_line_proof.new_value {
            anyhow::bail!("top line not currently supported for guta proofs");
        }

        //if circuit type is GUTANoChange, disable the proof
        if input.circuit_type == ProvingJobCircuitType::GUTANoChange {
            info!("⚠️ GUTANoChange proof, disabling it");
            return Ok(());
        }
        if input.top_line_proof.new_root == input.top_line_proof.old_root {
            anyhow::bail!("⚠️ realm root should be different");
        }

        // verify state consistency
        let old_root = self
            .store
            .get_user_top_tree_cap_root(checkpoint_id, config.realm_root_level, input.realm_id)
            .await?;

        info!("old root from db: {}", old_root);
        info!("old root from realm: {}", input.top_line_proof.old_root);
        if old_root != input.top_line_proof.old_root {
            tracing::error!("invalid top line proof old value {} from realm, expected {}", input.top_line_proof.old_root, old_root);
            anyhow::bail!("invalid top line proof old value from realm");
        }
        let top_line_proof_data = TopLineProofData {
            old_root: format!("{}", input.top_line_proof.old_root.to_string_le()),
            new_root: format!("{}", input.top_line_proof.new_root.to_string_le()),
            old_value: format!("{}", input.top_line_proof.old_value.to_string_le()),
            new_value: format!("{}", input.top_line_proof.new_value.to_string_le()),
        };

        tracing::info!(
            "✅ verified guta result proof public input: {:?} ",
            proof.public_inputs
        );

        // verify witness
        let guta_header = GlobalUserTreeAggregatorHeader {
            guta_circuit_whitelist: config.guta_circuit_whitelist,
            checkpoint_tree_root: input.checkpoint_tree_root,
            state_transition: SubTreeNodeStateTransition {
                old_node_value: input.top_line_proof.old_root,
                new_node_value: input.top_line_proof.new_root,
                node_index: F::from_noncanonical_u64(input.top_line_proof.index),
                node_level: F::from_canonical_u64((config.realm_root_level as usize + input.top_line_proof.siblings.len()) as u64),
            },
            stats: input.guta_stats,
        };
        let proof_public_inputs_hash = QHashOut::from_qfelts(&proof.public_inputs[11..15]);
        let expected_proof_public_inputs_hash = guta_header.qfhash::<QEDHasher>();
        if expected_proof_public_inputs_hash != proof_public_inputs_hash {
            tracing::error!(
                "ensure expected_proof_public_inputs_hash: {} == proof.public_inputs[11..15] {}",
                expected_proof_public_inputs_hash,
                proof_public_inputs_hash,
            );
            anyhow::bail!("invalid realm submit guta proof public inputs hash");
        }

        // verify proof
        verifier.verify_proof_of_type(input.circuit_type, &proof)?;

        let realm_proof_public_inputs =  proof.public_inputs.clone();
        let circuit_type = input.circuit_type;

        // Report GUTA submission to watcher with structured metadata
        let checkpoint_id = self.get_latest_checkpoint_id().await?;
        let next_checkpoint_id = checkpoint_id + 1;

        if input.checkpoint_id  != next_checkpoint_id {
            warn!("❌ Invalid checkpoint id from realm: {}, latest checkpoint id {}", input.checkpoint_id, checkpoint_id);
            anyhow::bail!("invalid checkpoint id from realm");
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
        trace!("✅ wrote guta result to proof store");
        checkpoint_queue.cdq_push_imm(queue_item.clone()).await?;
        trace!("✅ wrote guta result to proof store end");
        let metadata = queue_item.get_dq_metadata();
        let items: Vec<SubmitGUTARealmResultAPIQueueItem<GoldilocksField>> =
            checkpoint_queue.cdq_peek_imm(metadata.channel_id).await?;
        debug!(
            "Retrieved GUTA queue items: {} items, metadata: {:#?}",
            items.len(),
            metadata
        );

        // Report GUTA submission to watcher with structured metadata
        let metadata = UserGutaSubmissionMetadata {
            checkpoint_id,
            circuit_type,
            top_line_proof: top_line_proof_data,
            realm_proof_public_inputs,
            node_id: self.watcher_client.get_node_id().await.unwrap_or_default(),
            node_type: "coordinator".to_string(),
        };

        if let Err(e) = self.watcher_client
            .submit_guta(realm_id, metadata)
            .await
        {
            // Log the error but don't fail the GUTA submission
            warn!("❌ Failed to report GUTA submission to watcher: {}", e);
        } else {
            info!("📊 GUTA submission reported to watcher for realm_id: {}", realm_id);
        }


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
        realm_id: u32,
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

        let realm_merkle_proof = self.get_user_sub_tree_merkle_proof(
            request_checkpoint_id,
            0,
            self.ctx.coordinator_config.realm_root_level,
            realm_id as u64,
        ).await?;

        // Convert compact to full sync info with all required fields
        let sync_info = CheckpointSyncInfo {
            latest_checkpoint_id: latest,
            description: None,
            source_coordinator_edge_id: None,
            sync_timestamp: Utc::now().timestamp() as u64,
            compact,
            realm_root: realm_merkle_proof.value,
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
}

use super::error::RpcError;
use super::rpc::CoordinatorEdgeRpcServer;
use super::types::LatestCheckpointResponse;
use crate::common::whitelist::{WhiteList, WhiteListCache};
use async_trait::async_trait;
use jsonrpsee::core::RpcResult;
use plonky2::field::goldilocks_field::GoldilocksField;
use qed_core::config::network_constants::COORDINATOR_USER_TREE_HEIGHT;
use serde::Serialize;
use qed_prover::local::request::{QDeployContractRPCRequest, QRegisterUserRPCRequest};
use qed_prover::wallet::secp_sign::SignedRequest;
use qed_store::queue::redis_queue::NotificationQueue;
use qed_store::queue::task_queue::{current_timestamp_millis, JobValidationStatus, QJob, QProvingTaskStore, QProvingTaskStoreImpl};
use crate::watcher::events::{JobCompletedEvent, JobStartedEvent, TopLineProofData, UserDeployContractMetadata, UserGutaSubmissionMetadata, WatcherMessage};
use crate::watcher::watcher::NodeType;
use crate::watcher::watcher_client::WatcherClient;
use crate::watcher::watcher_service::{current_timestamp, current_timestamp_mills, WATCHER_RSMQ};

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
        realm_id: u64,
    ) -> RpcResult<String> {
        self.submit_guta(input, proof, realm_id)
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
        realm_id: u32,
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
                        ProvingJobCircuitType::GUTATwoGUTAWithCheckpointUpgrade |
                        ProvingJobCircuitType::GUTAVerifyToCap => {
                            graph.guta_graph.has_node(job_id)
                        } | ProvingJobCircuitType::GUTAVerifyToCapWithCheckpointUpgrade => {
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

        let checkpoint_leaf = self
            .get_checkpoint_leaf_data(actual_checkpoint_id)
            .await
            .map_err(|e| {
                ErrorObject::owned(
                    jsonrpsee::types::ErrorCode::InternalError.code(),
                    format!("Failed to get checkpoint data for {}: {}", actual_checkpoint_id, e),
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
                | ProvingJobCircuitType::GUTATwoGUTAWithCheckpointUpgrade
                | ProvingJobCircuitType::GUTAVerifyToCapWithCheckpointUpgrade
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

            debug!("job_id: {}", job_id.to_hex_string());

            match graph.generate_variable_height_reward_proof(job_id, self.ctx.coordinator_config.coordinator_id, &*self.proof_store).await {
                Ok((variable_height_proof, root_job_id)) => {
                    debug!("coordinator proof: {}, root_job_id: {}", serde_json::to_string_pretty(&variable_height_proof).unwrap(), root_job_id.to_hex_string());
                    let (computed_root, _) = variable_height_proof.compute_root_and_nullifier_index();

                    if computed_root != expected_root {
                        tracing::warn!(
                            "Root mismatch for job({}): expected {}, got {}",
                            job_id.to_hex_string(),
                            expected_root,
                            computed_root
                        );
                    }

                    proofs.push((variable_height_proof, root_job_id));
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

    async fn get_graphviz(&self, checkpoint_id: u64) -> RpcResult<String> {
        use jsonrpsee::types::ErrorObject;

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

        let graphviz_content = graph.get_graphviz();
        Ok(graphviz_content)
    }
}

#[async_trait]
impl JobSchedulerRpcServer for CoordinatorEdgeHandler {
    async fn get_pending_job(&self, signed: SignedRequest<QEDHash>) -> RpcResult<Option<QJob>> {
        self.whitelist_cache
            .verify_request(
                &signed,
                &MESSAGE_CLAIM_JOB.to_string(),
                Some(Duration::from_secs(30)),
            )
            .map_err(|e| RpcError::Anyhow(e.into()))?;

        let worker_id = signed.worker_public_key.to_string();
        let j = match self.task_store.claim_job_from_current_layer(&worker_id).await {
            Ok(job) => job,
            Err(e) => {
                error!("Error claiming job from current task: {:?}", e);
                return Err(RpcError::Anyhow(e.into()))
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
            }
            _ => {
                trace!("No pending job from current task");
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
        proof: QEDProof,
        signed: SignedRequest<QEDHash>,
    ) -> RpcResult<()> {
        // Verify signature and whitelist
        self.whitelist_cache
            .verify_request(&signed, &proof, Some(Duration::from_secs(300)))
            .map_err(|e| RpcError::Anyhow(e.into()))?;

        let job_id = job.job_id;
        let worker_id = signed.worker_public_key.to_string();

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

        self.acknowledge_job_completion(&job, worker_id).await.map_err(RpcError::Anyhow)?;
        Ok(())
    }
}

impl CoordinatorEdgeHandler {
    async fn acknowledge_job_completion(&self, job: &QJob, worker_id: impl ToString) -> anyhow::Result<()> {
        let job_id = job.job_id;
        let worker_id = worker_id.to_string();

        // Acknowledge job completion and get the job status
        let job_status = match self.task_store.acknowledge_job_completion(&job, &worker_id).await {
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
            self.history_queue
                .notify_core_goal_completed_imm(job_id)
                .await?;
        }

        Ok(())
    }
}
