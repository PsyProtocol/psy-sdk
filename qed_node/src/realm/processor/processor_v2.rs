use std::{marker::PhantomData, sync::Arc, time::Duration};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::collections::HashMap;
use std::time::Instant;
use anyhow::{anyhow, bail, ensure};
use async_trait::async_trait;
use plonky2::{
    field::{
        goldilocks_field::GoldilocksField,
        types::{Field, PrimeField64},
    },
    hash::hash_types::RichField,
    plonk::config::PoseidonGoldilocksConfig,
};
use qed_core::job::worker_queue::WorkerEventTransmitterAsyncImm;
use qed_crypto::hash::merkle::core::compute_historical_and_current_merkle_roots_core_gt;
use qed_data::guta::proof_input::{GUTAOnlyRegisterUsersInput, GUTARegisterUserFullInput, VerifyGUTAToCapCircuitInputSimple};
use qed_store::queue::task_queue::QProvingTaskStore;
use tracing::{debug, error, info, trace, warn};
use rand::Rng;
use serde::{Deserialize, Serialize};
use tokio::time::sleep;
use kvq::traits::{KVQSerializable, KVQPair};
use qed_common_circuit::hash::merkle::gadgets::delta_merkle_proof;
use qed_core::{
    config::network_constants::{
        GLOBAL_USER_TREE_HEIGHT, REALM_USER_TREE_HEIGHT, GLOBAL_CONTRACT_TREE_HEIGHT,
        MAX_CONTRACT_STATE_TREE_HEIGHT, COORDINATOR_USER_TREE_HEIGHT
    },
    data::qhashout::QHashOut,
    job::{
        id::{ProvingJobCircuitType, ProvingJobDataType, QJobTopic, QProvingJobDataID, ProvingJobDataId},
        traits::{QProofStoreAsyncImm, QProofStoreReaderAsync, QProofStoreWriterAsyncImm},
    },
    utils::graph::BidirectionalGraph,
};
use qed_crypto::{
    common::{
        cached_circuit_library::get_cached_circuit_library,
        circuit_library::CircuitInfoLibraryCore,
        generic_circuit_verifier::GenericCircuitVerifier,
        user_id::get_user_id_from_registration_id
    },
    hash::{
        merkle::{
            core::{compute_root_merkle_proof_generic, DeltaMerkleProofCore, MerkleProofCore},
            treeprover::{data::CircuitInputWithDependencies, subtree::SubTreeNodeStateTransition},
            utils::{
                common::{SimpleMerkleNodeKey, QMerkleNode},
                sub_tree_nca::{NCAProofsWithTopLine, PartialUpdateNearestCommonAncestorProof},
            },
        },
        traits::{
            hasher::{FieldQHasher, MerkleHasher, MerkleZeroHasher, PoseidonHasher},
            qhashable::QFieldHashable,
        },
    },
};
use qed_data::{
    config::{
        genesis_config::GenesisConfig,
        store_config::{
            BaseContractStateTreeStore, QEDHash, QEDHasher, QEDProof, UserContractTreeStore,
            CONTRACT_STATE_TREE_ID, MAX_CHECKPOINT, USER_CONTRACT_STATE_TREE_TABLE_TYPE
        }
    },
    guta::{
        header::GlobalUserTreeAggregatorHeader,
        proof_input::{
            VerifyEndCapSimpleStandardInput, VerifySingleEndCapInput, VerifyTwoEndCapCircuitInput,
            VerifyTwoGUTAProofGadgetStandardInput, VerifyTwoGUTAProofGadgetStandardInputSimple,
        },
        stats::GUTAStats,
    },
    models::{
        checkpoint::{block_state::L2BlockStatesModel, sync_info::CheckpointError},
        kvq_merkle::{key::KVQMerkleNodeKey, model::{KVQMerkleTreeModelCore, KVQSemiFixedConfigMerkleTreeModelReaderCore}}
    },
    qdata::{
        checkpoint::QEDL2BlockState, staging_checkpoint_info::StagingCheckpointInfo,
        ups_end_cap_result::UPSEndCapResultCompact, user::QEDUserLeaf
    },
    traits::qdatastore::{qmetadata::QMetaDataStoreWriterSync, qtreedata::QTreeDataStoreWriterSync}
};
use qed_store::{
    node::realm::{QEDRealmStoreReaderAsync, QEDRealmStoreWriterAsyncImm},
    queue::{task_queue::QProvingTaskStoreImpl, QPendingUserStoreAsyncImm, ProofStoreRedisAsync, new_redis_async_pool},
    store::QEDStore,
    store::journal::{Journal, JournalStore}
};
use qed_data::config::store_config::{StagingCheckpointInfoStore, StagingDeltaRecordStore};
use qed_data::guta::proof_input::VerifyLeftGUTARightEndCapInputSimple;
use crate::common::slot::SLOT_SIZE;
use crate::{
    common_v2::traits::realm::{
        random_uuid_for_checkpoint, CoordinatorClient, GenericTreeNodeUpdate,
        GlobalBlockUpdateFromCoordinator, GlobalUserTreeMerkleReader, GraphDependencyBuilder,
        RealmDataForCoordinator, RealmDataForCoordinatorHeader, RealmEdgeContractStateTreeUpdate,
        RealmProcessorCombinedUpdate, RealmProcessorEdgeQueueHelper, RealmProcessorStateClient,
        SimpleTreeUpdateBuilder, UniqueQueueId
    },
    common::clock::SlotTimer,
    common::retry::Retryable,
    common::slot::{LocalClock, Slot},
    common::verifier::get_cached_generic_verifier,
    coordinator::client_v2::ConcreteCoordinatorClient,
    realm::{RealmNodeConfig, RealmProcessor as RealmProcessorV1},
    realm::processor::{ConcreteRealmProcessorContext, SyncCheckpointResult},
    realm::state::edge_queue_helper::RealmEdgeQueueHelper,
    realm::state::processor::RealmConfig,
    realm::state::processor_v2::RealmProcessorContextV2,
};

use crate::realm::state::queue_factory::QueueFactory;

type C = PoseidonGoldilocksConfig;
const D: usize = 2;
const MAX_ATTEMPTS: u64 = 5;

pub struct GUTAStatsWithProofId<F: RichField> {
    pub proof_id: QProvingJobDataID,
    pub guta_stats: GUTAStats<F>,
    pub merkle_key: SimpleMerkleNodeKey,
    pub old_value: QHashOut<F>,
    pub new_value: QHashOut<F>,
    pub checkpoint_root: QHashOut<F>,
}
type F = GoldilocksField;

pub async fn run_realm_processor_v2(config: RealmNodeConfig) -> anyhow::Result<()> {
    let mut realm_processor = RealmProcessorV2::new(config).await?;
    let _ = realm_processor.start().await?;
    Ok(())
}

pub struct RealmProcessorV2 {
    pub realm_config: RealmConfig,
    pub store: QEDStore,
    pub proof_verifier: Arc<GenericCircuitVerifier<C, D>>,
    pub task_store: QProvingTaskStoreImpl,
    pub config_path: String,
    pub edge_queue_helper: Arc<RealmEdgeQueueHelper<F>>,
    pub coordinator_client: Arc<ConcreteCoordinatorClient>,
    pub proof_store: Arc<ProofStoreRedisAsync>,
}

impl RealmProcessorV2 {
    fn verify_proof_of_type(
        &self,
        circuit_type: ProvingJobCircuitType,
        proof: &plonky2::plonk::proof::ProofWithPublicInputs<F, C, D>,
    ) -> anyhow::Result<()> {
        self.proof_verifier.verify_proof_of_type(circuit_type, proof)
    }

    async fn get_context(&self) -> anyhow::Result<RealmProcessorContextV2<ProofStoreRedisAsync>> {
        Ok(RealmProcessorContextV2 {
            realm_config: self.realm_config,
            proof_store: self.proof_store.clone(),
            store: self.store.clone(),
            proof_verifier: self.proof_verifier.clone(),
            task_store: Arc::new(self.task_store.clone()),
            config_path: self.config_path.clone(),
        })
    }
    pub async fn new(
        config: RealmNodeConfig
    ) -> anyhow::Result<Self> {
        info!("Realm Processor V2 Config: {:?}", config);

        let pool = new_redis_async_pool(
            config.redis.redis_uri.as_str(),
            config.redis.pool_size.unwrap_or(10)
        ).await?;

        let task_store = QProvingTaskStoreImpl::new(
            &config.redis.redis_uri.as_str(),
            config.redis.pool_size.unwrap_or(10),
        )
        .await?;

        let realm_qps = ProofStoreRedisAsync::new(
            &config.redis.redis_uri.as_str(),
            config.queue.queue_biz_key,
        ).await?;

        let store = QEDStore::new(&config.backend.to_backend()).await?;
        let proof_verifier = Arc::new(get_cached_generic_verifier::<C, D>());
        let realm_config = RealmConfig::get_standard(config.realm.realm_id);
        let coordinator_client = Arc::new(ConcreteCoordinatorClient::new(config.coordinator_addr)?);

        let queue_helper = QueueFactory::create_rsmq_helper::<crate::realm::F>(
            &config.redis.redis_uri,
            config.redis.pool_size.unwrap_or(10),
            config.realm.realm_id,
            Arc::new(store.clone()),
        ).await?;

        Ok(Self {
            realm_config,
            store,
            proof_verifier,
            task_store,
            config_path: config.config_path,
            edge_queue_helper: Arc::new(queue_helper),
            coordinator_client,
            proof_store: Arc::new(realm_qps),
        })
    }

      pub async fn start(mut self) -> anyhow::Result<()> {
        info!("Realm Processor V2 starting");

        self.initialize_store().await?;

        loop {
            match self.process_block().await {
                Err(err) if !err.to_string().contains("No user updates") => {
                    error!("Block processing failed: {:?}", err);
                },
                _ => {}
            }

            tokio::time::sleep(Duration::from_millis(500)).await;
        }

        Ok(())
    }

