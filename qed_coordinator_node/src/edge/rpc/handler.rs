use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::Duration;
use anyhow::bail;
use plonky2::field::types::Field;
use plonky2::hash::hash_types::{HashOut};
use plonky2::plonk::config::PoseidonGoldilocksConfig;
use plonky2::plonk::proof::ProofWithPublicInputs;
use rand::RngCore;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tracing::{ error, info, warn};
use qed_core::config::network_constants::QED_CHECKPOINT_JOB_ID_CHANNEL;
use qed_core::data::qhashout::QHashOut;
use qed_core::job::drain_queue::{CheckpointDrainQueueConsumerAsyncImm, CheckpointDrainQueueEmitterAsyncImm, WithDrainQueueMetadata};
use qed_core::job::id::ProvingJobDataId;
use qed_core::job::traits::QProofStoreWriterAsyncImm;
use qed_core::job::worker_queue::{ProvingDispatcher, ProvingWorkerListener};
use qed_crypto::hash::merkle::core::MerkleProofCore;
use qed_crypto::signature::zk::data::ZKPublicKeyInfo;
use qed_data::guta::api::SubmitGUTARealmResultAPINoProofInput;
use qed_data::qblock::cmds::deploy_contract::QBCDeployContract;
use qed_data::qdata::checkpoint::{QEDCheckpointGlobalStateRoots, QEDCheckpointLeaf, QEDL2BlockState};
use qed_data::qdata::contract::{ContractCodeDefinition, QEDContractLeaf};
use qed_data::qdata::user::QEDUserLeaf;
use qed_data::qsync::coordinator::QEDCheckpointSyncInfoCompact;
use qed_node::nimpl::worker_queue_redis::redis_queue::{CEQueueNotification, CPQueueNotification, RedisQueue, CE_NOTIFICATIONS, CP_NOTIFICATIONS};
use qed_store::config::store_config::{QEDFelt, QEDHasher};
use qed_store::node::coordinator::store_traits::QEDCoordinatorStoreReaderAsync;
use qed_store::store::node::realm::writer_imm::get_user_id_from_registration_id;
use qed_store::traits::qdatastore::qmetadata::QMetaDataStoreReaderSync;
use crate::context::with_temp_ctx_read_async;
use crate::edge::context::{with_ctx_read_async, GLOBAL_COORD_EDGE_CTX, LATEST_CHECKPOINT_ID, REGISTERED_USERS, REGISTER_USER_COUNTER};
use crate::edge::processor::{handle_cp_sync, process_realm_job};
use crate::edge::rpc::types::GetUserIdRequest;

type F = QEDFelt;
type C = PoseidonGoldilocksConfig;
const D: usize = 2;

#[derive(Clone)]
pub struct CoordinatorEdgeHandler {
    notify_queue: RedisQueue,
    cp_listener: Arc<Mutex<Option<JoinHandle<()>>>>,
    realm_job_listener: Arc<Mutex<Option<JoinHandle<()>>>>,
}

impl CoordinatorEdgeHandler {
    pub fn new(redis_uri: &str) -> anyhow::Result<Self> {
        Ok(Self {
            notify_queue: RedisQueue::new(redis_uri)?,
            cp_listener: Arc::new(Mutex::new(None)),
            realm_job_listener: Arc::new(Mutex::new(None)),
        })
    }
    ///receive StartSync notification from CP
    pub async fn spawn_cp_sync_listener(&self) -> anyhow::Result<()> {
        info!("cp sync listener spawned");
        // note: run this only once
        if self.cp_listener.lock().await.is_some() {
            return Ok(());
        }

        let mut notify_q = self.notify_queue.clone();
        let ctx_lock = GLOBAL_COORD_EDGE_CTX.clone();

        let handle = tokio::spawn(async move {
            loop {
                match notify_q.pop_one(CP_NOTIFICATIONS) {
                    Ok(Some(bytes)) => {
                        if let Ok(CPQueueNotification::StartSync) =
                            serde_json::from_slice::<CPQueueNotification>(&bytes)
                        {
                            info!("🔔 CP listener received StartSync notification");
                            let checkpoint_id =
                                LATEST_CHECKPOINT_ID.fetch_add(1, Ordering::Relaxed) + 1;

                            info!("ℹ️ latest checkpoint now = {checkpoint_id}");


                            if let Ok(guard) = ctx_lock.try_read() {
                                if let Some(ctx) = guard.as_ref() {
                                    if let Err(e) = handle_cp_sync(ctx).await {
                                        error!("❌ handle_cp_sync: {e}");
                                    }
                                }
                            }
                        }
                    }

                    // Queue is empty, return Ok(None)
                    Ok(None) => { /* nothing */ }

                    // If the queue doesn't exist, create it
                    Err(e) if e.to_string().contains("Queue not found") => {
                        info!("ℹ️ CP_NOTIFICATIONS not found, creating …");
                        if let Err(init_err) = notify_q.ensure_queue(CP_NOTIFICATIONS) {
                            warn!("failed to create CP_NOTIFICATIONS: {init_err:?}");
                        }

                    }
                    // Other errors
                    Err(e) => warn!("CP listener pop error: {e:?}"),
                }
                //todo! maybe longer or shorter
                tokio::time::sleep(Duration::from_millis(600)).await;
            }
        });

        *self.cp_listener.lock().await = Some(handle);
        Ok(())
    }

