// std
use std::sync::atomic::Ordering;
use std::sync::Arc;

use anyhow::{bail};
use chrono::Utc;
use rand::RngCore;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tracing::{debug, error, info};

use kvq::traits::KVQSerializable;
use plonky2::plonk::config::PoseidonGoldilocksConfig;
use plonky2::plonk::proof::ProofWithPublicInputs;

// qed_core
use qed_core::config::network_constants::COORD_STATUS_CHANNEL_ID;
use qed_core::data::qhashout::QHashOut;
use qed_core::job::drain_queue::{
    CheckpointDrainQueueConsumerSyncImm, CheckpointDrainQueueEmitterAsyncImm,
    WithDrainQueueMetadata,
};
use qed_core::job::id::ProvingJobCircuitType;
use qed_core::job::traits::QProofStoreWriterAsyncImm;
use qed_core::job::worker_queue::ProvingDispatcher;

// qed_crypto
use qed_crypto::hash::merkle::core::MerkleProofCore;
use qed_crypto::signature::zk::data::ZKPublicKeyInfo;

// qed_data
use qed_data::guta::api::SubmitGUTARealmResultAPINoProofInput;
use qed_data::qblock::cmds::deploy_contract::QBCDeployContract;
use qed_data::qdata::checkpoint::{
    QEDCheckpointGlobalStateRoots, QEDCheckpointLeaf, QEDL2BlockState,
};
use qed_data::qdata::contract::{ContractCodeDefinition, QEDContractLeaf};
use qed_data::qdata::user::QEDUserLeaf;
use qed_data::qsync::coordinator::QEDCheckpointSyncInfoCompact;

// qed_node
use qed_node::coordinator::state::user_map::{get_node_redis_pool, get_user_id_by_pubkey};
use qed_node::nimpl::worker_queue_redis::redis_queue::{
    CEQueueNotification, RedisQueue, CE_NOTIFICATIONS,
};
use qed_node_common::coordinator::CheckpointSyncInfo;
// qed_store
use qed_store::config::store_config::{QEDFelt, QEDHasher};
use qed_store::node::coordinator::store_traits::QEDCoordinatorStoreReaderAsync;
use qed_store::traits::qdatastore::qmetadata::QMetaDataStoreReaderSync;
use qed_store::traits::qdatastore::qtreedata::QTreeDataStoreReaderSync;
use reth_libmdbx::error;
// crate inner
use crate::communicate::GlobalCoordinatorStatus;
use crate::context::{with_temp_ctx_read_async, GLOBAL_COORD_EDGE_STATE};
use crate::edge::context::LATEST_CHECKPOINT_ID;
use crate::{CoordinatorEdgeArgs};

type F = QEDFelt;
type C = PoseidonGoldilocksConfig;
const D: usize = 2;

#[derive(Clone)]
pub struct CoordinatorEdgeHandler {
    notify_queue: RedisQueue,
    cp_listener: Arc<Mutex<Option<JoinHandle<()>>>>,
}

impl CoordinatorEdgeHandler {
    pub fn new(args: CoordinatorEdgeArgs) -> anyhow::Result<Self> {
        let redis_uri = args.coordinator_redis_uri.clone();
        Ok(Self {
            notify_queue: RedisQueue::new(redis_uri.as_str())?,
            cp_listener: Arc::new(Mutex::new(None)),
        })
    }
    ///receive StartSync notification from CP
    pub async fn spawn_cp_sync_listener(&self) -> anyhow::Result<()> {
        // note: run this only once
        if self.cp_listener.lock().await.is_some() {
            return Ok(());
        }
        let handle = tokio::spawn(async move {
            let mut last_logged_checkpoint = None;

            loop {
                let fallback = match get_latest_status_from_global_queue().await {
                    Ok(Some(status)) => {
                        let latest = LATEST_CHECKPOINT_ID.load(Ordering::Relaxed);

                        if Some(status.confirmed_checkpoint_id) != last_logged_checkpoint {
                            debug!("🔔 Detected new checkpoint sync status: {:?}", status);
                            last_logged_checkpoint = Some(status.confirmed_checkpoint_id);
                        }

                        if status.confirmed_checkpoint_id > latest {
                            info!(
                                "🔄 Updating local checkpoint from {} → {}",
                                latest, status.confirmed_checkpoint_id
                            );
                            LATEST_CHECKPOINT_ID
                                .store(status.confirmed_checkpoint_id, Ordering::Relaxed);
                            info!(
                                "⭐ Coordinator Edge now updated to {}",
                                status.confirmed_checkpoint_id
                            );
                        } else {
                            debug!(
                                "ℹ️ No new confirmed checkpoint detected, local = {}, redis = {}",
                                latest, status.confirmed_checkpoint_id
                            );
                        }

                        false
                    }

                    Ok(None) => {
                        debug!("⚠️ Redis queue empty or status missing. Fallback to DB.");
                        true
                    }

                    Err(e) => {
                        error!("❌ Redis query failed: {:?}", e);
                        true
                    }
                };
                if fallback {
                    //it means redis queue is empty or error, fallback to db
                    if let Err(e) = recover_latest_checkpoint_from_db_if_needed().await {
                        error!("❌ Failed to recover checkpoint from DB: {:?}", e);
                    }
                }
                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
            }
        });
        *self.cp_listener.lock().await = Some(handle);
        Ok(())
    }

    pub async fn register_user(
        &self,
        zk_user_info: ZKPublicKeyInfo<QEDFelt>,
    ) -> anyhow::Result<()> {
        let hash = zk_user_info.public_key_param;
        let pk_hex = hex::encode(hash.to_bytes()?);

        let redis_pool = get_node_redis_pool()?;
        let result = get_user_id_by_pubkey(redis_pool.as_ref(), &pk_hex).await?;

        if let Some(user_id) = result {
            info!("🛑 User already registered in Redis, user_id = {}", user_id);
            return Ok(());
        }
        info!("🆕 User not found in Redis. Starting new registration.");

        with_temp_ctx_read_async::<_, _, _, C, D>(|ctx| {
            let queue = ctx.checkpoint_queue.clone();
            let zk_user = zk_user_info.clone();

            async move {
                queue.cdq_push_imm(zk_user).await?;
                info!("✅ User pushed to checkpoint queue.");
                Ok(())
            }
        })
            .await?;
        Ok(())
    }