    async fn process_block(&mut self) -> anyhow::Result<()> {
        let now = Instant::now();
        debug!("1️⃣ Processing block: prepare_for_next_block");
        let current_unique_checkpoint = self.prepare_for_next_block().await?;
        debug!("prepare_for_next_block cost time: {:?}, current_unique_checkpoint: {:?}", now.elapsed(), current_unique_checkpoint);

        debug!("2️⃣ Processing block: build_block_delta");
        let delta = self.build_block_delta(current_unique_checkpoint, vec![]).await?;
        debug!("build_block_delta cost time: {:?}, current_unique_checkpoint: {:?}", now.elapsed(), current_unique_checkpoint);
        debug!("3️⃣ Processing block: wait_for_inclusion");
        self.wait_for_inclusion(&delta).await?;
        debug!("wait_for_inclusion cost time: {:?}, current_unique_checkpoint: {:?}", now.elapsed(), current_unique_checkpoint);
        Ok(())
    }

    async fn wait_for_inclusion(
        &self,
        delta: &RealmProcessorCombinedUpdate<F>,
    ) -> anyhow::Result<()> {
        self.save_update_delta_record(delta).await?;
        info!("💾 Saved delta record");

        self.task_store.finish(delta.local_checkpoint_id, self.realm_config.realm_id).await?;
        info!("🏁 Finished task store");

        self.propagate_update_delta_record_to_peers(delta).await?;
        info!("📡 Propagated delta record to peers");

        self.proof_store.wait_for_block_proving_jobs_imm(delta.local_checkpoint_id, Some(Duration::from_millis(SLOT_SIZE))).await?;
        info!("⏳ Waited for block proving jobs completion");

        let full_message_for_coordinator = RealmDataForCoordinator {
            header: delta.header.clone(),
            proof: self.proof_store.get_bytes_by_id(delta.root_job_id.get_output_id()).await?,
        };
        info!(
            coordinator_message = %serde_json::to_string_pretty(&full_message_for_coordinator.header).unwrap(),
            "📤 Prepared coordinator message"
        );

        let current_checkpoint_id = self.coordinator_client.get_current_checkpoint_id().await?;
        self.coordinator_client.submit_realm_result(&full_message_for_coordinator).await?;
        info!("📤 Submitted realm result to coordinator");

        info!(
            final_root_job_id = %delta.root_job_id.to_hex_string(),
            new_realm_root = %serde_json::to_string_pretty(&delta.new_realm_root).unwrap(),
            old_realm_root = %serde_json::to_string_pretty(&delta.old_realm_root).unwrap(),
            "✅ Delta processing completed successfully"
        );

        info!(
            expected_realm_root = %serde_json::to_string_pretty(&delta.new_realm_root).unwrap(),
            checkpoint_id = delta.local_checkpoint_id,
            realm_id = delta.realm_id,
            "⏳ Starting wait_for_inclusion"
        );

        let mut attempts = 0u64;
        let mut expected_checkpoint_id = current_checkpoint_id + 1;

        loop {
            if attempts >= MAX_ATTEMPTS {
                anyhow::bail!(
                    "Realm root not included after {} attempts. Expected: {:?}",
                    MAX_ATTEMPTS,
                    serde_json::to_string_pretty(&delta.new_realm_root).unwrap(),
                );
            }
            tokio::time::sleep(Duration::from_secs(1)).await;
            info!(
                attempt = attempts + 1,
                max_attempts = MAX_ATTEMPTS,
                "🔄 Waiting for coordinator completion (attempt {}/{})",
                attempts + 1,
                MAX_ATTEMPTS
            );
            info!(
                current_checkpoint_id = current_checkpoint_id,
                expected_checkpoint_id = expected_checkpoint_id,
                realm_id = self.realm_config.realm_id,
                "⏳ Waiting for checkpoint {} (current: {})",
                expected_checkpoint_id,
                current_checkpoint_id
            );
            // TODO
            let coordinator_update = match self.coordinator_client.wait_until_coordinator_completed(self.realm_config.realm_id as u64, expected_checkpoint_id).await {
                Ok(coordinator_update) => coordinator_update,
                Err(e) => {
                    attempts += 1;
                    error!("❌ Error waiting for coordinator completion: {:?}", e);
                    continue;
                }
            };
            info!(
                coordinator_realm_root = %serde_json::to_string_pretty(&coordinator_update.realm_root).unwrap(),
                coordinator_checkpoint_id = coordinator_update.compact.l2_block_state.checkpoint_id,
                "📥 Received coordinator update"
            );

            if coordinator_update.realm_root == delta.new_realm_root {
                info!("✅ Realm root matched! Inclusion confirmed");
                break;
            }

            attempts += 1;
            expected_checkpoint_id += 1;
            warn!(
                expected_root = %serde_json::to_string_pretty(&delta.new_realm_root).unwrap(),
                received_root = %serde_json::to_string_pretty(&coordinator_update.realm_root).unwrap(),
                attempt = attempts,
                "❌ Realm root mismatch on attempt {}",
                attempts
            );
        }

        info!("🔄 Starting final sync after inclusion confirmation");
        let (current_checkpoint_id, current_realm_root) = self.get_latest_checkpoint_and_realm_root().await?;
        info!(
            current_checkpoint_id = current_checkpoint_id,
            current_realm_root = %serde_json::to_string_pretty(&current_realm_root).unwrap(),
            "📊 Retrieved current state for final sync"
        );

        self.sync_finalized_to_latest_checkpoint(
            current_checkpoint_id,
            current_realm_root,
        ).await?;
        info!("✅ Final sync completed");

        info!("✅ wait_for_inclusion completed successfully");
        Ok(())
    }

    async fn handle_only_pending_users(
        &self,
        current_checkpoint_id: u64,
        pending_users: Vec<MerkleProofCore<QHashOut<F>>>,
        mut combined_update: RealmProcessorCombinedUpdate<F>,
    ) -> anyhow::Result<RealmProcessorCombinedUpdate<F>> {
        let realm_id = self.realm_config.realm_id as u64;
        let mut tree_update_builder = SimpleTreeUpdateBuilder { updates: vec![] };

        let updates: Vec<GenericTreeNodeUpdate<F>> = pending_users.iter()
            .map(|proof| {
                let user_id = get_user_id_from_registration_id(proof.index);
                let user_leaf = QEDUserLeaf {
                    public_key: proof.value,
                    user_state_tree_root: self.realm_config.default_user_state_tree_root,
                    balance: F::ZERO,
                    nonce: F::ZERO,
                    last_checkpoint_id: F::ZERO,
                    event_index: F::ZERO,
                    user_id: F::from_canonical_u64(user_id),
                };
                GenericTreeNodeUpdate {
                    level: GLOBAL_USER_TREE_HEIGHT,
                    index: user_id,
                    new_value: user_leaf.qfhash::<QEDHasher>(),
                }
            })
            .collect();

        let (realm_root_update, delta_proofs) = self
            .get_multiple_delta_merkle_proofs::<QEDHasher>(
                &mut tree_update_builder,
                current_checkpoint_id,
                updates,
                Some(self.realm_config.realm_root_level),
            )
            .await?;

        let regs: Vec<GUTARegisterUserFullInput<F>> = pending_users.iter()
            .zip(delta_proofs.iter())
            .map(|(registration_proof, delta_proof)| GUTARegisterUserFullInput {
                user_registration_tree_merkle_proof: registration_proof.clone(),
                global_user_tree_update_proof: delta_proof.clone(),
            })
            .collect();

        let guta_stats = GUTAStats {
            fees_collected: F::ZERO,
            user_ops_processed: F::ZERO,
            total_transactions: F::ZERO,
            slots_modified: F::ZERO,
        };

        let checkpoint_tree_root = self.store.get_checkpoint_tree_root(current_checkpoint_id).await?;
        let input = GUTAOnlyRegisterUsersInput {
            checkpoint_tree_root,
            guta_register_user_inputs: regs,
        };

        let w_id = QProvingJobDataID::new(
            QJobTopic::GenerateStandardProof,
            current_checkpoint_id,
            0, // slot_id - default value for realm processing
            realm_id as u32,
            0,
            0,
            ProvingJobCircuitType::GUTAOnlyRegisterUsers,
            ProvingJobDataType::InputWitness,
            0,
        );

        self.proof_store
            .set_bytes_by_id(w_id.get_input_witness_id(), &bincode::serialize(&input)?)
            .await?;
        self.task_store.register_dependencies(w_id.get_output_id(), &[]).await;

        combined_update.root_job_id = w_id.get_output_id();
        combined_update.header.root_job_id = w_id;
        combined_update.new_realm_root = realm_root_update.new_value;
        combined_update.header.end_realm_root = realm_root_update.new_value;
        combined_update.header.start_realm_root = delta_proofs.first().unwrap().old_root;
        combined_update.header.guta_stats = guta_stats;

        let user_leaves: Vec<QEDUserLeaf<F>> = pending_users.iter()
            .map(|proof| {
                let user_id = get_user_id_from_registration_id(proof.index);
                QEDUserLeaf {
                    public_key: proof.value,
                    user_state_tree_root: self.realm_config.default_user_state_tree_root,
                    balance: F::ZERO,
                    nonce: F::ZERO,
                    last_checkpoint_id: F::ZERO,
                    event_index: F::ZERO,
                    user_id: F::from_canonical_u64(user_id),
                }
            })
            .collect();
        combined_update.updated_users.extend(user_leaves);
        combined_update.global_user_tree_updates = tree_update_builder.finalize();

        Ok(combined_update)
    }