    /// Listen to RP output proving_data_job_id (GUTA result)
    pub async fn spawn_realm_job_listener(&self) -> anyhow::Result<()> {
        info!("realm job listener spawned");
        //run this only once
        if self.realm_job_listener.lock().await.is_some() {
            return Ok(());
        }

        let channel_id = QED_CHECKPOINT_JOB_ID_CHANNEL;

        let ctx_lock = GLOBAL_COORD_EDGE_CTX.clone();

        let handle = tokio::spawn(async move {
            loop {
                // get ctx
                if let Ok(guard) = ctx_lock.try_read() {
                    if let Some(ctx) = guard.as_ref() {
                        let next_checkpoint = LATEST_CHECKPOINT_ID.load(Ordering::Relaxed) + 1;
                        // debug!("waiting for realm job, channel_id = {}, next_checkpoint = {}", channel_id, next_checkpoint);

                        match ctx
                            .proof_store
                            .cdq_drain_imm::<ProvingJobDataId>(channel_id, next_checkpoint)
                            .await
                        {
                            Ok(jobs) => {
                                if jobs.is_empty() {
                                    // debug!("🟡 No jobs at checkpoint {}, retrying later", next_checkpoint);
                                } else {
                                    for job_id in jobs {
                                        info!("🎯 Got ProvingJobDataId: {:?}", job_id);

                                        if let Err(e) = process_realm_job(ctx, job_id).await {
                                            error!("❌ process_realm_job error: {:?}", e);
                                        } else {
                                            info!("✅ process_realm_job success, checkpoint = {}", job_id.checkpoint_id);
                                        }
                                    }
                                }

                            }
                            Err(e) => {
                                warn!("⚠️ cdq_drain_imm error: {:?}", e);
                            }
                        }
                    }
                }
                tokio::time::sleep(Duration::from_millis(500)).await;
            }
        });

        // let mut lock = self.realm_job_listener.lock().await;
        // *lock = Some(handle);
        *self.realm_job_listener.lock().await = Some(handle);
        Ok(())
    }

    pub async fn register_user(&self, zk_user_info: ZKPublicKeyInfo<QEDFelt>) -> anyhow::Result<()> {

        let public_key = zk_user_info.public_key_param;

        if let Some(user_id) = REGISTERED_USERS.get(&public_key) {
            info!("🛑 user already registered, user_id = {}", *user_id);
            return Ok(());
        }

        let register_id = REGISTER_USER_COUNTER.fetch_add(1, Ordering::Relaxed);
        let user_id = get_user_id_from_registration_id(register_id);
        REGISTERED_USERS.insert(public_key, user_id);

        with_ctx_read_async(|ctx| {
            let queue = ctx.checkpoint_queue.clone();
            let zk_user = zk_user_info.clone(); // avoid the lifetime issue

            async move {
                info!(
                    "🚀 pushing  new user to drain queue, user_id = {}, pub_key = {},",
                    user_id, zk_user.public_key_param
                );
                queue.cdq_push_imm(zk_user).await?;
                info!("✅ pushed to drain queue.");
                Ok(())
            }
        })
            .await
    }