    pub async fn get_user_id(&self, qhash: QHashOut<QEDFelt>) -> anyhow::Result<u64> {
        let pubkey_hex = hex::encode(qhash.to_bytes()?);
        let redis_pool = get_node_redis_pool()?;

        let result = get_user_id_by_pubkey(redis_pool.as_ref(), &pubkey_hex).await;

        let Some(user_id) = result? else {
            error!("❌ User not found");
            bail!("User not found");
        };

        Ok(user_id)
    }
    pub async fn deploy_contract(
        &self,
        contract: QBCDeployContract<QEDFelt>,
    ) -> anyhow::Result<()> {
        let next_checkpoint_id = LATEST_CHECKPOINT_ID.load(Ordering::Relaxed) + 1;
        with_temp_ctx_read_async::<_, _, _, C, D>(|ctx| {
            let queue = ctx.checkpoint_queue.clone();
            let config = ctx.coordinator_config.clone();
            let contract = contract.clone(); // 如果需要

            async move {
                let with_root = contract.into_with_whitelist_root::<QEDHasher>()?;

                let cd_for_queue = WithDrainQueueMetadata::new_params(
                    config.deploy_contract_channel_id,
                    next_checkpoint_id,
                    rand::thread_rng().next_u64(),
                    with_root,
                );

                queue.cdq_push_imm(cd_for_queue).await?;
                Ok(())
            }
        })
            .await
    }
    pub async fn submit_guta(
        &self,
        input: SubmitGUTARealmResultAPINoProofInput<QEDFelt>,
        proof: ProofWithPublicInputs<QEDFelt, PoseidonGoldilocksConfig, 2>,
    ) -> anyhow::Result<()> {
        let (checkpoint_queue, proof_store, config, verifier) =
            with_temp_ctx_read_async::<_, _, _, C, D>(|ctx| {
                let checkpoint_queue = ctx.checkpoint_queue.clone();
                let proof_store = ctx.proof_store.clone();
                let config = ctx.coordinator_config.clone();
                let verifier = ctx.proof_verifier.clone();

                std::future::ready(Ok((checkpoint_queue, proof_store, config, verifier)))
            })
                .await?;
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
        let old_root = with_temp_ctx_read_async::<_, _, _, C, D>(|ctx| async move {
            match ctx
                .store_reader
                .get_user_latest_top_tree_cap_root(config.realm_root_level, input.realm_id)
                .await
            {
                Ok(root) => Ok(root),
                Err(e) => {
                    error!("❌ Failed to get old root: {:?}", e);

                    if let Some(mdbx_err) = e.downcast_ref::<error::Error>() {
                        error!("❌ MDBX explain: {}", mdbx_err.explain());
                    }

                    let mut source = e.source();
                    while let Some(err) = source {
                        error!("⛓ Caused by: {}", err);
                        source = err.source();
                    }

                    Err(anyhow::anyhow!("Failed to get old root"))
                }
            }
        })
            .await?;

        info!(
            "old root from db: {:?}, hex = {:?}",
            old_root,
            hex::encode(old_root.to_bytes()?)
        );
        info!("old root from realm: {:?}", input.top_line_proof.old_root);
        if old_root != input.top_line_proof.old_root && old_root != input.top_line_proof.new_root {
            anyhow::bail!("invalid top line proof old value from realm");
        }

        // build queue item
        let queue_item =
            input.to_queue_item(config.guta_channel_id, config.realm_root_level as u32);
        let proof_id = queue_item.proof_id;
        info!(
            "🚀 Pushing GUTA result to drain queue, realm_id = {}",
            proof_id.task_index
        );

        // write to proof store
        proof_store.set_proof_by_id(proof_id, &proof).await?;
        info!("✅ wrote guta result to proof store");
        checkpoint_queue.cdq_push_imm(queue_item).await?;
        info!("✅ wrote guta result to proof store end");

        Ok(())
    }

    pub async fn build_block(&self) -> anyhow::Result<()> {
        let next_checkpoint = LATEST_CHECKPOINT_ID.load(Ordering::Relaxed) + 1;
        self.notify_queue.clone().dispatch(
            CE_NOTIFICATIONS,
            CEQueueNotification::StartProduceBlock { next_checkpoint },
        )?;
        info!("☎️ build block {} cmd have send to CP", next_checkpoint);
        Ok(())
    }