    async fn handle_pending_users_with_existing_jobs(
        &self,
        checkpoint_id: u64,
        pending_users: Vec<MerkleProofCore<QHashOut<F>>>,
        mut combined_update: RealmProcessorCombinedUpdate<F>,
        tree_update_builder: &mut SimpleTreeUpdateBuilder<F>,
        existing_root_job_id: QProvingJobDataID,
    ) -> anyhow::Result<RealmProcessorCombinedUpdate<F>> {
        use qed_crypto::common::user_id::get_user_id_from_registration_id;
        use qed_data::guta::proof_input::{GUTAOnlyRegisterUsersInput, GUTARegisterUserFullInput, VerifyGUTARegisterUsersCircuitInputSimple};
        use qed_data::guta::header::GlobalUserTreeAggregatorHeader;
        use qed_crypto::hash::merkle::treeprover::data::CircuitInputWithDependencies;

        let realm_id = self.realm_config.realm_id as u64;

        let updates: Vec<GenericTreeNodeUpdate<F>> = pending_users.iter()
            .map(|proof| {
                let user_id = get_user_id_from_registration_id(proof.index);
                let user_leaf = QEDUserLeaf {
                    public_key: proof.value,
                    user_state_tree_root: self.realm_config.default_user_state_tree_root,
                    balance: F::ZERO,
                    nonce: F::ZERO,
                    last_checkpoint_id: F::ZERO,
                    event_index: F::ZERO,
                    user_id: F::from_canonical_u64(user_id),
                };
                GenericTreeNodeUpdate {
                    level: GLOBAL_USER_TREE_HEIGHT,
                    index: user_id,
                    new_value: user_leaf.qfhash::<QEDHasher>(),
                }
            })
            .collect();

        let (_realm_root_update, delta_proofs) = self
            .get_multiple_delta_merkle_proofs::<QEDHasher>(
                tree_update_builder,
                checkpoint_id,
                updates,
                None,
            )
            .await?;

        let regs: Vec<GUTARegisterUserFullInput<F>> = pending_users.iter()
            .zip(delta_proofs.iter())
            .map(|(registration_proof, delta_proof)| GUTARegisterUserFullInput {
                user_registration_tree_merkle_proof: registration_proof.clone(),
                global_user_tree_update_proof: delta_proof.clone(),
            })
            .collect();

        let checkpoint_tree_root = self.store.get_checkpoint_tree_root(checkpoint_id).await?;
        let pending_users_input = GUTAOnlyRegisterUsersInput {
            checkpoint_tree_root,
            guta_register_user_inputs: regs.clone(),
        };

        let pending_users_job_id = QProvingJobDataID::new(
            QJobTopic::GenerateStandardProof,
            checkpoint_id,
            0, // slot_id - default value for realm processing
            realm_id as u32,
            0,
            1,
            ProvingJobCircuitType::GUTAOnlyRegisterUsers,
            ProvingJobDataType::InputWitness,
            0,
        );

        self.proof_store
            .set_bytes_by_id(pending_users_job_id.get_input_witness_id(), &bincode::serialize(&pending_users_input)?)
            .await?;
        self.task_store.register_dependencies(pending_users_job_id.get_output_id(), &[]).await;

        let final_root_job_id = QProvingJobDataID::new(
            QJobTopic::GenerateStandardProof,
            checkpoint_id,
            0, // slot_id - default value for realm processing
            realm_id as u32,
            0,
            2,
            ProvingJobCircuitType::GUTATwoEndCap,
            ProvingJobDataType::InputWitness,
            0,
        );

        let checkpoint_tree_root = self.store.get_latest_checkpoint_tree_root().await?;

        let guta = GlobalUserTreeAggregatorHeader {
            guta_circuit_whitelist: QHashOut::ZERO,
            checkpoint_tree_root,
            state_transition: SubTreeNodeStateTransition {
                old_node_value: combined_update.old_realm_root,
                new_node_value: delta_proofs.last().unwrap().new_root,
                node_index: F::from_canonical_u64(combined_update.realm_id),
                node_level: F::from_canonical_u8(self.realm_config.realm_root_level),
            },
            stats: GUTAStats {
                fees_collected: F::ZERO,
                user_ops_processed: F::from_canonical_usize(pending_users.len()),
                total_transactions: F::ZERO,
                slots_modified: F::from_canonical_usize(pending_users.len()),
            },
        };

        let bp = self.store.get_user_sub_tree_merkle_proof(
            checkpoint_id ,
            0,
            self.realm_config.realm_root_level,
            combined_update.realm_id,
        ).await?;
        let top_line_siblings = bp.siblings;

        let input = VerifyGUTARegisterUsersCircuitInputSimple {
            guta_proof_header: guta,
            top_line_siblings,
            guta_register_user_inputs: regs.clone(),
        };

        let ww = CircuitInputWithDependencies::<VerifyGUTARegisterUsersCircuitInputSimple<F>> {
            input,
            dependencies: vec![existing_root_job_id],
        };

        let final_root_job_id = QProvingJobDataID::new(
            QJobTopic::GenerateStandardProof,
            checkpoint_id,
            0, // slot_id - default value for realm processing
            self.realm_config.realm_id as u32,
            0,
            0,
            ProvingJobCircuitType::GUTARegisterUsers,
            ProvingJobDataType::InputWitness,
            0,
        );

        self.proof_store
            .set_bytes_by_id(final_root_job_id.get_input_witness_id(), &bincode::serialize(&ww)?)
            .await?;
        self.task_store.register_dependencies(final_root_job_id.get_output_id(), &ww.dependencies).await;

        combined_update.root_job_id = final_root_job_id.get_output_id();
        combined_update.header.root_job_id = final_root_job_id;
        combined_update.header.end_realm_root = delta_proofs.last().unwrap().new_root;

        let user_leaves: Vec<QEDUserLeaf<F>> = pending_users.iter()
            .map(|proof| {
                let user_id = get_user_id_from_registration_id(proof.index);
                QEDUserLeaf {
                    public_key: proof.value,
                    user_state_tree_root: self.realm_config.default_user_state_tree_root,
                    balance: F::ZERO,
                    nonce: F::ZERO,
                    last_checkpoint_id: F::ZERO,
                    event_index: F::ZERO,
                    user_id: F::from_canonical_u64(user_id),
                }
            })
            .collect();
        combined_update.updated_users.extend(user_leaves);

        Ok(combined_update)
    }

    async fn prepare_for_next_block(&self) -> anyhow::Result<UniqueQueueId> {
        let realm_id = self.realm_config.realm_id as u64;
        info!("🔄 Starting prepare_for_next_block for realm_id: {}", realm_id);
        let (current_checkpoint_id, current_realm_root) = self.get_latest_checkpoint_and_realm_root().await?;
        let coordinator_checkpoint_id = self.coordinator_client.get_current_checkpoint_id().await?;
        info!(
            current_checkpoint_id = current_checkpoint_id,
            current_realm_root = %serde_json::to_string_pretty(&current_realm_root).unwrap(),
            "📊 Retrieved current checkpoint and realm root"
        );

        let coordinator_realm_state = self.coordinator_client.get_current_realm_status_on_coordinator(realm_id).await?;
        info!(
            coordinator_realm_state = %serde_json::to_string_pretty(&coordinator_realm_state).unwrap(),
            "🌐 Retrieved coordinator realm state"
        );

        info!("🔄 Starting sync_finalized_to_latest_checkpoint");
        self.sync_finalized_to_latest_checkpoint(
            current_checkpoint_id,
            current_realm_root,
        ).await?;
        info!("✅ Completed sync_finalized_to_latest_checkpoint");

        let last_unique_queue = self.get_shared_queue_id().await?;
        info!(
            last_queue_id = last_unique_queue.id,
            last_uuid = %last_unique_queue.uuid,
            "📋 Retrieved last unique queue"
        );

        if !self.edge_queue_helper.has_user_updates(last_unique_queue).await? {
            anyhow::bail!("No user updates");
        }
        let latest_checkpoint_id = self.get_latest_checkpoint_id().await?;
        let next_queue_id = UniqueQueueId {
            id: latest_checkpoint_id + 1,
            uuid: random_uuid_for_checkpoint(),
        };
        info!(
            next_queue_id = next_queue_id.id,
            next_uuid = %next_queue_id.uuid,
            "🆕 Generated next unique queue"
        );

        self.set_shared_checkpoint_info(next_queue_id, Default::default()).await?;
        info!("💾 Set shared checkpoint info for next checkpoint");

        info!(
            returned_queue_id = last_unique_queue.id,
            returned_uuid = %last_unique_queue.uuid,
            "✅ prepare_for_next_block completed, returning last queue"
        );
        Ok(last_unique_queue)
    }

    async fn initialize_store(&self) -> anyhow::Result<u64> {
        let realm_id = self.realm_config.realm_id as u64;
        match self.get_latest_checkpoint_id().await {
            Ok(id) => Ok(id),
            Err(e) if matches!(e.downcast_ref::<CheckpointError>(), Some(CheckpointError::NotFound)) => {
                let updates = self.coordinator_client
                    .get_latest_block_updates_from_coordinator(realm_id, 0, MAX_CHECKPOINT)
                    .await?;

                for update in updates.iter() {
                    self.apply_only_global_block_update_dangerous(update).await?;
                }

                let genesis_config = GenesisConfig::from_path(&self.config_path)?.ok_or(anyhow::format_err!("Genesis config not found"))?;
                self.process_genesis_user_states(&genesis_config).await?;

                let next_queue_id = UniqueQueueId {
                    id: 0,
                    uuid: random_uuid_for_checkpoint(),
                };
                self.set_shared_checkpoint_info(next_queue_id, Default::default()).await?;
                Ok(0)
            }
            Err(e) => Err(e),
        }
    }