    pub async fn get_user_id_by_pub_key(&self, params: GetUserIdRequest) -> anyhow::Result<Option<u64>> {
        // 1. Decode hex string to QHashOut<QEDFelt>
        let bytes = hex::decode(&params.public_key_param)
            .map_err(|e| anyhow::anyhow!("Invalid hex string: {}", e))?;

        if bytes.len() != 32 {
            bail!("Invalid public_key_param length (expected 32 bytes)");
        }

        let mut elements = [0u64; 4];
        for i in 0..4 {
            elements[i] = u64::from_le_bytes(bytes[i * 8..(i + 1) * 8].try_into()?);
        }

        let qhash = qhash_from_u64_array(elements);

        // 2. Query dashmap
        if let Some(user_id) = REGISTERED_USERS.get(&qhash) {
            Ok(Some(*user_id))
        } else {
            Ok(None)
        }
    }
    pub async fn deploy_contract(
        &self,
        contract: QBCDeployContract<QEDFelt>,
    ) -> anyhow::Result<()> {
        let next_checkpoint_id = LATEST_CHECKPOINT_ID.load(Ordering::Relaxed) + 2;
        with_ctx_read_async(|ctx| {
            let queue = ctx.checkpoint_queue.clone();
            let config = ctx.coordinator_config.clone();

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

        let (store_reader, checkpoint_queue, proof_store, config, verifier) =
            with_ctx_read_async(|ctx| {

                let store_reader = ctx.store_reader.clone();
                let checkpoint_queue = ctx.checkpoint_queue.clone();
                let proof_store = ctx.proof_store.clone();
                let config = ctx.coordinator_config.clone();
                let verifier = ctx.proof_verifier.clone();

                std::future::ready(Ok((
                    store_reader,
                    checkpoint_queue,
                    proof_store,
                    config,
                    verifier,
                )))
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

        // verify state consistency
        let old_root = store_reader
            .get_user_latest_top_tree_cap_root(config.realm_root_level, input.realm_id)
            .await?;

        if old_root != input.top_line_proof.old_root && old_root != input.top_line_proof.new_root {
            anyhow::bail!("invalid top line proof old value from realm");
        }

        // build queue item
        let queue_item =
            input.to_queue_item(config.guta_channel_id, config.realm_root_level as u32);
        let proof_id = queue_item.proof_id;

        // write to proof store
        proof_store.set_proof_by_id(proof_id, &proof).await?;
        checkpoint_queue.cdq_push_imm(queue_item).await?;

        Ok(())
    }

    pub async fn build_block(&self) -> anyhow::Result<()> {
        info!("🚀 build_block called");
        self.notify_queue
            .clone()
            .dispatch(CE_NOTIFICATIONS, CEQueueNotification::StartProduceBlock)?;
        info!("✅ build_block dispatched to queue");
        Ok(())
    }

    // async fn get_contract_leaf_data(&self, contract_id: u64) -> anyhow::Result<QEDContractLeaf<F>>;
    pub async fn get_contract_leaf_data(&self, contract_id: u64) -> anyhow::Result<QEDContractLeaf<QEDFelt>> {
        with_temp_ctx_read_async::<_,_,_,C,D>(|ctx| async move {
            QEDCoordinatorStoreReaderAsync::get_contract_leaf_data(&*ctx.store_reader, contract_id).await
        }).await
    }

    // async fn get_contract_leaf_data_f(&self, contract_id: F) -> anyhow::Result<QEDContractLeaf<F>>;
    pub async fn get_contract_leaf_data_f(&self, contract_id: F) -> anyhow::Result<QEDContractLeaf<F>> {
        with_temp_ctx_read_async::<_,_,_,C,D>(|ctx| async move {
            QEDCoordinatorStoreReaderAsync::get_contract_leaf_data_f(&*ctx.store_reader, contract_id).await
        }).await
    }
    // async fn get_checkpoint_leaf_data(&self, checkpoint_id: u64) -> anyhow::Result<QEDCheckpointLeaf<F>>;
    pub async fn get_checkpoint_leaf_data(&self, checkpoint_id: u64) -> anyhow::Result<QEDCheckpointLeaf<QEDFelt>> {
        with_temp_ctx_read_async::<_,_,_,C,D>(|ctx| async move {
            QEDCoordinatorStoreReaderAsync::get_checkpoint_leaf_data(&*ctx.store_reader, checkpoint_id).await
        }).await
    }
    // async fn get_checkpoint_leaf_data_f(&self, checkpoint_id: F) -> anyhow::Result<QEDCheckpointLeaf<F>>;
    pub async fn get_checkpoint_leaf_data_f(&self, checkpoint_id: F) -> anyhow::Result<QEDCheckpointLeaf<F>> {
        with_temp_ctx_read_async::<_,_,_,C,D>(|ctx| async move {
            QEDCoordinatorStoreReaderAsync::get_checkpoint_leaf_data_f(&*ctx.store_reader, checkpoint_id).await
        }).await
    }

    // async fn get_contract_code_definition(&self, contract_id: u64) -> anyhow::Result<ContractCodeDefinition>;
    pub async fn get_contract_code_definition(&self, contract_id: u64) -> anyhow::Result<ContractCodeDefinition> {
        with_temp_ctx_read_async::<_,_,_,C,D>(|ctx| async move {
            QEDCoordinatorStoreReaderAsync::get_contract_code_definition(&*ctx.store_reader, contract_id).await
        }).await
    }

    // async fn get_contract_code_definition_f(&self, contract_id: F) -> anyhow::Result<ContractCodeDefinition>;
    pub async fn get_contract_code_definition_f(&self, contract_id: F) -> anyhow::Result<ContractCodeDefinition> {
        with_temp_ctx_read_async::<_,_,_,C,D>(|ctx| async move {
            QEDCoordinatorStoreReaderAsync::get_contract_code_definition_f(&*ctx.store_reader, contract_id).await
        }).await
    }

    // async fn get_latest_l2_block_state(&self) -> anyhow::Result<QEDL2BlockState>;
    pub async fn get_latest_l2_block_state(&self) -> anyhow::Result<QEDL2BlockState> {

        with_temp_ctx_read_async::<_,_,_,C,D>(|ctx| async move {
            QEDCoordinatorStoreReaderAsync::get_latest_l2_block_state(&*ctx.store_reader).await
        }).await
    }

    // async fn get_l2_block_state(&self, checkpoint_id: u64) -> anyhow::Result<QEDL2BlockState>;

    pub async fn get_l2_block_state(&self, checkpoint_id: u64) -> anyhow::Result<QEDL2BlockState> {
        with_temp_ctx_read_async::<_,_,_,C,D>(|ctx| async move {
            QEDCoordinatorStoreReaderAsync::get_l2_block_state(&*ctx.store_reader, checkpoint_id).await
        }).await
    }


    // async fn get_l2_block_state_f(&self, checkpoint_id: F) -> anyhow::Result<QEDL2BlockState>;
    pub async fn get_l2_block_state_f(&self, checkpoint_id: F) -> anyhow::Result<QEDL2BlockState> {
        with_temp_ctx_read_async::<_,_,_,C,D>(|ctx| async move {
            QEDCoordinatorStoreReaderAsync::get_l2_block_state_f(&*ctx.store_reader, checkpoint_id).await
        }).await
    }

    // async fn get_user_registration_tree_root(&self, checkpoint_id: u64) -> anyhow::Result<QHashOut<F>>;
    pub async fn get_user_registration_tree_root(&self, checkpoint_id: u64) -> anyhow::Result<QHashOut<F>> {
        with_temp_ctx_read_async::<_,_,_,C,D>(|ctx| async move {
            QEDCoordinatorStoreReaderAsync::get_user_registration_tree_root(&*ctx.store_reader, checkpoint_id).await
        }).await
    }
    // async fn get_user_registration_tree_root_f(&self, checkpoint_id: F) -> anyhow::Result<QHashOut<F>>;
    pub async fn get_user_registration_tree_root_f(&self, checkpoint_id: F) -> anyhow::Result<QHashOut<F>> {
        with_temp_ctx_read_async::<_,_,_,C,D>(|ctx| async move {
            QEDCoordinatorStoreReaderAsync::get_user_registration_tree_root_f(&*ctx.store_reader, checkpoint_id).await
        }).await
    }
    // async fn get_user_registration_tree_leaf_hash(&self, checkpoint_id: u64, leaf_index: u64) -> anyhow::Result<QHashOut<F>>;
    pub async fn get_user_registration_tree_leaf_hash(&self, checkpoint_id: u64, leaf_index: u64) -> anyhow::Result<QHashOut<F>> {
        with_temp_ctx_read_async::<_,_,_,C,D>(|ctx| async move {
            QEDCoordinatorStoreReaderAsync::get_user_registration_tree_leaf_hash(&*ctx.store_reader, checkpoint_id, leaf_index).await
        }).await
    }
    // async fn get_user_registration_tree_leaf_hash_f(&self, checkpoint_id: F, leaf_index: F) -> anyhow::Result<QHashOut<F>>;
    pub async fn get_user_registration_tree_leaf_hash_f(&self, checkpoint_id: F, leaf_index: F) -> anyhow::Result<QHashOut<F>> {
        with_temp_ctx_read_async::<_,_,_,C,D>(|ctx| async move {
            QEDCoordinatorStoreReaderAsync::get_user_registration_tree_leaf_hash_f(&*ctx.store_reader, checkpoint_id, leaf_index).await
        }).await
    }

    // async fn get_user_registration_tree_merkle_proof(&self, checkpoint_id: u64, leaf_index: u64) -> anyhow::Result<MerkleProofCore<QHashOut<F>>>;
    pub async fn get_user_registration_tree_merkle_proof(&self, checkpoint_id: u64, leaf_index: u64) -> anyhow::Result<MerkleProofCore<QHashOut<F>>> {
        with_temp_ctx_read_async::<_,_,_,C,D>(|ctx| async move {
            QEDCoordinatorStoreReaderAsync::get_user_registration_tree_merkle_proof(&*ctx.store_reader, checkpoint_id, leaf_index).await
        }).await
    }
    // async fn get_user_registration_tree_merkle_proof_f(&self, checkpoint_id: F, leaf_index: F) -> anyhow::Result<MerkleProofCore<QHashOut<F>>>;
    pub async fn get_user_registration_tree_merkle_proof_f(&self, checkpoint_id: F, leaf_index: F) -> anyhow::Result<MerkleProofCore<QHashOut<F>>> {
        with_temp_ctx_read_async::<_,_,_,C,D>(|ctx| async move {
            QEDCoordinatorStoreReaderAsync::get_user_registration_tree_merkle_proof_f(&*ctx.store_reader, checkpoint_id, leaf_index).await
        }).await
    }
    //

    // async fn get_user_tree_root(&self, checkpoint_id: u64) -> anyhow::Result<QHashOut<F>>;
    pub async fn get_user_tree_root(&self, checkpoint_id: u64) -> anyhow::Result<QHashOut<F>> {
        with_temp_ctx_read_async::<_,_,_,C,D>(|ctx| async move {
            QEDCoordinatorStoreReaderAsync::get_user_tree_root(&*ctx.store_reader, checkpoint_id).await
        }).await
    }
    // async fn get_user_tree_root_f(&self, checkpoint_id: F) -> anyhow::Result<QHashOut<F>>;
    pub async fn get_user_tree_root_f(&self, checkpoint_id: F) -> anyhow::Result<QHashOut<F>> {
        with_temp_ctx_read_async::<_,_,_,C,D>(|ctx| async move {
            QEDCoordinatorStoreReaderAsync::get_user_tree_root_f(&*ctx.store_reader, checkpoint_id).await
        }).await
    }
    // async fn get_user_sub_tree_merkle_proof(&self, checkpoint_id: u64, root_level: u8, leaf_level: u8, leaf_index: u64) -> anyhow::Result<MerkleProofCore<QHashOut<F>>>;
    pub async fn get_user_sub_tree_merkle_proof(&self, checkpoint_id: u64, root_level: u8, leaf_level: u8, leaf_index: u64) -> anyhow::Result<MerkleProofCore<QHashOut<F>>> {
        with_temp_ctx_read_async::<_,_,_,C,D>(|ctx| async move {
            QEDCoordinatorStoreReaderAsync::get_user_sub_tree_merkle_proof(&*ctx.store_reader, checkpoint_id, root_level, leaf_level, leaf_index).await
        }).await
    }
    // async fn get_user_top_tree_merkle_proof(&self, checkpoint_id: u64, leaf_level: u8, leaf_index: u64) -> anyhow::Result<MerkleProofCore<QHashOut<F>>>;
    pub async fn get_user_top_tree_merkle_proof(&self, checkpoint_id: u64, leaf_level: u8, leaf_index: u64) -> anyhow::Result<MerkleProofCore<QHashOut<F>>> {
        with_temp_ctx_read_async::<_,_,_,C,D>(|ctx| async move {
            QEDCoordinatorStoreReaderAsync::get_user_top_tree_merkle_proof(&*ctx.store_reader, checkpoint_id, leaf_level, leaf_index).await
        }).await
    }
    // async fn get_user_top_tree_cap_root(&self, checkpoint_id: u64, cap_level: u8, cap_index: u64) -> anyhow::Result<QHashOut<F>>;
    pub async fn get_user_top_tree_cap_root(&self, checkpoint_id: u64, cap_level: u8, cap_index: u64) -> anyhow::Result<QHashOut<F>> {
        with_temp_ctx_read_async::<_,_,_,C,D>(|ctx| async move {
            QEDCoordinatorStoreReaderAsync::get_user_top_tree_cap_root(&*ctx.store_reader, checkpoint_id, cap_level, cap_index).await
        }).await
    }
    // async fn get_user_latest_top_tree_cap_root(&self, cap_level: u8, cap_index: u64) -> anyhow::Result<QHashOut<F>>;
    pub async fn get_user_latest_top_tree_cap_root(&self, cap_level: u8, cap_index: u64) -> anyhow::Result<QHashOut<F>> {
        with_temp_ctx_read_async::<_,_,_,C,D>(|ctx| async move {
            QEDCoordinatorStoreReaderAsync::get_user_latest_top_tree_cap_root(&*ctx.store_reader, cap_level, cap_index).await
        }).await
    }
    //
    //
    // async fn get_contract_function_tree_root(&self, checkpoint_id: u64, contract_id: u32) -> anyhow::Result<QHashOut<F>>;
    pub async fn get_contract_function_tree_root(&self, checkpoint_id: u64, contract_id: u32) -> anyhow::Result<QHashOut<F>> {
        with_temp_ctx_read_async::<_,_,_,C,D>(|ctx| async move {
            QEDCoordinatorStoreReaderAsync::get_contract_function_tree_root(&*ctx.store_reader, checkpoint_id, contract_id).await
        }).await
    }

    // async fn get_contract_function_tree_root_f(&self, checkpoint_id: F, contract_id: F) -> anyhow::Result<QHashOut<F>>;
    pub async fn get_contract_function_tree_root_f(&self, checkpoint_id: F, contract_id: F) -> anyhow::Result<QHashOut<F>> {
        with_temp_ctx_read_async::<_,_,_,C,D>(|ctx| async move {
            QEDCoordinatorStoreReaderAsync::get_contract_function_tree_root_f(&*ctx.store_reader, checkpoint_id, contract_id).await
        }).await
    }
    // async fn get_contract_function_tree_leaf_hash(&self, checkpoint_id: u64, contract_id: u32, function_id: u32) -> anyhow::Result<QHashOut<F>>;
    pub async fn get_contract_function_tree_leaf_hash(&self, checkpoint_id: u64, contract_id: u32, function_id: u32) -> anyhow::Result<QHashOut<F>> {
        with_temp_ctx_read_async::<_,_,_,C,D>(|ctx| async move {
            QEDCoordinatorStoreReaderAsync::get_contract_function_tree_leaf_hash(&*ctx.store_reader, checkpoint_id, contract_id, function_id).await
        }).await
    }
    // async fn get_contract_function_tree_leaf_hash_f(&self, checkpoint_id: F, contract_id: F, function_id: F) -> anyhow::Result<QHashOut<F>>;
    pub async fn get_contract_function_tree_leaf_hash_f(&self, checkpoint_id: F, contract_id: F, function_id: F) -> anyhow::Result<QHashOut<F>> {
        with_temp_ctx_read_async::<_,_,_,C,D>(|ctx| async move {
            QEDCoordinatorStoreReaderAsync::get_contract_function_tree_leaf_hash_f(&*ctx.store_reader, checkpoint_id, contract_id, function_id).await
        }).await
    }
    // async fn get_contract_function_tree_merkle_proof(&self, checkpoint_id: u64, contract_id: u32, function_id: u32) -> anyhow::Result<MerkleProofCore<QHashOut<F>>>;
    pub async fn get_contract_function_tree_merkle_proof(&self, checkpoint_id: u64, contract_id: u32, function_id: u32) -> anyhow::Result<MerkleProofCore<QHashOut<F>>> {
        with_temp_ctx_read_async::<_,_,_,C,D>(|ctx| async move {
            QEDCoordinatorStoreReaderAsync::get_contract_function_tree_merkle_proof(&*ctx.store_reader, checkpoint_id, contract_id, function_id).await
        }).await
    }
    // async fn get_contract_function_tree_merkle_proof_f(&self, checkpoint_id: F, contract_id: F, function_id: F) -> anyhow::Result<MerkleProofCore<QHashOut<F>>>;
    pub async fn get_contract_function_tree_merkle_proof_f(&self, checkpoint_id: F, contract_id: F, function_id: F) -> anyhow::Result<MerkleProofCore<QHashOut<F>>> {
        with_temp_ctx_read_async::<_,_,_,C,D>(|ctx| async move {
            QEDCoordinatorStoreReaderAsync::get_contract_function_tree_merkle_proof_f(&*ctx.store_reader, checkpoint_id, contract_id, function_id).await
        }).await
    }
    //
    //

    // async fn get_contract_tree_root(&self, checkpoint_id: u64) -> anyhow::Result<QHashOut<F>>;
    pub async fn get_contract_tree_root(&self, checkpoint_id: u64) -> anyhow::Result<QHashOut<F>> {
        with_temp_ctx_read_async::<_,_,_,C,D>(|ctx| async move {
            QEDCoordinatorStoreReaderAsync::get_contract_tree_root(&*ctx.store_reader, checkpoint_id).await
        }).await
    }
    // async fn get_contract_tree_root_f(&self, checkpoint_id: F) -> anyhow::Result<QHashOut<F>>;
    pub async fn get_contract_tree_root_f(&self, checkpoint_id: F) -> anyhow::Result<QHashOut<F>> {
        with_temp_ctx_read_async::<_,_,_,C,D>(|ctx| async move {
            QEDCoordinatorStoreReaderAsync::get_contract_tree_root_f(&*ctx.store_reader, checkpoint_id).await
        }).await
    }
    // async fn get_contract_tree_leaf_hash(&self, checkpoint_id: u64, contract_id: u32) -> anyhow::Result<QHashOut<F>>;
    pub async fn get_contract_tree_leaf_hash(&self, checkpoint_id: u64, contract_id: u32) -> anyhow::Result<QHashOut<F>> {
        with_temp_ctx_read_async::<_,_,_,C,D>(|ctx| async move {
            QEDCoordinatorStoreReaderAsync::get_contract_tree_leaf_hash(&*ctx.store_reader, checkpoint_id, contract_id).await
        }).await
    }
    // async fn get_contract_tree_leaf_hash_f(&self, checkpoint_id: F, contract_id: F) -> anyhow::Result<QHashOut<F>>;
    pub async fn get_contract_tree_leaf_hash_f(&self, checkpoint_id: F, contract_id: F) -> anyhow::Result<QHashOut<F>> {
        with_temp_ctx_read_async::<_,_,_,C,D>(|ctx| async move {
            QEDCoordinatorStoreReaderAsync::get_contract_tree_leaf_hash_f(&*ctx.store_reader, checkpoint_id, contract_id).await
        }).await
    }
    // async fn get_contract_tree_merkle_proof(&self, checkpoint_id: u64, contract_id: u32) -> anyhow::Result<MerkleProofCore<QHashOut<F>>>;
    pub async fn get_contract_tree_merkle_proof(&self, checkpoint_id: u64, contract_id: u32) -> anyhow::Result<MerkleProofCore<QHashOut<F>>> {
        with_temp_ctx_read_async::<_,_,_,C,D>(|ctx| async move {
            QEDCoordinatorStoreReaderAsync::get_contract_tree_merkle_proof(&*ctx.store_reader, checkpoint_id, contract_id).await
        }).await
    }
    // async fn get_contract_tree_merkle_proof_f(&self, checkpoint_id: F, contract_id: F) -> anyhow::Result<MerkleProofCore<QHashOut<F>>>;
    pub async fn get_contract_tree_merkle_proof_f(&self, checkpoint_id: F, contract_id: F) -> anyhow::Result<MerkleProofCore<QHashOut<F>>> {
        with_temp_ctx_read_async::<_,_,_,C,D>(|ctx| async move {
            QEDCoordinatorStoreReaderAsync::get_contract_tree_merkle_proof_f(&*ctx.store_reader, checkpoint_id, contract_id).await
        }).await
    }

    // async fn get_deposit_tree_root(&self, checkpoint_id: u64) -> anyhow::Result<QHashOut<F>>;
    pub async fn get_deposit_tree_root(&self, checkpoint_id: u64) -> anyhow::Result<QHashOut<F>> {
        with_temp_ctx_read_async::<_,_,_,C,D>(|ctx| async move {
            QEDCoordinatorStoreReaderAsync::get_deposit_tree_root(&*ctx.store_reader, checkpoint_id).await
        }).await
    }

    // async fn get_deposit_tree_root_f(&self, checkpoint_id: F) -> anyhow::Result<QHashOut<F>>;
    pub async fn get_deposit_tree_root_f(&self, checkpoint_id: F) -> anyhow::Result<QHashOut<F>> {
        with_temp_ctx_read_async::<_,_,_,C,D>(|ctx| async move {
            QEDCoordinatorStoreReaderAsync::get_deposit_tree_root_f(&*ctx.store_reader, checkpoint_id).await
        }).await
    }
    // async fn get_deposit_tree_leaf_hash(&self, checkpoint_id: u64, deposit_id: u32) -> anyhow::Result<QHashOut<F>>;
    pub async fn get_deposit_tree_leaf_hash(&self, checkpoint_id: u64, deposit_id: u32) -> anyhow::Result<QHashOut<F>> {
        with_temp_ctx_read_async::<_,_,_,C,D>(|ctx| async move {
            QEDCoordinatorStoreReaderAsync::get_deposit_tree_leaf_hash(&*ctx.store_reader, checkpoint_id, deposit_id).await
        }).await
    }
    // async fn get_deposit_tree_leaf_hash_f(&self, checkpoint_id: F, deposit_id: F) -> anyhow::Result<QHashOut<F>>;
    pub async fn get_deposit_tree_leaf_hash_f(&self, checkpoint_id: F, deposit_id: F) -> anyhow::Result<QHashOut<F>> {
        with_temp_ctx_read_async::<_,_,_,C,D>(|ctx| async move {
            QEDCoordinatorStoreReaderAsync::get_deposit_tree_leaf_hash_f(&*ctx.store_reader, checkpoint_id, deposit_id).await
        }).await
    }
    // async fn get_deposit_tree_merkle_proof(&self, checkpoint_id: u64, deposit_id: u32) -> anyhow::Result<MerkleProofCore<QHashOut<F>>>;
    pub async fn get_deposit_tree_merkle_proof(&self, checkpoint_id: u64, deposit_id: u32) -> anyhow::Result<MerkleProofCore<QHashOut<F>>> {
        with_temp_ctx_read_async::<_,_,_,C,D>(|ctx| async move {
            QEDCoordinatorStoreReaderAsync::get_deposit_tree_merkle_proof(&*ctx.store_reader, checkpoint_id, deposit_id).await
        }).await
    }
    // async fn get_deposit_tree_merkle_proof_f(&self, checkpoint_id: F, deposit_id: F) -> anyhow::Result<MerkleProofCore<QHashOut<F>>>;
    pub async fn get_deposit_tree_merkle_proof_f(&self, checkpoint_id: F, deposit_id: F) -> anyhow::Result<MerkleProofCore<QHashOut<F>>> {
        with_temp_ctx_read_async::<_,_,_,C,D>(|ctx| async move {
            QEDCoordinatorStoreReaderAsync::get_deposit_tree_merkle_proof_f(&*ctx.store_reader, checkpoint_id, deposit_id).await
        }).await
    }
    //
    //
    // async fn get_withdrawal_tree_root(&self, checkpoint_id: u64) -> anyhow::Result<QHashOut<F>>;
    pub async fn get_withdrawal_tree_root(&self, checkpoint_id: u64) -> anyhow::Result<QHashOut<F>> {
        with_temp_ctx_read_async::<_,_,_,C,D>(|ctx| async move {
            QEDCoordinatorStoreReaderAsync::get_withdrawal_tree_root(&*ctx.store_reader, checkpoint_id).await
        }).await
    }

    // async fn get_withdrawal_tree_root_f(&self, checkpoint_id: F) -> anyhow::Result<QHashOut<F>>;
    pub async fn get_withdrawal_tree_root_f(&self, checkpoint_id: F) -> anyhow::Result<QHashOut<F>> {
        with_temp_ctx_read_async::<_,_,_,C,D>(|ctx| async move {
            QEDCoordinatorStoreReaderAsync::get_withdrawal_tree_root_f(&*ctx.store_reader, checkpoint_id).await
        }).await
    }
    // async fn get_withdrawal_tree_leaf_hash(&self, checkpoint_id: u64, withdrawal_id: u32) -> anyhow::Result<QHashOut<F>>;
    pub async fn get_withdrawal_tree_leaf_hash(&self, checkpoint_id: u64, withdrawal_id: u32) -> anyhow::Result<QHashOut<F>> {
        with_temp_ctx_read_async::<_,_,_,C,D>(|ctx| async move {
            QEDCoordinatorStoreReaderAsync::get_withdrawal_tree_leaf_hash(&*ctx.store_reader, checkpoint_id, withdrawal_id).await
        }).await
    }
    // async fn get_withdrawal_tree_leaf_hash_f(&self, checkpoint_id: F, withdrawal_id: F) -> anyhow::Result<QHashOut<F>>;
    pub async fn get_withdrawal_tree_leaf_hash_f(&self, checkpoint_id: F, withdrawal_id: F) -> anyhow::Result<QHashOut<F>> {
        with_temp_ctx_read_async::<_,_,_,C,D>(|ctx| async move {
            QEDCoordinatorStoreReaderAsync::get_withdrawal_tree_leaf_hash_f(&*ctx.store_reader, checkpoint_id, withdrawal_id).await
        }).await
    }
    // async fn get_withdrawal_tree_merkle_proof(&self, checkpoint_id: u64, withdrawal_id: u32) -> anyhow::Result<MerkleProofCore<QHashOut<F>>>;
    pub async fn get_withdrawal_tree_merkle_proof(&self, checkpoint_id: u64, withdrawal_id: u32) -> anyhow::Result<MerkleProofCore<QHashOut<F>>> {
        with_temp_ctx_read_async::<_,_,_,C,D>(|ctx| async move {
            QEDCoordinatorStoreReaderAsync::get_withdrawal_tree_merkle_proof(&*ctx.store_reader, checkpoint_id, withdrawal_id).await
        }).await
    }
    // async fn get_withdrawal_tree_merkle_proof_f(&self, checkpoint_id: F, withdrawal_id: F) -> anyhow::Result<MerkleProofCore<QHashOut<F>>>;
    pub async fn get_withdrawal_tree_merkle_proof_f(&self, checkpoint_id: F, withdrawal_id: F) -> anyhow::Result<MerkleProofCore<QHashOut<F>>> {
        with_temp_ctx_read_async::<_,_,_,C,D>(|ctx| async move {
            QEDCoordinatorStoreReaderAsync::get_withdrawal_tree_merkle_proof_f(&*ctx.store_reader, checkpoint_id, withdrawal_id).await
        }).await
    }
    //
    // async fn get_latest_checkpoint_tree_root(&self) -> anyhow::Result<QHashOut<F>>;
    pub async fn get_latest_checkpoint_tree_root(&self) -> anyhow::Result<QHashOut<F>> {
        with_temp_ctx_read_async::<_,_,_,C,D>(|ctx| async move {
            QEDCoordinatorStoreReaderAsync::get_latest_checkpoint_tree_root(&*ctx.store_reader).await
        }).await
    }
    // async fn get_checkpoint_tree_root(&self, checkpoint_id: u64) -> anyhow::Result<QHashOut<F>>;
    pub async fn get_checkpoint_tree_root(&self, checkpoint_id: u64) -> anyhow::Result<QHashOut<F>> {
        with_temp_ctx_read_async::<_,_,_,C,D>(|ctx| async move {
            QEDCoordinatorStoreReaderAsync::get_checkpoint_tree_root(&*ctx.store_reader, checkpoint_id).await
        }).await
    }
    // async fn get_checkpoint_tree_root_f(&self, checkpoint_id: F) -> anyhow::Result<QHashOut<F>>;
    pub async fn get_checkpoint_tree_root_f(&self, checkpoint_id: F) -> anyhow::Result<QHashOut<F>> {
        with_temp_ctx_read_async::<_,_,_,C,D>(|ctx| async move {
            QEDCoordinatorStoreReaderAsync::get_checkpoint_tree_root_f(&*ctx.store_reader, checkpoint_id).await
        }).await
    }
    // async fn get_checkpoint_tree_leaf_hash(&self, checkpoint_id: u64, leaf_checkpoint_id: u64) -> anyhow::Result<QHashOut<F>>;
    pub async fn get_checkpoint_tree_leaf_hash(&self, checkpoint_id: u64, leaf_checkpoint_id: u64) -> anyhow::Result<QHashOut<F>> {
        with_temp_ctx_read_async::<_,_,_,C,D>(|ctx| async move {
            QEDCoordinatorStoreReaderAsync::get_checkpoint_tree_leaf_hash(&*ctx.store_reader, checkpoint_id, leaf_checkpoint_id).await
        }).await
    }
    // async fn get_checkpoint_tree_leaf_hash_f(&self, checkpoint_id: F, leaf_checkpoint_id: F) -> anyhow::Result<QHashOut<F>>;
    pub async fn get_checkpoint_tree_leaf_hash_f(&self, checkpoint_id: F, leaf_checkpoint_id: F) -> anyhow::Result<QHashOut<F>> {
        with_temp_ctx_read_async::<_,_,_,C,D>(|ctx| async move {
            QEDCoordinatorStoreReaderAsync::get_checkpoint_tree_leaf_hash_f(&*ctx.store_reader, checkpoint_id, leaf_checkpoint_id).await
        }).await
    }
    // async fn get_checkpoint_tree_merkle_proof(&self, checkpoint_id: u64, leaf_checkpoint_id: u64) -> anyhow::Result<MerkleProofCore<QHashOut<F>>>;
    pub async fn get_checkpoint_tree_merkle_proof(&self, checkpoint_id: u64, leaf_checkpoint_id: u64) -> anyhow::Result<MerkleProofCore<QHashOut<F>>> {
        with_temp_ctx_read_async::<_,_,_,C,D>(|ctx| async move {
            QEDCoordinatorStoreReaderAsync::get_checkpoint_tree_merkle_proof(&*ctx.store_reader, checkpoint_id, leaf_checkpoint_id).await
        }).await
    }
    // async fn get_checkpoint_tree_merkle_proof_f(&self, checkpoint_id: F, leaf_checkpoint_id: F) -> anyhow::Result<MerkleProofCore<QHashOut<F>>>;
    pub async fn get_checkpoint_tree_merkle_proof_f(&self, checkpoint_id: F, leaf_checkpoint_id: F) -> anyhow::Result<MerkleProofCore<QHashOut<F>>> {
        with_temp_ctx_read_async::<_,_,_,C,D>(|ctx| async move {
            QEDCoordinatorStoreReaderAsync::get_checkpoint_tree_merkle_proof_f(&*ctx.store_reader, checkpoint_id, leaf_checkpoint_id).await
        }).await
    }

    // async fn get_checkpoint_global_state_roots(&self, checkpoint_id: u64) -> anyhow::Result<QEDCheckpointGlobalStateRoots<F>>;
    pub async fn get_checkpoint_global_state_roots(&self, checkpoint_id: u64) -> anyhow::Result<QEDCheckpointGlobalStateRoots<QEDFelt>> {
        with_temp_ctx_read_async::<_,_,_,C,D>(|ctx| async move {
            QEDCoordinatorStoreReaderAsync::get_checkpoint_global_state_roots(&*ctx.store_reader, checkpoint_id).await
        }).await
    }
    // async fn get_checkpoint_sync_info_compact(&self, checkpoint_id: u64) -> anyhow::Result<QEDCheckpointSyncInfoCompact<F>>;
    pub async fn get_checkpoint_sync_info_compact(&self, checkpoint_id: u64) -> anyhow::Result<QEDCheckpointSyncInfoCompact<QEDFelt>> {
        with_temp_ctx_read_async::<_,_,_,C,D>(|ctx| async move {
            QEDCoordinatorStoreReaderAsync::get_checkpoint_sync_info_compact(&*ctx.store_reader, checkpoint_id).await
        }).await
    }

    pub async fn get_user_leaf_data(&self, checkpoint_id: u64, user_id: u64) -> anyhow::Result<QEDUserLeaf<QEDFelt>> {
        with_temp_ctx_read_async::<_,_,_,C,D>(|ctx| async move {
            ctx.store_reader.get_user_leaf_data(checkpoint_id, user_id)
        }).await
    }





}

fn qhash_from_u64_array(arr: [u64; 4]) -> QHashOut<QEDFelt> {
    let elements = arr.map(QEDFelt::from_canonical_u64);
    QHashOut(HashOut { elements })
}