    pub async fn get_checkpoint_sync_info(
        &self,
        request_checkpoint_id: u64,
    ) -> anyhow::Result<CheckpointSyncInfo<F>> {
        let latest = LATEST_CHECKPOINT_ID.load(Ordering::Relaxed);

        if request_checkpoint_id > latest {
            bail!(
                "Requested checkpoint_id {} exceeds latest local checkpoint_id {}",
                request_checkpoint_id,
                latest
            );
        }

        let compact = with_temp_ctx_read_async::<_, _, _, C, D>(|ctx| async move {
            QEDCoordinatorStoreReaderAsync::get_checkpoint_sync_info_compact(
                &*ctx.store_reader,
                request_checkpoint_id,
            )
                .await
        })
            .await?;
        let sync_info = CheckpointSyncInfo {
            latest_checkpoint_id: latest,
            description: None,
            source_coordinator_edge_id: None,
            sync_timestamp: Utc::now().timestamp() as u64,
            compact,
        };
        Ok(sync_info)
    }
    // async fn get_contract_leaf_data(&self, contract_id: u64) -> anyhow::Result<QEDContractLeaf<F>>;
    pub async fn get_contract_leaf_data(
        &self,
        contract_id: u64,
    ) -> anyhow::Result<QEDContractLeaf<QEDFelt>> {
        with_temp_ctx_read_async::<_, _, _, C, D>(|ctx| async move {
            QEDCoordinatorStoreReaderAsync::get_contract_leaf_data(&*ctx.store_reader, contract_id)
                .await
        })
            .await
    }
    // async fn get_contract_leaf_data_f(&self, contract_id: F) -> anyhow::Result<QEDContractLeaf<F>>;
    pub async fn get_contract_leaf_data_f(
        &self,
        contract_id: F,
    ) -> anyhow::Result<QEDContractLeaf<F>> {
        with_temp_ctx_read_async::<_, _, _, C, D>(|ctx| async move {
            QEDCoordinatorStoreReaderAsync::get_contract_leaf_data_f(
                &*ctx.store_reader,
                contract_id,
            )
                .await
        })
            .await
    }
    // async fn get_checkpoint_leaf_data(&self, checkpoint_id: u64) -> anyhow::Result<QEDCheckpointLeaf<F>>;
    pub async fn get_checkpoint_leaf_data(
        &self,
        checkpoint_id: u64,
    ) -> anyhow::Result<QEDCheckpointLeaf<QEDFelt>> {
        with_temp_ctx_read_async::<_, _, _, C, D>(|ctx| async move {
            QEDCoordinatorStoreReaderAsync::get_checkpoint_leaf_data(
                &*ctx.store_reader,
                checkpoint_id,
            )
                .await
        })
            .await
    }
    // async fn get_checkpoint_leaf_data_f(&self, checkpoint_id: F) -> anyhow::Result<QEDCheckpointLeaf<F>>;
    pub async fn get_checkpoint_leaf_data_f(
        &self,
        checkpoint_id: F,
    ) -> anyhow::Result<QEDCheckpointLeaf<F>> {
        with_temp_ctx_read_async::<_, _, _, C, D>(|ctx| async move {
            QEDCoordinatorStoreReaderAsync::get_checkpoint_leaf_data_f(
                &*ctx.store_reader,
                checkpoint_id,
            )
                .await
        })
            .await
    }
    // async fn get_contract_code_definition(&self, contract_id: u64) -> anyhow::Result<ContractCodeDefinition>;
    pub async fn get_contract_code_definition(
        &self,
        contract_id: u64,
    ) -> anyhow::Result<ContractCodeDefinition> {
        with_temp_ctx_read_async::<_, _, _, C, D>(|ctx| async move {
            QEDCoordinatorStoreReaderAsync::get_contract_code_definition(
                &*ctx.store_reader,
                contract_id,
            )
                .await
        })
            .await
    }
    // async fn get_contract_code_definition_f(&self, contract_id: F) -> anyhow::Result<ContractCodeDefinition>;
    pub async fn get_contract_code_definition_f(
        &self,
        contract_id: F,
    ) -> anyhow::Result<ContractCodeDefinition> {
        with_temp_ctx_read_async::<_, _, _, C, D>(|ctx| async move {
            QEDCoordinatorStoreReaderAsync::get_contract_code_definition_f(
                &*ctx.store_reader,
                contract_id,
            )
                .await
        })
            .await
    }
    // async fn get_latest_l2_block_state(&self) -> anyhow::Result<QEDL2BlockState>;
    pub async fn get_latest_l2_block_state(&self) -> anyhow::Result<QEDL2BlockState> {
        with_temp_ctx_read_async::<_, _, _, C, D>(|ctx| async move {
            QEDCoordinatorStoreReaderAsync::get_latest_l2_block_state(&*ctx.store_reader).await
        })
            .await
    }
    // async fn get_l2_block_state(&self, checkpoint_id: u64) -> anyhow::Result<QEDL2BlockState>;
    pub async fn get_l2_block_state(&self, checkpoint_id: u64) -> anyhow::Result<QEDL2BlockState> {
        with_temp_ctx_read_async::<_, _, _, C, D>(|ctx| async move {
            QEDCoordinatorStoreReaderAsync::get_l2_block_state(&*ctx.store_reader, checkpoint_id)
                .await
        })
            .await
    }
    // async fn get_l2_block_state_f(&self, checkpoint_id: F) -> anyhow::Result<QEDL2BlockState>;
    pub async fn get_l2_block_state_f(&self, checkpoint_id: F) -> anyhow::Result<QEDL2BlockState> {
        with_temp_ctx_read_async::<_, _, _, C, D>(|ctx| async move {
            QEDCoordinatorStoreReaderAsync::get_l2_block_state_f(&*ctx.store_reader, checkpoint_id)
                .await
        })
            .await
    }
    // async fn get_user_registration_tree_root(&self, checkpoint_id: u64) -> anyhow::Result<QHashOut<F>>;
    pub async fn get_user_registration_tree_root(
        &self,
        checkpoint_id: u64,
    ) -> anyhow::Result<QHashOut<F>> {
        with_temp_ctx_read_async::<_, _, _, C, D>(|ctx| async move {
            QEDCoordinatorStoreReaderAsync::get_user_registration_tree_root(
                &*ctx.store_reader,
                checkpoint_id,
            )
                .await
        })
            .await
    }
    // async fn get_user_registration_tree_root_f(&self, checkpoint_id: F) -> anyhow::Result<QHashOut<F>>;
    pub async fn get_user_registration_tree_root_f(
        &self,
        checkpoint_id: F,
    ) -> anyhow::Result<QHashOut<F>> {
        with_temp_ctx_read_async::<_, _, _, C, D>(|ctx| async move {
            QEDCoordinatorStoreReaderAsync::get_user_registration_tree_root_f(
                &*ctx.store_reader,
                checkpoint_id,
            )
                .await
        })
            .await
    }
    // async fn get_user_registration_tree_leaf_hash(&self, checkpoint_id: u64, leaf_index: u64) -> anyhow::Result<QHashOut<F>>;
    pub async fn get_user_registration_tree_leaf_hash(
        &self,
        checkpoint_id: u64,
        leaf_index: u64,
    ) -> anyhow::Result<QHashOut<F>> {
        with_temp_ctx_read_async::<_, _, _, C, D>(|ctx| async move {
            QEDCoordinatorStoreReaderAsync::get_user_registration_tree_leaf_hash(
                &*ctx.store_reader,
                checkpoint_id,
                leaf_index,
            )
                .await
        })
            .await
    }
    // async fn get_user_registration_tree_leaf_hash_f(&self, checkpoint_id: F, leaf_index: F) -> anyhow::Result<QHashOut<F>>;
    pub async fn get_user_registration_tree_leaf_hash_f(
        &self,
        checkpoint_id: F,
        leaf_index: F,
    ) -> anyhow::Result<QHashOut<F>> {
        with_temp_ctx_read_async::<_, _, _, C, D>(|ctx| async move {
            QEDCoordinatorStoreReaderAsync::get_user_registration_tree_leaf_hash_f(
                &*ctx.store_reader,
                checkpoint_id,
                leaf_index,
            )
                .await
        })
            .await
    }
    // async fn get_user_registration_tree_merkle_proof(&self, checkpoint_id: u64, leaf_index: u64) -> anyhow::Result<MerkleProofCore<QHashOut<F>>>;
    pub async fn get_user_registration_tree_merkle_proof(
        &self,
        checkpoint_id: u64,
        leaf_index: u64,
    ) -> anyhow::Result<MerkleProofCore<QHashOut<F>>> {
        with_temp_ctx_read_async::<_, _, _, C, D>(|ctx| async move {
            QEDCoordinatorStoreReaderAsync::get_user_registration_tree_merkle_proof(
                &*ctx.store_reader,
                checkpoint_id,
                leaf_index,
            )
                .await
        })
            .await
    }
    // async fn get_user_registration_tree_merkle_proof_f(&self, checkpoint_id: F, leaf_index: F) -> anyhow::Result<MerkleProofCore<QHashOut<F>>>;
    pub async fn get_user_registration_tree_merkle_proof_f(
        &self,
        checkpoint_id: F,
        leaf_index: F,
    ) -> anyhow::Result<MerkleProofCore<QHashOut<F>>> {
        with_temp_ctx_read_async::<_, _, _, C, D>(|ctx| async move {
            QEDCoordinatorStoreReaderAsync::get_user_registration_tree_merkle_proof_f(
                &*ctx.store_reader,
                checkpoint_id,
                leaf_index,
            )
                .await
        })
            .await
    }
    // async fn get_user_tree_root(&self, checkpoint_id: u64) -> anyhow::Result<QHashOut<F>>;
    pub async fn get_user_tree_root(&self, checkpoint_id: u64) -> anyhow::Result<QHashOut<F>> {
        with_temp_ctx_read_async::<_, _, _, C, D>(|ctx| async move {
            QEDCoordinatorStoreReaderAsync::get_user_tree_root(&*ctx.store_reader, checkpoint_id)
                .await
        })
            .await
    }
    // async fn get_user_tree_root_f(&self, checkpoint_id: F) -> anyhow::Result<QHashOut<F>>;
    pub async fn get_user_tree_root_f(&self, checkpoint_id: F) -> anyhow::Result<QHashOut<F>> {
        with_temp_ctx_read_async::<_, _, _, C, D>(|ctx| async move {
            QEDCoordinatorStoreReaderAsync::get_user_tree_root_f(&*ctx.store_reader, checkpoint_id)
                .await
        })
            .await
    }
    // async fn get_user_sub_tree_merkle_proof(&self, checkpoint_id: u64, root_level: u8, leaf_level: u8, leaf_index: u64) -> anyhow::Result<MerkleProofCore<QHashOut<F>>>;
    pub async fn get_user_sub_tree_merkle_proof(
        &self,
        checkpoint_id: u64,
        root_level: u8,
        leaf_level: u8,
        leaf_index: u64,
    ) -> anyhow::Result<MerkleProofCore<QHashOut<F>>> {
        with_temp_ctx_read_async::<_, _, _, C, D>(|ctx| async move {
            QEDCoordinatorStoreReaderAsync::get_user_sub_tree_merkle_proof(
                &*ctx.store_reader,
                checkpoint_id,
                root_level,
                leaf_level,
                leaf_index,
            )
                .await
        })
            .await
    }
    // async fn get_user_top_tree_merkle_proof(&self, checkpoint_id: u64, leaf_level: u8, leaf_index: u64) -> anyhow::Result<MerkleProofCore<QHashOut<F>>>;
    pub async fn get_user_top_tree_merkle_proof(
        &self,
        checkpoint_id: u64,
        leaf_level: u8,
        leaf_index: u64,
    ) -> anyhow::Result<MerkleProofCore<QHashOut<F>>> {
        with_temp_ctx_read_async::<_, _, _, C, D>(|ctx| async move {
            QEDCoordinatorStoreReaderAsync::get_user_top_tree_merkle_proof(
                &*ctx.store_reader,
                checkpoint_id,
                leaf_level,
                leaf_index,
            )
                .await
        })
            .await
    }
    // async fn get_user_top_tree_cap_root(&self, checkpoint_id: u64, cap_level: u8, cap_index: u64) -> anyhow::Result<QHashOut<F>>;
    pub async fn get_user_top_tree_cap_root(
        &self,
        checkpoint_id: u64,
        cap_level: u8,
        cap_index: u64,
    ) -> anyhow::Result<QHashOut<F>> {
        with_temp_ctx_read_async::<_, _, _, C, D>(|ctx| async move {
            QEDCoordinatorStoreReaderAsync::get_user_top_tree_cap_root(
                &*ctx.store_reader,
                checkpoint_id,
                cap_level,
                cap_index,
            )
                .await
        })
            .await
    }
    // async fn get_user_latest_top_tree_cap_root(&self, cap_level: u8, cap_index: u64) -> anyhow::Result<QHashOut<F>>;
    pub async fn get_user_latest_top_tree_cap_root(
        &self,
        cap_level: u8,
        cap_index: u64,
    ) -> anyhow::Result<QHashOut<F>> {
        with_temp_ctx_read_async::<_, _, _, C, D>(|ctx| async move {
            QEDCoordinatorStoreReaderAsync::get_user_latest_top_tree_cap_root(
                &*ctx.store_reader,
                cap_level,
                cap_index,
            )
                .await
        })
            .await
    }
    // async fn get_contract_function_tree_root(&self, checkpoint_id: u64, contract_id: u32) -> anyhow::Result<QHashOut<F>>;
    pub async fn get_contract_function_tree_root(
        &self,
        checkpoint_id: u64,
        contract_id: u32,
    ) -> anyhow::Result<QHashOut<F>> {
        with_temp_ctx_read_async::<_, _, _, C, D>(|ctx| async move {
            QEDCoordinatorStoreReaderAsync::get_contract_function_tree_root(
                &*ctx.store_reader,
                checkpoint_id,
                contract_id,
            )
                .await
        })
            .await
    }
    // async fn get_contract_function_tree_root_f(&self, checkpoint_id: F, contract_id: F) -> anyhow::Result<QHashOut<F>>;
    pub async fn get_contract_function_tree_root_f(
        &self,
        checkpoint_id: F,
        contract_id: F,
    ) -> anyhow::Result<QHashOut<F>> {
        with_temp_ctx_read_async::<_, _, _, C, D>(|ctx| async move {
            QEDCoordinatorStoreReaderAsync::get_contract_function_tree_root_f(
                &*ctx.store_reader,
                checkpoint_id,
                contract_id,
            )
                .await
        })
            .await
    }
    // async fn get_contract_function_tree_leaf_hash(&self, checkpoint_id: u64, contract_id: u32, function_id: u32) -> anyhow::Result<QHashOut<F>>;
    pub async fn get_contract_function_tree_leaf_hash(
        &self,
        checkpoint_id: u64,
        contract_id: u32,
        function_id: u32,
    ) -> anyhow::Result<QHashOut<F>> {
        with_temp_ctx_read_async::<_, _, _, C, D>(|ctx| async move {
            QEDCoordinatorStoreReaderAsync::get_contract_function_tree_leaf_hash(
                &*ctx.store_reader,
                checkpoint_id,
                contract_id,
                function_id,
            )
                .await
        })
            .await
    }
    // async fn get_contract_function_tree_leaf_hash_f(&self, checkpoint_id: F, contract_id: F, function_id: F) -> anyhow::Result<QHashOut<F>>;
    pub async fn get_contract_function_tree_leaf_hash_f(
        &self,
        checkpoint_id: F,
        contract_id: F,
        function_id: F,
    ) -> anyhow::Result<QHashOut<F>> {
        with_temp_ctx_read_async::<_, _, _, C, D>(|ctx| async move {
            QEDCoordinatorStoreReaderAsync::get_contract_function_tree_leaf_hash_f(
                &*ctx.store_reader,
                checkpoint_id,
                contract_id,
                function_id,
            )
                .await
        })
            .await
    }
    // async fn get_contract_function_tree_merkle_proof(&self, checkpoint_id: u64, contract_id: u32, function_id: u32) -> anyhow::Result<MerkleProofCore<QHashOut<F>>>;
    pub async fn get_contract_function_tree_merkle_proof(
        &self,
        checkpoint_id: u64,
        contract_id: u32,
        function_id: u32,
    ) -> anyhow::Result<MerkleProofCore<QHashOut<F>>> {
        with_temp_ctx_read_async::<_, _, _, C, D>(|ctx| async move {
            QEDCoordinatorStoreReaderAsync::get_contract_function_tree_merkle_proof(
                &*ctx.store_reader,
                checkpoint_id,
                contract_id,
                function_id,
            )
                .await
        })
            .await
    }
    // async fn get_contract_function_tree_merkle_proof_f(&self, checkpoint_id: F, contract_id: F, function_id: F) -> anyhow::Result<MerkleProofCore<QHashOut<F>>>;
    pub async fn get_contract_function_tree_merkle_proof_f(
        &self,
        checkpoint_id: F,
        contract_id: F,
        function_id: F,
    ) -> anyhow::Result<MerkleProofCore<QHashOut<F>>> {
        with_temp_ctx_read_async::<_, _, _, C, D>(|ctx| async move {
            QEDCoordinatorStoreReaderAsync::get_contract_function_tree_merkle_proof_f(
                &*ctx.store_reader,
                checkpoint_id,
                contract_id,
                function_id,
            )
                .await
        })
            .await
    }
    // async fn get_contract_tree_root(&self, checkpoint_id: u64) -> anyhow::Result<QHashOut<F>>;
    pub async fn get_contract_tree_root(&self, checkpoint_id: u64) -> anyhow::Result<QHashOut<F>> {
        with_temp_ctx_read_async::<_, _, _, C, D>(|ctx| async move {
            QEDCoordinatorStoreReaderAsync::get_contract_tree_root(
                &*ctx.store_reader,
                checkpoint_id,
            )
                .await
        })
            .await
    }
    // async fn get_contract_tree_root_f(&self, checkpoint_id: F) -> anyhow::Result<QHashOut<F>>;
    pub async fn get_contract_tree_root_f(&self, checkpoint_id: F) -> anyhow::Result<QHashOut<F>> {
        with_temp_ctx_read_async::<_, _, _, C, D>(|ctx| async move {
            QEDCoordinatorStoreReaderAsync::get_contract_tree_root_f(
                &*ctx.store_reader,
                checkpoint_id,
            )
                .await
        })
            .await
    }
    // async fn get_contract_tree_leaf_hash(&self, checkpoint_id: u64, contract_id: u32) -> anyhow::Result<QHashOut<F>>;
    pub async fn get_contract_tree_leaf_hash(
        &self,
        checkpoint_id: u64,
        contract_id: u32,
    ) -> anyhow::Result<QHashOut<F>> {
        with_temp_ctx_read_async::<_, _, _, C, D>(|ctx| async move {
            QEDCoordinatorStoreReaderAsync::get_contract_tree_leaf_hash(
                &*ctx.store_reader,
                checkpoint_id,
                contract_id,
            )
                .await
        })
            .await
    }
    // async fn get_contract_tree_leaf_hash_f(&self, checkpoint_id: F, contract_id: F) -> anyhow::Result<QHashOut<F>>;
    pub async fn get_contract_tree_leaf_hash_f(
        &self,
        checkpoint_id: F,
        contract_id: F,
    ) -> anyhow::Result<QHashOut<F>> {
        with_temp_ctx_read_async::<_, _, _, C, D>(|ctx| async move {
            QEDCoordinatorStoreReaderAsync::get_contract_tree_leaf_hash_f(
                &*ctx.store_reader,
                checkpoint_id,
                contract_id,
            )
                .await
        })
            .await
    }
    // async fn get_contract_tree_merkle_proof(&self, checkpoint_id: u64, contract_id: u32) -> anyhow::Result<MerkleProofCore<QHashOut<F>>>;
    pub async fn get_contract_tree_merkle_proof(
        &self,
        checkpoint_id: u64,
        contract_id: u32,
    ) -> anyhow::Result<MerkleProofCore<QHashOut<F>>> {
        with_temp_ctx_read_async::<_, _, _, C, D>(|ctx| async move {
            QEDCoordinatorStoreReaderAsync::get_contract_tree_merkle_proof(
                &*ctx.store_reader,
                checkpoint_id,
                contract_id,
            )
                .await
        })
            .await
    }
    // async fn get_contract_tree_merkle_proof_f(&self, checkpoint_id: F, contract_id: F) -> anyhow::Result<MerkleProofCore<QHashOut<F>>>;
    pub async fn get_contract_tree_merkle_proof_f(
        &self,
        checkpoint_id: F,
        contract_id: F,
    ) -> anyhow::Result<MerkleProofCore<QHashOut<F>>> {
        with_temp_ctx_read_async::<_, _, _, C, D>(|ctx| async move {
            QEDCoordinatorStoreReaderAsync::get_contract_tree_merkle_proof_f(
                &*ctx.store_reader,
                checkpoint_id,
                contract_id,
            )
                .await
        })
            .await
    }
    // async fn get_deposit_tree_root(&self, checkpoint_id: u64) -> anyhow::Result<QHashOut<F>>;
    pub async fn get_deposit_tree_root(&self, checkpoint_id: u64) -> anyhow::Result<QHashOut<F>> {
        with_temp_ctx_read_async::<_, _, _, C, D>(|ctx| async move {
            QEDCoordinatorStoreReaderAsync::get_deposit_tree_root(&*ctx.store_reader, checkpoint_id)
                .await
        })
            .await
    }
    // async fn get_deposit_tree_root_f(&self, checkpoint_id: F) -> anyhow::Result<QHashOut<F>>;
    pub async fn get_deposit_tree_root_f(&self, checkpoint_id: F) -> anyhow::Result<QHashOut<F>> {
        with_temp_ctx_read_async::<_, _, _, C, D>(|ctx| async move {
            QEDCoordinatorStoreReaderAsync::get_deposit_tree_root_f(
                &*ctx.store_reader,
                checkpoint_id,
            )
                .await
        })
            .await
    }
    // async fn get_deposit_tree_leaf_hash(&self, checkpoint_id: u64, deposit_id: u32) -> anyhow::Result<QHashOut<F>>;
    pub async fn get_deposit_tree_leaf_hash(
        &self,
        checkpoint_id: u64,
        deposit_id: u32,
    ) -> anyhow::Result<QHashOut<F>> {
        with_temp_ctx_read_async::<_, _, _, C, D>(|ctx| async move {
            QEDCoordinatorStoreReaderAsync::get_deposit_tree_leaf_hash(
                &*ctx.store_reader,
                checkpoint_id,
                deposit_id,
            )
                .await
        })
            .await
    }
    // async fn get_deposit_tree_leaf_hash_f(&self, checkpoint_id: F, deposit_id: F) -> anyhow::Result<QHashOut<F>>;
    pub async fn get_deposit_tree_leaf_hash_f(
        &self,
        checkpoint_id: F,
        deposit_id: F,
    ) -> anyhow::Result<QHashOut<F>> {
        with_temp_ctx_read_async::<_, _, _, C, D>(|ctx| async move {
            QEDCoordinatorStoreReaderAsync::get_deposit_tree_leaf_hash_f(
                &*ctx.store_reader,
                checkpoint_id,
                deposit_id,
            )
                .await
        })
            .await
    }
    // async fn get_deposit_tree_merkle_proof(&self, checkpoint_id: u64, deposit_id: u32) -> anyhow::Result<MerkleProofCore<QHashOut<F>>>;
    pub async fn get_deposit_tree_merkle_proof(
        &self,
        checkpoint_id: u64,
        deposit_id: u32,
    ) -> anyhow::Result<MerkleProofCore<QHashOut<F>>> {
        with_temp_ctx_read_async::<_, _, _, C, D>(|ctx| async move {
            QEDCoordinatorStoreReaderAsync::get_deposit_tree_merkle_proof(
                &*ctx.store_reader,
                checkpoint_id,
                deposit_id,
            )
                .await
        })
            .await
    }
    // async fn get_deposit_tree_merkle_proof_f(&self, checkpoint_id: F, deposit_id: F) -> anyhow::Result<MerkleProofCore<QHashOut<F>>>;
    pub async fn get_deposit_tree_merkle_proof_f(
        &self,
        checkpoint_id: F,
        deposit_id: F,
    ) -> anyhow::Result<MerkleProofCore<QHashOut<F>>> {
        with_temp_ctx_read_async::<_, _, _, C, D>(|ctx| async move {
            QEDCoordinatorStoreReaderAsync::get_deposit_tree_merkle_proof_f(
                &*ctx.store_reader,
                checkpoint_id,
                deposit_id,
            )
                .await
        })
            .await
    }
    // async fn get_withdrawal_tree_root(&self, checkpoint_id: u64) -> anyhow::Result<QHashOut<F>>;
    pub async fn get_withdrawal_tree_root(
        &self,
        checkpoint_id: u64,
    ) -> anyhow::Result<QHashOut<F>> {
        with_temp_ctx_read_async::<_, _, _, C, D>(|ctx| async move {
            QEDCoordinatorStoreReaderAsync::get_withdrawal_tree_root(
                &*ctx.store_reader,
                checkpoint_id,
            )
                .await
        })
            .await
    }
    // async fn get_withdrawal_tree_root_f(&self, checkpoint_id: F) -> anyhow::Result<QHashOut<F>>;
    pub async fn get_withdrawal_tree_root_f(
        &self,
        checkpoint_id: F,
    ) -> anyhow::Result<QHashOut<F>> {
        with_temp_ctx_read_async::<_, _, _, C, D>(|ctx| async move {
            QEDCoordinatorStoreReaderAsync::get_withdrawal_tree_root_f(
                &*ctx.store_reader,
                checkpoint_id,
            )
                .await
        })
            .await
    }
    // async fn get_withdrawal_tree_leaf_hash(&self, checkpoint_id: u64, withdrawal_id: u32) -> anyhow::Result<QHashOut<F>>;
    pub async fn get_withdrawal_tree_leaf_hash(
        &self,
        checkpoint_id: u64,
        withdrawal_id: u32,
    ) -> anyhow::Result<QHashOut<F>> {
        with_temp_ctx_read_async::<_, _, _, C, D>(|ctx| async move {
            QEDCoordinatorStoreReaderAsync::get_withdrawal_tree_leaf_hash(
                &*ctx.store_reader,
                checkpoint_id,
                withdrawal_id,
            )
                .await
        })
            .await
    }
    // async fn get_withdrawal_tree_leaf_hash_f(&self, checkpoint_id: F, withdrawal_id: F) -> anyhow::Result<QHashOut<F>>;
    pub async fn get_withdrawal_tree_leaf_hash_f(
        &self,
        checkpoint_id: F,
        withdrawal_id: F,
    ) -> anyhow::Result<QHashOut<F>> {
        with_temp_ctx_read_async::<_, _, _, C, D>(|ctx| async move {
            QEDCoordinatorStoreReaderAsync::get_withdrawal_tree_leaf_hash_f(
                &*ctx.store_reader,
                checkpoint_id,
                withdrawal_id,
            )
                .await
        })
            .await
    }
    // async fn get_withdrawal_tree_merkle_proof(&self, checkpoint_id: u64, withdrawal_id: u32) -> anyhow::Result<MerkleProofCore<QHashOut<F>>>;
    pub async fn get_withdrawal_tree_merkle_proof(
        &self,
        checkpoint_id: u64,
        withdrawal_id: u32,
    ) -> anyhow::Result<MerkleProofCore<QHashOut<F>>> {
        with_temp_ctx_read_async::<_, _, _, C, D>(|ctx| async move {
            QEDCoordinatorStoreReaderAsync::get_withdrawal_tree_merkle_proof(
                &*ctx.store_reader,
                checkpoint_id,
                withdrawal_id,
            )
                .await
        })
            .await
    }
    // async fn get_withdrawal_tree_merkle_proof_f(&self, checkpoint_id: F, withdrawal_id: F) -> anyhow::Result<MerkleProofCore<QHashOut<F>>>;
    pub async fn get_withdrawal_tree_merkle_proof_f(
        &self,
        checkpoint_id: F,
        withdrawal_id: F,
    ) -> anyhow::Result<MerkleProofCore<QHashOut<F>>> {
        with_temp_ctx_read_async::<_, _, _, C, D>(|ctx| async move {
            QEDCoordinatorStoreReaderAsync::get_withdrawal_tree_merkle_proof_f(
                &*ctx.store_reader,
                checkpoint_id,
                withdrawal_id,
            )
                .await
        })
            .await
    }
    // async fn get_latest_checkpoint_tree_root(&self) -> anyhow::Result<QHashOut<F>>;
    pub async fn get_latest_checkpoint_tree_root(&self) -> anyhow::Result<QHashOut<F>> {
        with_temp_ctx_read_async::<_, _, _, C, D>(|ctx| async move {
            QEDCoordinatorStoreReaderAsync::get_latest_checkpoint_tree_root(&*ctx.store_reader)
                .await
        })
            .await
    }
    // async fn get_checkpoint_tree_root(&self, checkpoint_id: u64) -> anyhow::Result<QHashOut<F>>;
    pub async fn get_checkpoint_tree_root(
        &self,
        checkpoint_id: u64,
    ) -> anyhow::Result<QHashOut<F>> {
        with_temp_ctx_read_async::<_, _, _, C, D>(|ctx| async move {
            QEDCoordinatorStoreReaderAsync::get_checkpoint_tree_root(
                &*ctx.store_reader,
                checkpoint_id,
            )
                .await
        })
            .await
    }
    // async fn get_checkpoint_tree_root_f(&self, checkpoint_id: F) -> anyhow::Result<QHashOut<F>>;
    pub async fn get_checkpoint_tree_root_f(
        &self,
        checkpoint_id: F,
    ) -> anyhow::Result<QHashOut<F>> {
        with_temp_ctx_read_async::<_, _, _, C, D>(|ctx| async move {
            QEDCoordinatorStoreReaderAsync::get_checkpoint_tree_root_f(
                &*ctx.store_reader,
                checkpoint_id,
            )
                .await
        })
            .await
    }
    // async fn get_checkpoint_tree_leaf_hash(&self, checkpoint_id: u64, leaf_checkpoint_id: u64) -> anyhow::Result<QHashOut<F>>;
    pub async fn get_checkpoint_tree_leaf_hash(
        &self,
        checkpoint_id: u64,
        leaf_checkpoint_id: u64,
    ) -> anyhow::Result<QHashOut<F>> {
        with_temp_ctx_read_async::<_, _, _, C, D>(|ctx| async move {
            QEDCoordinatorStoreReaderAsync::get_checkpoint_tree_leaf_hash(
                &*ctx.store_reader,
                checkpoint_id,
                leaf_checkpoint_id,
            )
                .await
        })
            .await
    }
    // async fn get_checkpoint_tree_leaf_hash_f(&self, checkpoint_id: F, leaf_checkpoint_id: F) -> anyhow::Result<QHashOut<F>>;
    pub async fn get_checkpoint_tree_leaf_hash_f(
        &self,
        checkpoint_id: F,
        leaf_checkpoint_id: F,
    ) -> anyhow::Result<QHashOut<F>> {
        with_temp_ctx_read_async::<_, _, _, C, D>(|ctx| async move {
            QEDCoordinatorStoreReaderAsync::get_checkpoint_tree_leaf_hash_f(
                &*ctx.store_reader,
                checkpoint_id,
                leaf_checkpoint_id,
            )
                .await
        })
            .await
    }
    // async fn get_checkpoint_tree_merkle_proof(&self, checkpoint_id: u64, leaf_checkpoint_id: u64) -> anyhow::Result<MerkleProofCore<QHashOut<F>>>;
    pub async fn get_checkpoint_tree_merkle_proof(
        &self,
        checkpoint_id: u64,
        leaf_checkpoint_id: u64,
    ) -> anyhow::Result<MerkleProofCore<QHashOut<F>>> {
        with_temp_ctx_read_async::<_, _, _, C, D>(|ctx| async move {
            QEDCoordinatorStoreReaderAsync::get_checkpoint_tree_merkle_proof(
                &*ctx.store_reader,
                checkpoint_id,
                leaf_checkpoint_id,
            )
                .await
        })
            .await
    }
    // async fn get_checkpoint_tree_merkle_proof_f(&self, checkpoint_id: F, leaf_checkpoint_id: F) -> anyhow::Result<MerkleProofCore<QHashOut<F>>>;
    pub async fn get_checkpoint_tree_merkle_proof_f(
        &self,
        checkpoint_id: F,
        leaf_checkpoint_id: F,
    ) -> anyhow::Result<MerkleProofCore<QHashOut<F>>> {
        with_temp_ctx_read_async::<_, _, _, C, D>(|ctx| async move {
            QEDCoordinatorStoreReaderAsync::get_checkpoint_tree_merkle_proof_f(
                &*ctx.store_reader,
                checkpoint_id,
                leaf_checkpoint_id,
            )
                .await
        })
            .await
    }
    // async fn get_checkpoint_global_state_roots(&self, checkpoint_id: u64) -> anyhow::Result<QEDCheckpointGlobalStateRoots<F>>;
    pub async fn get_checkpoint_global_state_roots(
        &self,
        checkpoint_id: u64,
    ) -> anyhow::Result<QEDCheckpointGlobalStateRoots<QEDFelt>> {
        with_temp_ctx_read_async::<_, _, _, C, D>(|ctx| async move {
            QEDCoordinatorStoreReaderAsync::get_checkpoint_global_state_roots(
                &*ctx.store_reader,
                checkpoint_id,
            )
                .await
        })
            .await
    }
    // async fn get_checkpoint_sync_info_compact(&self, checkpoint_id: u64) -> anyhow::Result<QEDCheckpointSyncInfoCompact<F>>;
    pub async fn get_checkpoint_sync_info_compact(
        &self,
        checkpoint_id: u64,
    ) -> anyhow::Result<QEDCheckpointSyncInfoCompact<QEDFelt>> {
        with_temp_ctx_read_async::<_, _, _, C, D>(|ctx| async move {
            QEDCoordinatorStoreReaderAsync::get_checkpoint_sync_info_compact(
                &*ctx.store_reader,
                checkpoint_id,
            )
                .await
        })
            .await
    }