    async fn build_block_delta(
        &self,
        unique_queue_id: UniqueQueueId,
        pending_users: Vec<MerkleProofCore<QHashOut<F>>>,
    ) -> anyhow::Result<RealmProcessorCombinedUpdate<F>> {
        let (current_checkpoint_id, start_realm_root) = self.get_latest_checkpoint_and_realm_root().await?;
        info!(
            checkpoint_id = current_checkpoint_id,
            queue_id = unique_queue_id.id,
            queue_uuid = %unique_queue_id.uuid,
            pending_users_count = pending_users.len(),
            "🏗️ Starting build_block_delta"
        );

        self.task_store.clear_task_graph().await?;
        info!("🧹 Cleared task graph");

        let realm_id = self.realm_config.realm_id as u64;
        let realm_manager_id = 0u64;
        let start_realm_root = self
            .store
            .get_user_bottom_tree_merkle_proof(
                self.realm_config.realm_root_level,
                current_checkpoint_id,
                (self.realm_config.realm_id as u64) << (REALM_USER_TREE_HEIGHT as u64)
            )
            .await?
            .root;
        info!(
            realm_id = realm_id,
            start_realm_root = %serde_json::to_string_pretty(&start_realm_root).unwrap(),
            "📊 Retrieved start realm root"
        );

        let mut combined_update = RealmProcessorCombinedUpdate {
            realm_id,
            realm_manager_id,
            local_checkpoint_id: current_checkpoint_id,
            queue_id: unique_queue_id.id,
            queue_uuid: unique_queue_id.uuid,
            old_realm_root: start_realm_root,
            new_realm_root: start_realm_root,
            contract_state_tree_updates: vec![],
            user_contract_tree_updates: vec![],
            global_user_tree_updates: vec![],
            updated_users: vec![],
            root_job_id: QProvingJobDataID::notify_realm_complete(0, 1337133769, self.realm_config.realm_id),
            header: RealmDataForCoordinatorHeader {
                realm_id,
                checkpoint_id: current_checkpoint_id,
                start_realm_root,
                end_realm_root: start_realm_root,
                guta_stats: GUTAStats {
                    fees_collected: F::ZERO,
                    user_ops_processed: F::ZERO,
                    total_transactions: F::ZERO,
                    slots_modified: F::ZERO,
                },
                root_job_id: QProvingJobDataID::notify_realm_complete(0, 1337133769, self.realm_config.realm_id),
            },
        };
        info!("🏗️ Initialized combined_update structure");

        let mut user_updates = self.edge_queue_helper
            .dump_user_updates(unique_queue_id)
            .await?;
        info!(
            user_updates_count = user_updates.len(),
            pending_users_count = pending_users.len(),
            "📥 Retrieved user updates from edge queue"
        );
        let real_checkpoint_id = current_checkpoint_id;
        let checkpoint_tree_root = self.store.get_checkpoint_tree_root(real_checkpoint_id).await?;

        // update checkpoint tree merkle proof
        for user_update in user_updates.iter_mut() {
            debug!("real_checkpoint_id: {}, current_checkpoint_id: {}",
                real_checkpoint_id, current_checkpoint_id);

            if user_update.misc_data.checkpoint_historical_merkle_proof.root != checkpoint_tree_root {
                debug!("❗ Mismatch in checkpoint tree root, updating merkle proof");
                warn!("checkpoint_id: {}, user_update checkpoint tree root: {}, store checkpoint tree root: {}",
                    current_checkpoint_id, user_update.misc_data.checkpoint_historical_merkle_proof.root,
                    checkpoint_tree_root);

                let endcap_checkpoint_id = user_update.misc_data.checkpoint_historical_merkle_proof.index;
                let checkpoint_tree_proof = self.store.get_checkpoint_tree_merkle_proof(real_checkpoint_id, endcap_checkpoint_id).await?;

                let (user_computed_historical_root, user_computed_current_root) = compute_historical_and_current_merkle_roots_core_gt::<QHashOut<F>, QEDHasher>(&checkpoint_tree_proof);
                ensure!(user_computed_current_root == checkpoint_tree_proof.root,
                    "checkpoint_id: {}, user_update computed root: {}, store checkpoint tree root: {}",
                    current_checkpoint_id, user_computed_current_root, checkpoint_tree_proof.root);

                ensure!(user_computed_current_root == checkpoint_tree_root,
                    "checkpoint_id: {}, user_update checkpoint tree root: {}, store checkpoint tree root: {}",
                    current_checkpoint_id, user_computed_current_root, checkpoint_tree_root);

                ensure!(user_computed_historical_root == user_update.misc_data.checkpoint_root,
                    "checkpoint_id: {}, user_update checkpoint tree root hash: {}, store checkpoint tree root hash: {}",
                    current_checkpoint_id, user_computed_historical_root, user_update.misc_data.checkpoint_root);

                user_update.misc_data.checkpoint_historical_merkle_proof = checkpoint_tree_proof;
            }
        }

        if user_updates.len() == 0 {
            if pending_users.len() == 0 {
                info!("⏭️ Case: No user updates and no pending users - returning empty update");
                return Ok(combined_update);
            } else {
                info!("👥 Case: No user updates but has pending users - handling only pending users");
                return self.handle_only_pending_users(current_checkpoint_id, pending_users, combined_update).await;
            }
        }
        // TODO-PERF: do we need to do .to_canonical_u64()?
        user_updates.sort_by(|a, b| {
            a.new_user_leaf
                .user_id
                .to_canonical_u64()
                .cmp(&b.new_user_leaf.user_id.to_canonical_u64())
        });
        info!("🔄 Sorted user updates by user_id");

        let mut guta_agg_header =  GlobalUserTreeAggregatorHeader {
            guta_circuit_whitelist: self.realm_config.guta_circuit_whitelist,
            checkpoint_tree_root: self.store.get_checkpoint_tree_root(current_checkpoint_id).await?,
            state_transition: SubTreeNodeStateTransition {
                old_node_value: start_realm_root,
                new_node_value: start_realm_root,
                node_index: F::from_noncanonical_u64(0),
                node_level: F::from_canonical_u64(24),
            },
            stats: GUTAStats {
                fees_collected: F::ZERO,
                user_ops_processed: F::ZERO,
                total_transactions: F::ZERO,
                slots_modified: F::ZERO,
            },
        };

        let mut tree_update_builder = SimpleTreeUpdateBuilder { updates: vec![] };
        if user_updates.len() == 1 {
            info!("🔢 Case: Single user update - processing with GUTASingleEndCap");
            let user_update = &user_updates[0];
            info!(
                user_id = %user_update.new_user_leaf.user_id,
                old_leaf = %serde_json::to_string_pretty(&user_update.old_user_leaf).unwrap(),
                new_leaf = %serde_json::to_string_pretty(&user_update.new_user_leaf).unwrap(),
                "👤 Processing single user update"
            );

            let w_id = QProvingJobDataID::new(
                QJobTopic::GenerateStandardProof,
                current_checkpoint_id,
                0, // slot_id - default value for realm processing
                realm_id as u32,
                0,
                0,
                ProvingJobCircuitType::GUTASingleEndCap,
                ProvingJobDataType::InputWitness,
                0,
            );
            info!(job_id = %w_id.to_hex_string(), "📝 Created GUTASingleEndCap job");

            let end_user_leaf_hash = user_update.new_user_leaf.qfhash::<QEDHasher>();

            let (root_addition, _) = self
                .get_single_node_delta_merkle_proof::<QEDHasher>(
                    &mut tree_update_builder,
                    current_checkpoint_id,
                    GenericTreeNodeUpdate {
                        index: user_update.new_user_leaf.user_id.to_canonical_u64(),
                        level: GLOBAL_USER_TREE_HEIGHT,
                        new_value: end_user_leaf_hash,
                    },
                    COORDINATOR_USER_TREE_HEIGHT,
                )
                .await?;
            info!(
                root_addition = %serde_json::to_string_pretty(&root_addition).unwrap(),
                "🌳 Generated single node delta merkle proof"
            );

            let start_user_leaf_hash = user_update.old_user_leaf.qfhash::<QEDHasher>();

            tree_update_builder.add_update(root_addition.level, root_addition.index, root_addition.new_value);

            let input = CircuitInputWithDependencies {
                input: VerifySingleEndCapInput {
                    guta_circuit_whitelist: self.realm_config.guta_circuit_whitelist,
                    a_end_cap: user_update.misc_data.clone(),
                    start_user_leaf_hash: start_user_leaf_hash,
                    end_user_leaf_hash: end_user_leaf_hash,
                    user_id: user_update.new_user_leaf.user_id,
                },
                dependencies: vec![user_update.proof_id.get_output_id()],
            };
            info!(
                circuit_input = %serde_json::to_string_pretty(&input.input).unwrap(),
                dependencies = %format!("[{}]", input.dependencies.iter().map(|id| id.to_hex_string()).collect::<Vec<_>>().join(", ")),
                "🔧 Created VerifySingleEndCapInput circuit input"
            );

            self.proof_store
                .set_bytes_by_id(w_id.get_input_witness_id(), &bincode::serialize(&input)?)
                .await?;
            self.task_store.register_dependencies(w_id.get_output_id(), &[]).await;
            info!("💾 Stored circuit input and registered dependencies");

            combined_update.root_job_id = w_id;
            combined_update.header.root_job_id = w_id;
            combined_update.header.end_realm_root = root_addition.new_value;
            combined_update.header.guta_stats = input.input.get_guta_header_a().stats;
            combined_update.new_realm_root = root_addition.new_value;

            guta_agg_header = input.input.get_new_guta_header();
            info!("✅ Single user case completed");
        } else if user_updates.len() == 2 {
            info!("🔢 Case: Two user updates - processing with GUTATwoEndCap (force to root)");
            let right_user_update = &user_updates[1];
            let left_user_update = &user_updates[0];
            info!(
                left_user_id = %left_user_update.new_user_leaf.user_id,
                right_user_id = %right_user_update.new_user_leaf.user_id,
                "👥 Processing two user updates"
            );

            let w_id = QProvingJobDataID::new(
                QJobTopic::GenerateStandardProof,
                current_checkpoint_id,
                0, // slot_id - default value for realm processing
                realm_id as u32,
                0,
                0,
                ProvingJobCircuitType::GUTATwoEndCap,
                ProvingJobDataType::InputWitness,
                0,
            );
            info!(job_id = %w_id.to_hex_string(), "📝 Created GUTATwoEndCap job");

            let left_end_user_leaf_hash = left_user_update.new_user_leaf.qfhash::<QEDHasher>();
            let right_end_user_leaf_hash = right_user_update.new_user_leaf.qfhash::<QEDHasher>();

            let (root_addition, dmp_left, dmp_right) = self
                .get_nca_delta_merkle_proof::<QEDHasher>(
                    &mut tree_update_builder,
                    current_checkpoint_id,
                    GenericTreeNodeUpdate {
                        index: left_user_update.new_user_leaf.user_id.to_canonical_u64(),
                        level: GLOBAL_USER_TREE_HEIGHT,
                        new_value: left_end_user_leaf_hash,
                    },
                    GenericTreeNodeUpdate {
                        index: right_user_update.new_user_leaf.user_id.to_canonical_u64(),
                        level: GLOBAL_USER_TREE_HEIGHT,
                        new_value: right_end_user_leaf_hash,
                    },
                    Some(COORDINATOR_USER_TREE_HEIGHT),
                )
                .await?;
            info!(
                root_addition = %serde_json::to_string_pretty(&root_addition).unwrap(),
                dmp_left = %serde_json::to_string_pretty(&dmp_left).unwrap(),
                dmp_right = %serde_json::to_string_pretty(&dmp_right).unwrap(),
                "🌳 Generated NCA delta merkle proof (force_to_root=true)"
            );
            tree_update_builder.add_update(root_addition.level, root_addition.index, root_addition.new_value);

            let input = CircuitInputWithDependencies {
                input: VerifyTwoEndCapCircuitInput {
                    guta_circuit_whitelist: self.realm_config.guta_circuit_whitelist,
                    a_end_cap: left_user_update.misc_data.clone(),
                    b_end_cap: right_user_update.misc_data.clone(),
                    nca_proof: PartialUpdateNearestCommonAncestorProof {
                        child_a: dmp_left.clone(),
                        child_b: dmp_right.clone(),
                        nearest_common_ancestor_level: COORDINATOR_USER_TREE_HEIGHT,
                    },
                },
                dependencies: vec![left_user_update.proof_id.get_output_id(), right_user_update.proof_id.get_output_id()],
            };
            info!(
                circuit_input = %serde_json::to_string_pretty(&input.input).unwrap(),
                dependencies = %format!("[{}]", input.dependencies.iter().map(|id| id.to_hex_string()).collect::<Vec<_>>().join(", ")),
                "🔧 Created VerifyTwoEndCapCircuitInput"
            );

            self.proof_store
                .set_bytes_by_id(w_id.get_input_witness_id(), &bincode::serialize(&input)?)
                .await?;
            self.task_store.register_dependencies(w_id.get_output_id(), &[]).await;
            info!("💾 Stored circuit input and registered dependencies");

            combined_update.root_job_id = w_id;
            combined_update.header.root_job_id = w_id;
            combined_update.header.end_realm_root = root_addition.new_value;
            combined_update.header.guta_stats = input.input.a_end_cap.guta_stats.combine_with(&input.input.b_end_cap.guta_stats);
            combined_update.new_realm_root = root_addition.new_value;

            guta_agg_header.state_transition = SubTreeNodeStateTransition {
                old_node_value: dmp_left.old_root,
                new_node_value: dmp_right.new_root,
                node_index: F::from_canonical_u64(root_addition.index),
                node_level: F::from_canonical_u8(root_addition.level),
            };
            guta_agg_header.stats = input.input.a_end_cap.guta_stats.combine_with(&input.input.b_end_cap.guta_stats);
            info!("✅ Two user case completed");
        } else {
            info!("🔢 Case: Multiple user updates ({}) - processing with GUTA tree aggregation", user_updates.len());
            let has_odd_end_caps = user_updates.len() % 2 == 1;
            let odd_proofs = if has_odd_end_caps { Some(user_updates.last().unwrap()) } else { None };
            info!(
                total_users = user_updates.len() + if has_odd_end_caps { 1 } else { 0 },
                has_odd_end_caps = has_odd_end_caps,
                remaining_pairs = user_updates.len() / 2,
                "🎯 Identified pairing strategy for GUTA tree"
            );

            let end_caps_trimmed_len_half = user_updates.len() / 2;
            let mut guta_records = Vec::with_capacity(user_updates.len());
            info!("🌳 Starting initial pairwise GUTA processing");

            for i in 0..end_caps_trimmed_len_half {
                let left_user_update = &user_updates[i * 2];
                let right_user_update = &user_updates[i * 2 + 1];
                info!(
                    pair_index = i,
                    left_user_id = %left_user_update.new_user_leaf.user_id,
                    right_user_id = %right_user_update.new_user_leaf.user_id,
                    "👥 Processing pair {}/{}",
                    i + 1,
                    end_caps_trimmed_len_half
                );

                let w_id = QProvingJobDataID::new(
                    QJobTopic::GenerateStandardProof,
                    unique_queue_id.id,
                    0, // slot_id - default value for realm processing
                    realm_id as u32,
                    0,
                    i as u32,
                    ProvingJobCircuitType::GUTATwoEndCap,
                    ProvingJobDataType::InputWitness,
                    0,
                );

                let left_end_user_leaf_hash = left_user_update.new_user_leaf.qfhash::<QEDHasher>();
                let right_end_user_leaf_hash = right_user_update.new_user_leaf.qfhash::<QEDHasher>();

                let (root_addition, dmp_left, dmp_right) = self
                    .get_nca_delta_merkle_proof::<QEDHasher>(
                        &mut tree_update_builder,
                        current_checkpoint_id,
                        GenericTreeNodeUpdate {
                            index: left_user_update.new_user_leaf.user_id.to_canonical_u64(),
                            level: GLOBAL_USER_TREE_HEIGHT,
                            new_value: left_end_user_leaf_hash,
                        },
                        GenericTreeNodeUpdate {
                            index: right_user_update.new_user_leaf.user_id.to_canonical_u64(),
                            level: GLOBAL_USER_TREE_HEIGHT,
                            new_value: right_end_user_leaf_hash,
                        },
                        None,
                    )
                    .await?;
                info!(
                    nca_level = root_addition.level,
                    nca_index = root_addition.index,
                    "🌳 Generated NCA proof for pair {} (force_to_root=false)",
                    i + 1
                );

                tree_update_builder.add_update(root_addition.level, root_addition.index, root_addition.new_value);
                let combined_guta_stats = left_user_update
                    .misc_data
                    .guta_stats
                    .combine_with(&right_user_update.misc_data.guta_stats);
                let root_old_value = dmp_left.old_root;
                let root_new_value = dmp_right.new_root;
                info!(
                    combined_stats = %serde_json::to_string_pretty(&combined_guta_stats).unwrap(),
                    "📊 Combined GUTA stats for pair {}",
                    i + 1
                );

                let input = CircuitInputWithDependencies {
                    input: VerifyTwoEndCapCircuitInput {
                        guta_circuit_whitelist: self.realm_config.guta_circuit_whitelist,
                        a_end_cap: left_user_update.misc_data.clone(),
                        b_end_cap: right_user_update.misc_data.clone(),
                        nca_proof: PartialUpdateNearestCommonAncestorProof {
                            child_a: dmp_left,
                            child_b: dmp_right,
                            nearest_common_ancestor_level: root_addition.level,
                        },
                    },
                    dependencies: vec![left_user_update.proof_id, right_user_update.proof_id],
                };
                self.proof_store
                    .set_bytes_by_id(w_id.get_input_witness_id(), &bincode::serialize(&input)?)
                    .await?;
                self.task_store.register_dependencies(w_id.get_output_id(), &[]).await;

                let combo = GUTAStatsWithProofId {
                    proof_id: w_id.get_output_id(),
                    guta_stats: combined_guta_stats,
                    merkle_key: SimpleMerkleNodeKey {
                        level: root_addition.level,
                        index: root_addition.index,
                    },
                    old_value: root_old_value,
                    new_value: root_new_value,
                    checkpoint_root: left_user_update.misc_data.checkpoint_historical_merkle_proof.root,
                };
                guta_records.push(combo);
                info!("✅ Completed pair {} processing", i + 1);
            }
            info!("✅ Completed initial pairwise GUTA processing, created {} GUTA records", guta_records.len());

            info!("🏗️ Starting GUTA tree aggregation from {} records", guta_records.len());
            let mut aggregation_level = 1;
            while guta_records.len() != 1 {
                let is_count_odd = guta_records.len() % 2 == 1;
                let far_right_proof = if is_count_odd { Some(guta_records.pop().unwrap()) } else { None };
                info!(
                    aggregation_level = aggregation_level,
                    current_records = guta_records.len(),
                    is_count_odd = is_count_odd,
                    "🔄 GUTA aggregation level {} - processing {} records",
                    aggregation_level,
                    guta_records.len()
                );

                let half_len = guta_records.len() / 2;
                let mut new_guta_records = Vec::with_capacity(if is_count_odd { half_len + 1 } else { half_len });
                for i in 0..half_len {
                    let left_value = &guta_records[i * 2];
                    let right_value = &guta_records[i * 2 + 1];

                    let (root_addition, dmp_left, dmp_right) = self
                        .get_nca_delta_merkle_proof::<QEDHasher>(
                            &mut tree_update_builder,
                            current_checkpoint_id,
                            GenericTreeNodeUpdate {
                                index: left_value.merkle_key.index,
                                level: left_value.merkle_key.level,
                                new_value: left_value.new_value,
                            },
                            GenericTreeNodeUpdate {
                                index: right_value.merkle_key.index,
                                level: right_value.merkle_key.level,
                                new_value: right_value.new_value,
                            },
                            None, // since we have more than two proofs, we DO NOT force to root
                        )
                        .await?;
                    let w_id = QProvingJobDataID::new(
                        QJobTopic::GenerateStandardProof,
                        current_checkpoint_id,
                        0, // slot_id - default value for realm processing
                        realm_id as u32,
                        root_addition.level as u32,
                        root_addition.index as u32,
                        ProvingJobCircuitType::GUTATwoGUTA,
                        ProvingJobDataType::InputWitness,
                        0,
                    );

                    tree_update_builder.add_update(root_addition.level, root_addition.index, root_addition.new_value);
                    let combined_guta_stats = left_value.guta_stats.combine_with(&right_value.guta_stats);
                    let root_old_value = dmp_left.old_root;
                    let root_new_value = dmp_right.new_root;

                    let input = CircuitInputWithDependencies {
                        input: VerifyTwoGUTAProofGadgetStandardInputSimple {
                            nca_proof: PartialUpdateNearestCommonAncestorProof {
                                child_a: dmp_left,
                                child_b: dmp_right,
                                nearest_common_ancestor_level: root_addition.level,
                            },
                            checkpoint_tree_root: left_value.checkpoint_root,
                            b_checkpoint_tree_root: right_value.checkpoint_root,
                            stats_b: right_value.guta_stats,
                            stats_a: left_value.guta_stats,
                        },
                        dependencies: vec![left_value.proof_id.get_output_id(), right_value.proof_id.get_output_id()],
                    };
                    self.proof_store
                        .set_bytes_by_id(w_id.get_input_witness_id(), &bincode::serialize(&input)?)
                        .await?;
                    self.task_store.register_dependencies(w_id.get_output_id(), &[left_value.proof_id, right_value.proof_id]).await;

                    let combo = GUTAStatsWithProofId {
                        proof_id: w_id.get_output_id(),
                        guta_stats: combined_guta_stats,
                        merkle_key: SimpleMerkleNodeKey {
                            level: root_addition.level,
                            index: root_addition.index,
                        },
                        old_value: root_old_value,
                        new_value: root_new_value,
                        checkpoint_root: left_value.checkpoint_root,
                    };
                    new_guta_records.push(combo);
                }
                guta_records = new_guta_records;
                if is_count_odd {
                    guta_records.push(far_right_proof.unwrap());
                    info!("↗️ Added odd record back to next level");
                }
                aggregation_level += 1;
                info!("✅ Completed aggregation level {}, {} records remaining", aggregation_level - 1, guta_records.len());
            }
            info!("🎯 GUTA tree aggregation completed, final record: level={}, index={}",
                guta_records[0].merkle_key.level, guta_records[0].merkle_key.index);

            if odd_proofs.is_some() {
                info!("🔄 Processing final odd proof with last GUTA record");
                let odd_proof_right = odd_proofs.unwrap();
                let last_guta_left = guta_records.pop().unwrap();

                let new_user_leaf_hash = odd_proof_right.new_user_leaf.qfhash::<QEDHasher>();

                let (root_addition, dmp_left, dmp_right) = self
                    .get_nca_delta_merkle_proof::<QEDHasher>(
                        &mut tree_update_builder,
                        current_checkpoint_id,
                        GenericTreeNodeUpdate {
                            index: last_guta_left.merkle_key.index,
                            level: last_guta_left.merkle_key.level,
                            new_value: last_guta_left.new_value,
                        },
                        GenericTreeNodeUpdate {
                            index: odd_proof_right.new_user_leaf.user_id.to_canonical_u64(),
                            level: COORDINATOR_USER_TREE_HEIGHT,
                            new_value: new_user_leaf_hash,
                        },
                        None, // since we have more than two proofs, we DO NOT force to root
                    )
                    .await?;

                let w_id = QProvingJobDataID::new(
                    QJobTopic::GenerateStandardProof,
                    current_checkpoint_id,
                    0, // slot_id - default value for realm processing
                    realm_id as u32,
                    root_addition.level as u32,
                    root_addition.index as u32,
                    ProvingJobCircuitType::GUTALeftGUTARightEndCap,
                    ProvingJobDataType::InputWitness,
                    0,
                );

                tree_update_builder.add_update(root_addition.level, root_addition.index, root_addition.new_value);

                let combined_guta_stats = last_guta_left.guta_stats.combine_with(&odd_proof_right.misc_data.guta_stats);
                let root_old_value = dmp_left.old_root;
                let root_new_value = dmp_right.new_root;

                let input = CircuitInputWithDependencies {
                    input: VerifyLeftGUTARightEndCapInputSimple {
                        nca_proof: PartialUpdateNearestCommonAncestorProof {
                            child_a: dmp_left,
                            child_b: dmp_right,
                            nearest_common_ancestor_level: root_addition.level,
                        },
                        checkpoint_tree_root: last_guta_left.checkpoint_root,
                        stats_a: last_guta_left.guta_stats,
                        b_end_cap: odd_proof_right.misc_data.clone()
                    },
                    dependencies: vec![last_guta_left.proof_id, odd_proof_right.proof_id],
                };
                self.proof_store
                    .set_bytes_by_id(w_id.get_input_witness_id(), &bincode::serialize(&input)?)
                    .await?;
                self.task_store.register_dependencies(w_id.get_output_id(), &[last_guta_left.proof_id]).await;

                let combo = GUTAStatsWithProofId {
                    proof_id: w_id.get_output_id(),
                    guta_stats: combined_guta_stats,
                    merkle_key: SimpleMerkleNodeKey {
                        level: root_addition.level,
                        index: root_addition.index,
                    },
                    old_value: root_old_value,
                    new_value: root_new_value,
                    checkpoint_root: last_guta_left.checkpoint_root,
                };
                let w_id = combo.proof_id;
                combined_update.root_job_id = w_id;
                combined_update.header.root_job_id = w_id;
                combined_update.header.end_realm_root = combo.new_value;
                combined_update.header.start_realm_root = combo.old_value;
                combined_update.header.guta_stats = combo.guta_stats;
                combined_update.new_realm_root = combo.new_value;

                guta_agg_header.state_transition = SubTreeNodeStateTransition {
                    old_node_value: combo.old_value,
                    new_node_value: combo.new_value,
                    node_index: F::from_canonical_u64(combo.merkle_key.index),
                    node_level: F::from_canonical_u8(combo.merkle_key.level),
                };
                guta_agg_header.stats = combo.guta_stats;
                info!("✅ Final odd proof processed with GUTALeftGUTARightEndCap");
            } else {
                let w_id = guta_records[0].proof_id;
                combined_update.root_job_id = w_id;
                combined_update.header.root_job_id = w_id;
                combined_update.header.end_realm_root = guta_records[0].new_value;
                combined_update.header.start_realm_root = guta_records[0].old_value;
                combined_update.header.guta_stats = guta_records[0].guta_stats;
                combined_update.new_realm_root = guta_records[0].new_value;

                guta_agg_header.state_transition = SubTreeNodeStateTransition {
                    old_node_value: guta_records[0].old_value,
                    new_node_value: guta_records[0].new_value,
                    node_index: F::from_canonical_u64(guta_records[0].merkle_key.index),
                    node_level: F::from_canonical_u8(guta_records[0].merkle_key.level),
                };
                guta_agg_header.stats = guta_records[0].guta_stats;
                info!("✅ Used final aggregated GUTA record as root job");
            }
            info!("🎯 Multiple user case completed");
        }

        if pending_users.len() > 0 {
            info!("👥 Processing pending users with existing jobs");
            let current_root_job_id = combined_update.root_job_id;
            combined_update = self.handle_pending_users_with_existing_jobs(
                    current_checkpoint_id,
                    pending_users,
                    combined_update,
                    &mut tree_update_builder,
                current_root_job_id
            ).await?;
            info!("✅ Completed pending users processing");
        }

        tracing::debug!("GUTA agg header: {}", serde_json::to_string_pretty(&guta_agg_header)?);

        if guta_agg_header.state_transition.node_level != F::from_canonical_u8(COORDINATOR_USER_TREE_HEIGHT) {
            // add a job to verify to the root cap
            let w_id = QProvingJobDataID::new(
                QJobTopic::GenerateStandardProof,
                current_checkpoint_id,
                0, // slot_id - default value for realm processing
                self.realm_config.realm_id as u32,
                0,
                0,
                ProvingJobCircuitType::GUTAVerifyToCap,
                ProvingJobDataType::InputWitness,
                0,
            );

            let bp = self
                .store
                .get_user_bottom_tree_merkle_proof(
                    COORDINATOR_USER_TREE_HEIGHT,
                    current_checkpoint_id,
                    (guta_agg_header.state_transition.node_index.to_canonical_u64())
                        << (GLOBAL_USER_TREE_HEIGHT as u64 - guta_agg_header.state_transition.node_level.to_canonical_u64()),
                )
                .await?;

            let top_line_siblings_len = (guta_agg_header.state_transition.node_level.to_canonical_u64() - COORDINATOR_USER_TREE_HEIGHT as u64) as usize;

            let good_sibs = bp.siblings[(bp.siblings.len() - top_line_siblings_len)..].to_vec();

            let input = CircuitInputWithDependencies {
                input: VerifyGUTAToCapCircuitInputSimple {
                    guta_proof_header: guta_agg_header,
                    top_line_siblings: good_sibs,
                },
                dependencies: vec![combined_update.root_job_id.get_output_id()],
            };

            tracing::debug!("GUTA proof input: {}", serde_json::to_string(&input)?);

            self.proof_store
                .set_bytes_by_id(w_id.get_input_witness_id(), &bincode::serialize(&input)?)
                .await?;
            self.task_store
                .register_dependencies(w_id.get_output_id(), &[combined_update.root_job_id.get_output_id()])
                .await;

            combined_update.root_job_id = w_id;
            combined_update.header.root_job_id = w_id;
        }

        info!("🔄 Collecting user contract and state tree updates");
        for user_update in user_updates {
            combined_update
                .user_contract_tree_updates
                .extend_from_slice(&user_update.user_contract_tree_updates);
            combined_update
                .contract_state_tree_updates
                .extend_from_slice(&user_update.contract_state_tree_updates);
            combined_update.updated_users.push(user_update.new_user_leaf);
        }

        combined_update.global_user_tree_updates = tree_update_builder.finalize();
        info!(
            tree_updates_count = combined_update.global_user_tree_updates.len(),
            updated_users_count = combined_update.updated_users.len(),
            "📊 Finalized tree updates and user collections"
        );

        Ok(combined_update)
    }

    async fn sync_global_block_updates_only(
        &self,
        from_checkpoint: u64,
        to_checkpoint: u64,
    ) -> anyhow::Result<()> {
        if from_checkpoint == to_checkpoint {
            return Ok(());
        }
        let updates = self.coordinator_client
            .get_latest_block_updates_from_coordinator(self.realm_config.realm_id as u64, from_checkpoint, to_checkpoint)
            .await?;
        for update in updates.iter() {
            self.apply_only_global_block_update_dangerous(update).await?;
        }
        Ok(())
    }

    async fn sync_realm_deltas_and_block_updates(
       &self,
        from_realm_root: QHashOut<F>,
        to_realm_root: QHashOut<F>,
        from_checkpoint: u64,
        to_checkpoint: u64,
    ) -> anyhow::Result<()> {
        let realm_id = self.realm_config.realm_id as u64;
        let deltas = self.sync_latest_realm_deltas_from_peers(realm_id, from_realm_root, to_realm_root)
            .await?;
        let updates = self.coordinator_client
            .get_latest_block_updates_from_coordinator(realm_id, from_checkpoint, to_checkpoint)
            .await?;

        if updates.len() >= deltas.len() {
            for (delta, update) in deltas.iter().zip(updates.iter()) {
                self.apply_realm_deltas(delta, update).await?;
            }
            for update in updates.iter().skip(deltas.len()) {
                self.apply_only_global_block_update_dangerous(update).await?;
            }
        } else {
            for (update, delta) in updates.iter().zip(deltas.iter()) {
                self.apply_realm_deltas(delta, update).await?;
            }
            for delta in deltas.iter().skip(updates.len()) {
                self.apply_only_realm_deltas_dangerous(delta).await?;
            }
        }
        Ok(())
    }