    pub async fn get_user_leaf_data(
        &self,
        checkpoint_id: u64,
        user_id: u64,
    ) -> anyhow::Result<QEDUserLeaf<QEDFelt>> {
        with_temp_ctx_read_async::<_, _, _, C, D>(|ctx| async move {
            ctx.store_reader.get_user_leaf_data(checkpoint_id, user_id)
        })
            .await
    }
    pub async fn get_user_tree_merkle_proof(
        &self,
        checkpoint_id: u64,
        user_id: u64,
    ) -> anyhow::Result<MerkleProofCore<QHashOut<QEDFelt>>> {
        with_temp_ctx_read_async::<_, _, _, C, D>(|ctx| async move {
            ctx.store_reader
                .get_user_tree_merkle_proof(checkpoint_id, user_id)
        })
            .await
    }

    pub async fn get_user_tree_merkle_proof_f(
        &self,
        checkpoint_id: F,
        user_id: F,
    ) -> anyhow::Result<MerkleProofCore<QHashOut<QEDFelt>>> {
        with_temp_ctx_read_async::<_, _, _, C, D>(|ctx| async move {
            ctx.store_reader
                .get_user_tree_merkle_proof_f(checkpoint_id, user_id)
        })
            .await
    }
}

pub async fn get_latest_status_from_global_queue() -> anyhow::Result<Option<GlobalCoordinatorStatus>>
{
    let state = GLOBAL_COORD_EDGE_STATE
        .get()
        .ok_or_else(|| anyhow::anyhow!("GLOBAL_COORD_EDGE_STATE is not initialized"))?;

    // note: we use the fixed checkpoint_id 0 to get the latest status
    let checkpoint_id = 0;
    let entries = state
        .sync_queue
        .cdq_get_imm_sync::<GlobalCoordinatorStatus>(COORD_STATUS_CHANNEL_ID, checkpoint_id)?;

    Ok(entries.into_iter().next())
}