    async fn sync_finalized_to_latest_checkpoint(
        &self,
        from_checkpoint: u64,
        from_realm_root: QHashOut<F>,
    ) -> anyhow::Result<()> {
        let realm_id = self.realm_config.realm_id as u64;
        info!(
            realm_id = realm_id,
            from_checkpoint = from_checkpoint,
            from_realm_root = %serde_json::to_string_pretty(&from_realm_root).unwrap(),
            "🔄 Starting sync_finalized_to_latest_checkpoint"
        );

        let updates = self.coordinator_client
            .get_latest_block_updates_from_coordinator(realm_id, from_checkpoint, MAX_CHECKPOINT)
            .await?;
        info!(
            updates_count = updates.len(),
            "📥 Retrieved block updates from coordinator"
        );

        if updates.len() <= 1 {
            info!("⏭️ No updates to sync (updates.len() <= 1), skipping");
            return Ok(());
        }

        if updates[0].realm_root != from_realm_root {
            error!(
                first_update_realm_root = %serde_json::to_string_pretty(&updates[0].realm_root).unwrap(),
                expected_from_realm_root = %serde_json::to_string_pretty(&from_realm_root).unwrap(),
                "❌ First update realm_root mismatch"
            );
            anyhow::bail!("First update realm_root {} does not match from_realm_root {}", updates[0].realm_root, from_realm_root);
        }
        info!("✅ Validated first update realm_root matches from_realm_root");

        let mut prev_realm_root = updates[0].realm_root;
        for (i, update) in updates.iter().enumerate().skip(1) {
            let checkpoint_id = update.compact.l2_block_state.checkpoint_id;
            info!(
                update_index = i,
                checkpoint_id = checkpoint_id,
                update_realm_root = %serde_json::to_string_pretty(&update.realm_root).unwrap(),
                prev_realm_root = %serde_json::to_string_pretty(&prev_realm_root).unwrap(),
                "🔄 Processing update {}/{}",
                i,
                updates.len() - 1
            );

            if update.realm_root != prev_realm_root {
                info!("🔄 Realm root changed, applying realm deltas + global update");
                if let Some(delta) = self.load_update_delta_records(realm_id, update.realm_root).await? {
                    info!(
                        delta_record = %serde_json::to_string_pretty(&delta).unwrap(),
                        "📋 Found delta record, applying realm deltas"
                    );
                    self.apply_realm_deltas(&delta, update).await?;
                    info!("✅ Applied realm deltas successfully");
                } else {
                    error!(
                        realm_root = %serde_json::to_string_pretty(&update.realm_root).unwrap(),
                        checkpoint_id = checkpoint_id,
                        "❌ Missing delta record"
                    );
                    anyhow::bail!("Missing delta record for realm_root {} at checkpoint {}", update.realm_root, checkpoint_id);
                }
            } else {
                info!("➡️ Realm root unchanged, applying only global block update");
                self.apply_only_global_block_update_dangerous(update).await?;
                info!("✅ Applied global block update successfully");
            }

            prev_realm_root = update.realm_root;
        }

        info!("✅ sync_finalized_to_latest_checkpoint completed successfully");
        Ok(())
    }

    async fn process_genesis_user_states(
        &self,
        genesis_config: &GenesisConfig<F>,
    ) -> anyhow::Result<()> {
        let realm_id = self.realm_config.realm_id as u64;
        tracing::info!("Processing genesis state for realm {}", realm_id);

        let mut user_contract_states: HashMap<u64, HashMap<u64, Vec<(u64, QHashOut<F>)>>> = HashMap::new();
        let mut user_id_to_register_id: HashMap<u64, u64> = HashMap::new();

        for (contract_id, users) in genesis_config.get_all_contracts() {
            for (register_id, user_state) in users {
                let user_id = get_user_id_from_registration_id(*register_id);

                if self.realm_config.includes_user_id(user_id) {
                    user_id_to_register_id.insert(user_id, *register_id);

                    let mut contract_slots = Vec::new();
                    for (slot_id, slot_value) in &user_state.slots {
                        contract_slots.push((*slot_id, *slot_value));
                    }

                    if !contract_slots.is_empty() {
                        user_contract_states.entry(user_id).or_insert_with(HashMap::new)
                            .insert(*contract_id, contract_slots);
                    }
                }
            }
        }

        let genesis_users = genesis_config.get_genesis_users();

        let mut realm_updates = Vec::new();
        for (user_id, contracts) in user_contract_states {
            let register_id = user_id_to_register_id[&user_id];

            let mut user_contract_tree_root = QEDHasher::get_zero_hash(GLOBAL_CONTRACT_TREE_HEIGHT.into());

            for (contract_id, slots) in contracts {
                let mut contract_state_root = QEDHasher::get_zero_hash(MAX_CONTRACT_STATE_TREE_HEIGHT.into());
                for (slot_id, slot_value) in slots {
                    contract_state_root = self.store.set_user_state_tree_leaf_hash(0, user_id, contract_id as u32, MAX_CONTRACT_STATE_TREE_HEIGHT, slot_id, slot_value)?.new_root;
                }

                user_contract_tree_root = self.store.set_user_contract_tree_leaf_hash(
                    0,
                    user_id,
                    contract_id as u32,
                    contract_state_root,
                )?.new_root;
            }

            let user_leaf = QEDUserLeaf {
                public_key: genesis_users[register_id as usize].get_public_key::<QEDHasher>(),
                user_state_tree_root: user_contract_tree_root,
                balance: F::ZERO,
                nonce: F::ZERO,
                last_checkpoint_id: F::ZERO,
                event_index: F::ZERO,
                user_id: F::from_canonical_u64(user_id),
            };

            let user_leaf_hash = user_leaf.qfhash::<QEDHasher>();

            self.store.set_user_leaf_data(0, &user_leaf)
                .map_err(|e| anyhow::anyhow!("Failed to set user leaf data for user {}: {}", user_id, e))?;

            let realm_update = QMerkleNode {
                key: SimpleMerkleNodeKey {
                    level: GLOBAL_USER_TREE_HEIGHT,
                    index: user_id,
                },
                value: user_leaf_hash,
            };
            realm_updates.push(realm_update);

            tracing::info!("✅ Genesis state set for user {} with UCT root {} (realm {})",
                  user_id, user_contract_tree_root, realm_id);
        }

        self.store.injest_user_tree_nodes_imm(0, COORDINATOR_USER_TREE_HEIGHT, &realm_updates).await?;

        tracing::info!("Genesis state initialization completed for realm {}", realm_id);

        Ok(())
    }

}

#[async_trait]
impl GlobalUserTreeMerkleReader<F> for RealmProcessorV2 {
    async fn get_sub_tree_merkle_proof<H: MerkleHasher<QHashOut<F>>>(
        &self,
        checkpoint_id: u64,
        from_level: u8,
        from_index: u64,
        to_level: u8,
    ) -> anyhow::Result<(u64, MerkleProofCore<QHashOut<F>>)> {
        self.store.get_sub_tree_merkle_proof::<H>(checkpoint_id, from_level, from_index, to_level).await
    }

    async fn resolve_delta_merkle_proofs_for_nca<H: MerkleHasher<QHashOut<F>>>(
        &self,
        tree_update_builder: &mut SimpleTreeUpdateBuilder<F>,
        checkpoint_id: u64,
        left_node: GenericTreeNodeUpdate<F>,
        right_node: GenericTreeNodeUpdate<F>,
        to_level: u8,
    ) -> anyhow::Result<(GenericTreeNodeUpdate<F>, DeltaMerkleProofCore<QHashOut<F>>, DeltaMerkleProofCore<QHashOut<F>>)> {
        self.store.resolve_delta_merkle_proofs_for_nca::<H>(tree_update_builder, checkpoint_id, left_node, right_node, to_level).await
    }

    async fn get_multiple_delta_merkle_proofs<H: MerkleHasher<QHashOut<F>>>(
        &self,
        tree_update_builder: &mut SimpleTreeUpdateBuilder<F>,
        checkpoint_id: u64,
        updates: Vec<GenericTreeNodeUpdate<F>>,
        to_level: Option<u8>,
    ) -> anyhow::Result<(GenericTreeNodeUpdate<F>, Vec<DeltaMerkleProofCore<QHashOut<F>>>)> {
        self.store.get_multiple_delta_merkle_proofs::<H>(tree_update_builder, checkpoint_id, updates, to_level).await
    }
}

impl RealmProcessorStateClient<F> for RealmProcessorV2 {
    async fn set_shared_checkpoint_info(&self, queue_id: UniqueQueueId, info: StagingCheckpointInfo) -> anyhow::Result<()> {
        StagingCheckpointInfoStore::<QEDStore>::set_checkpoint_info(&self.store, queue_id.uuid, queue_id.id, &info)?;
        Ok(())
    }

    async fn get_shared_queue_id(&self) -> anyhow::Result<UniqueQueueId> {
        if let Some((uuid, checkpoint_id, _info)) = StagingCheckpointInfoStore::<QEDStore>::get_latest_checkpoint_info_with_uuid(&self.store)? {
            Ok(UniqueQueueId {
                id: checkpoint_id,
                uuid,
            })
        } else {
            anyhow::bail!("No staging checkpoint info found")
        }
    }

    async fn save_update_delta_record(&self, data: &RealmProcessorCombinedUpdate<F>) -> anyhow::Result<()> {
        StagingDeltaRecordStore::<QEDStore>::set_delta_record(&self.store, data.new_realm_root, data.realm_id as u32, data)?;
        Ok(())
    }