pub async fn get_latest_checkpoint_from_db<C, const D: usize>() -> anyhow::Result<u64> {
    with_temp_ctx_read_async::<_, _, _, C, D>(|ctx| async move {
        let state =
            QEDCoordinatorStoreReaderAsync::get_latest_l2_block_state(&*ctx.store_reader).await?;
        Ok(state.checkpoint_id)
    })
        .await
}

pub async fn recover_latest_checkpoint_from_db_if_needed() -> anyhow::Result<()> {
    let latest_checkpoint = match get_latest_checkpoint_from_db::<C, D>().await {
        Ok(latest_checkpoint) => latest_checkpoint,
        Err(e) => {
            error!("❌ Failed to get latest checkpoint from DB: {:?}", e);
            return Ok(());
        }
    };

    let current_cached = LATEST_CHECKPOINT_ID.load(Ordering::Relaxed);

    if latest_checkpoint > current_cached {
        info!(
            "🔔 Detected newer checkpoint in DB, updating from {} -> {}",
            current_cached, latest_checkpoint
        );
        LATEST_CHECKPOINT_ID.store(latest_checkpoint, Ordering::Relaxed);
    } else {
        debug!(
            "ℹ️ No newer checkpoint in DB, current = {}, db = {}",
            current_cached, latest_checkpoint
        );
    }

    Ok(())
}