    async fn load_update_delta_records(&self, _realm_id: u64, target_realm_root: QHashOut<F>) -> anyhow::Result<Option<RealmProcessorCombinedUpdate<F>>> {
        let records = StagingDeltaRecordStore::<QEDStore>::get_delta_records_for_realm_root(&self.store, target_realm_root)?;
        if records.is_empty() {
            Ok(None)
        } else {
            Ok(records.into_iter().next())
        }
    }

    async fn prune_update_delta_records_from_target_root(&self, realm_end_root: QHashOut<F>) -> anyhow::Result<usize> {
        StagingDeltaRecordStore::<QEDStore>::delete_delta_records_for_realm_root(&self.store, realm_end_root)
    }

    async fn propagate_update_delta_record_to_peers(&self, _data: &RealmProcessorCombinedUpdate<F>) -> anyhow::Result<()> {
        Ok(())
    }

    async fn sync_latest_realm_deltas_from_peers(&self, _realm_id: u64, _from_realm_root: QHashOut<F>, _to_realm_root: QHashOut<F>) -> anyhow::Result<Vec<RealmProcessorCombinedUpdate<F>>> {
        Ok(vec![])
    }

    async fn apply_realm_deltas(&self, delta: &RealmProcessorCombinedUpdate<F>, global_block_update: &GlobalBlockUpdateFromCoordinator<F>) -> anyhow::Result<()> {
        let canonical_checkpoint_id = global_block_update.compact.l2_block_state.checkpoint_id;

        info!(
            canonical_checkpoint_id = canonical_checkpoint_id,
            local_checkpoint_id = delta.local_checkpoint_id,
            queue_id = delta.queue_id,
            queue_uuid = %delta.queue_uuid,
            "🚀 Starting apply_realm_deltas"
        );

        debug!(
            user_updates_count = delta.updated_users.len(),
            "📝 Ingesting user leaves batch"
        );
        self.store.injest_user_leaves_batch_imm(canonical_checkpoint_id, &delta.updated_users).await?;

        let user_tree_nodes: Vec<QMerkleNode<F>> = delta.updated_users
            .iter()
            .map(|update| {
                trace!(
                    user_id = update.user_id.to_canonical_u64(),
                    user_hash = %format!("{:?}", update.qfhash::<QEDHasher>()),
                    "🌳 Processing user tree node"
                );
                QMerkleNode {
                    key: SimpleMerkleNodeKey {
                        level: GLOBAL_USER_TREE_HEIGHT,
                        index: update.user_id.to_canonical_u64(),
                    },
                    value: update.qfhash::<QEDHasher>(),
                }
            })
            .collect();

        debug!(
            user_tree_nodes_count = user_tree_nodes.len(),
            coordinator_tree_height = COORDINATOR_USER_TREE_HEIGHT,
            "🌲 Ingesting user tree nodes"
        );
        self.store.injest_user_tree_nodes_imm(canonical_checkpoint_id, COORDINATOR_USER_TREE_HEIGHT, &user_tree_nodes).await?;

        debug!(
            contract_state_updates_count = delta.contract_state_tree_updates.len(),
            "📊 Processing contract state tree updates"
        );

        let contract_state_nodes: Vec<_> = delta.contract_state_tree_updates
            .iter()
            .enumerate()
            .map(|(i, update)| {
                trace!(
                    update_index = i,
                    user_id = update.user_id,
                    contract_id = update.contract_id,
                    level = update.level,
                    index = update.index,
                    new_value = %format!("{:?}", update.new_value),
                    "🔄 Preparing contract state update"
                );

                KVQPair {
                    key: KVQMerkleNodeKey::<USER_CONTRACT_STATE_TREE_TABLE_TYPE> {
                        tree_id: CONTRACT_STATE_TREE_ID,
                        primary_id: update.user_id,
                        secondary_id: update.contract_id,
                        level: update.level,
                        index: update.index,
                        checkpoint_id: canonical_checkpoint_id,
                    },
                    value: update.new_value,
                }
            })
            .collect();

        if !contract_state_nodes.is_empty() {
            BaseContractStateTreeStore::<QEDStore>::set_nodes(&self.store, &contract_state_nodes)?;
        }

        info!(
            contract_state_updates_applied = delta.contract_state_tree_updates.len(),
            "✅ Contract state tree updates applied"
        );

        debug!(
            user_contract_updates_count = delta.user_contract_tree_updates.len(),
            "👤 Processing user contract tree updates"
        );

        let user_contract_nodes: Vec<_> = delta.user_contract_tree_updates
            .iter()
            .enumerate()
            .map(|(i, update)| {
                trace!(
                    update_index = i,
                    user_id = update.user_id,
                    level = update.level,
                    index = update.index,
                    new_value = %format!("{:?}", update.new_value),
                    "🔄 Preparing user contract tree update"
                );

                KVQPair {
                    key: UserContractTreeStore::<QEDStore>::new_node_key_sfc(canonical_checkpoint_id, update.user_id, update.level, update.index as u64),
                    value: update.new_value,
                }
            })
            .collect();

        if !user_contract_nodes.is_empty() {
            UserContractTreeStore::<QEDStore>::set_nodes(&self.store, &user_contract_nodes)?;
        }

        info!(
            user_contract_updates_applied = delta.user_contract_tree_updates.len(),
            "✅ User contract tree updates applied"
        );

        info!("🌍 Applying global block update");
        self.apply_only_global_block_update_dangerous(global_block_update).await?;

        let queue_id = UniqueQueueId {
            id: delta.queue_id,
            uuid: delta.queue_uuid,
        };

        let staging_info = StagingCheckpointInfo {
            local_checkpoint_id: delta.local_checkpoint_id,
            canonical_checkpoint_id,
        };

        debug!(
            queue_id = queue_id.id,
            queue_uuid = %queue_id.uuid,
            local_checkpoint_id = staging_info.local_checkpoint_id,
            canonical_checkpoint_id = staging_info.canonical_checkpoint_id,
            "📋 Setting shared checkpoint info"
        );
        self.set_shared_checkpoint_info(queue_id, staging_info).await?;

        info!(
            canonical_checkpoint_id = canonical_checkpoint_id,
            total_user_updates = delta.updated_users.len(),
            total_contract_state_updates = delta.contract_state_tree_updates.len(),
            total_user_contract_updates = delta.user_contract_tree_updates.len(),
            "✅ apply_realm_deltas completed successfully"
        );

        Ok(())
    }

    async fn apply_only_global_block_update_dangerous(&self, global_block_update: &GlobalBlockUpdateFromCoordinator<F>) -> anyhow::Result<()> {
        let merkle_proofs = global_block_update.compact.get_registered_user_merkle_proofs::<QEDHasher>();
        let start_registration_user_id = global_block_update.compact.l2_block_state.next_user_id - (global_block_update.compact.registered_users.len() as u64);
        let realm_users: Vec<_> = merkle_proofs
            .into_iter()
            .enumerate()
            .filter_map(|(i, proof)| {
                let registration_id = start_registration_user_id + (i as u64);
                let real_id = get_user_id_from_registration_id(registration_id);
                if self.realm_config.includes_user_id(real_id) {
                    Some(proof)
                } else {
                    None
                }
            })
            .collect();

        if !realm_users.is_empty() {
            tracing::info!("Adding {} new pending users to edge queue for realm {}", realm_users.len(), self.realm_config.realm_id);
        }

        let sync_info = global_block_update.compact.clone().to_sync_info::<QEDHasher>();
        self.store.injest_checkpoint_sync_data_imm(sync_info).await?;
        Ok(())
    }

    async fn apply_only_realm_deltas_dangerous(&self, delta: &RealmProcessorCombinedUpdate<F>) -> anyhow::Result<()> {
        self.store.injest_user_leaves_batch_imm(delta.queue_id, &delta.updated_users).await?;
        let user_tree_nodes: Vec<QMerkleNode<F>> = delta.updated_users
            .iter()
            .map(|update| QMerkleNode {
                key: SimpleMerkleNodeKey {
                    level: GLOBAL_USER_TREE_HEIGHT,
                    index: update.user_id.to_canonical_u64(),
                },
                value: update.qfhash::<QEDHasher>(),
            })
            .collect();
        self.store.injest_user_tree_nodes_imm(delta.queue_id, COORDINATOR_USER_TREE_HEIGHT, &user_tree_nodes).await?;

        for update in &delta.contract_state_tree_updates {
            let nodes = vec![KVQPair {
                key: KVQMerkleNodeKey::<USER_CONTRACT_STATE_TREE_TABLE_TYPE> {
                    tree_id: CONTRACT_STATE_TREE_ID,
                    primary_id: update.user_id,
                    secondary_id: update.contract_id,
                    level: update.level,
                    index: update.index,
                    checkpoint_id: delta.queue_id,
                },
                value: update.new_value,
            }];
            BaseContractStateTreeStore::<QEDStore>::set_nodes(&self.store, &nodes)?;
        }

        for update in &delta.user_contract_tree_updates {
            let nodes = vec![KVQPair {
                key: UserContractTreeStore::<QEDStore>::new_node_key_sfc(delta.queue_id, update.user_id, update.level, update.index as u64),
                value: update.new_value,
            }];
            UserContractTreeStore::<QEDStore>::set_nodes(&self.store, &nodes)?;
        }

        Ok(())
    }

    async fn get_latest_checkpoint_id(&self) -> anyhow::Result<u64> {
        let sync_info = self.store.get_latest_l2_block_state().await?;
        Ok(sync_info.checkpoint_id)
    }

    async fn get_latest_checkpoint_and_realm_root(&self) -> anyhow::Result<(u64, QHashOut<F>)> {
        let sync_info = self.store.get_latest_l2_block_state().await?;
        let checkpoint_id = sync_info.checkpoint_id;

        let realm_root = self
            .store
            .get_user_bottom_tree_merkle_proof(
                self.realm_config.realm_root_level,
                checkpoint_id,
                (self.realm_config.realm_id as u64) << (REALM_USER_TREE_HEIGHT as u64)
            )
            .await?
            .root;

        Ok((checkpoint_id, realm_root))
    }

}

impl Retryable for RealmProcessorV2 {}
