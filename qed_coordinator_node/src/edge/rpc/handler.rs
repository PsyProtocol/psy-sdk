use std::env::args;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::Duration;
use anyhow::bail;
use jsonrpsee::types::ErrorObjectOwned;
use plonky2::field::types::Field;
use plonky2::hash::hash_types::{HashOut};
use plonky2::plonk::config::PoseidonGoldilocksConfig;
use plonky2::plonk::proof::ProofWithPublicInputs;
use qed_store::traits::qdatastore::qtreedata::QTreeDataStoreReaderSync;
use rand::RngCore;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tracing::{debug, error, info, warn};
use kvq::traits::KVQSerializable;
use qed_core::config::network_constants::QED_CHECKPOINT_JOB_ID_CHANNEL;
use qed_core::data::qhashout::QHashOut;
use qed_core::job::drain_queue::{CheckpointDrainQueueConsumerAsyncImm, CheckpointDrainQueueEmitterAsyncImm, WithDrainQueueMetadata};
use qed_core::job::id::{ProvingJobCircuitType, ProvingJobDataId};
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
use qed_node::coordinator::state::user_map::{get_node_redis_pool, get_user_id_by_pubkey};
use qed_node::nimpl::worker_queue_redis::redis_queue::{CEQueueNotification, CPQueueNotification, RedisQueue, CE_NOTIFICATIONS};
use qed_store::config::store_config::{QEDFelt, QEDHasher};
use qed_store::node::coordinator::store_traits::QEDCoordinatorStoreReaderAsync;
use qed_store::store::node::realm::writer_imm::get_user_id_from_registration_id;
use qed_store::traits::qdatastore::qmetadata::QMetaDataStoreReaderSync;
use crate::context::{with_temp_ctx_read_async, UserRegisterState, GLOBAL_DRAIN_QUEUE};
use crate::{CoordinatorEdgeArgs, CoordinatorEdgeQueueArgs};
use crate::communicate::{get_latest_global_coordinator_status, GlobalCoordinatorStatus};
use crate::context::UserRegisterState::{Registered, Registering};
use crate::edge::context::{with_ctx_read_async, GLOBAL_COORD_EDGE_CTX, LATEST_CHECKPOINT_ID, REGISTERED_USERS, REGISTER_USER_COUNTER};
use crate::edge::processor::{build_checkpoint_sync_info};
use crate::edge::rpc::types::GetUserIdRequest;
use crate::rpc::types::CheckpointSyncInfo;

type F = QEDFelt;
type C = PoseidonGoldilocksConfig;
const D: usize = 2;

#[derive(Clone)]
pub struct CoordinatorEdgeHandler {
    args: CoordinatorEdgeQueueArgs,
    notify_queue: RedisQueue,
    cp_listener: Arc<Mutex<Option<JoinHandle<()>>>>,
    realm_job_listener: Arc<Mutex<Option<JoinHandle<()>>>>,
}

impl CoordinatorEdgeHandler {
    pub fn new(args: CoordinatorEdgeArgs) -> anyhow::Result<Self> {
        let redis_uri = args.coordinator_redis_uri.clone();
        Ok(Self {
            args: args.coordinator_edge_queue_args,
            notify_queue: RedisQueue::new(redis_uri.as_str())?,
            cp_listener: Arc::new(Mutex::new(None)),
            realm_job_listener: Arc::new(Mutex::new(None)),
        })
    }
    ///receive StartSync notification from CP
    pub async fn spawn_cp_sync_listener(&self) -> anyhow::Result<()> {
        info!("cp sync listener spawned (polling mode)");
        // note: run this only once
        if self.cp_listener.lock().await.is_some() {
            return Ok(());
        }
        let handle = tokio::spawn(async move {
            let mut last_logged_checkpoint = None;

            loop {
                match get_latest_status_from_global_queue().await {
                    Ok(Some(status)) => {
                        let latest = LATEST_CHECKPOINT_ID.load(Ordering::Relaxed);
                        if Some(status.confirmed_checkpoint_id) != last_logged_checkpoint {
                            info!(
                                "🔔 Detected new checkpoint sync status: {:?}",
                                status
                            );
                            last_logged_checkpoint = Some(status.confirmed_checkpoint_id);
                        }

                        if status.confirmed_checkpoint_id > latest {
                            info!(
                                "🔔 Detected new confirmed checkpoint: {}, current latest: {} → updating local latest",
                                status.confirmed_checkpoint_id, latest
                            );
                            LATEST_CHECKPOINT_ID.store(status.confirmed_checkpoint_id, Ordering::Relaxed);
                            info!("⭐ Coordinator Edge now updated to {}", status.confirmed_checkpoint_id);
                        }else {
                            debug!("ℹ️ No new confirmed checkpoint detected, current latest: {}", latest);
                        }
                    }
                    Ok(None) => {
                        debug!("ℹ️ Coordinator status not yet available.");
                    }
                    Err(e) => {
                        error!("❌ Failed to get coordinator status: {:?}", e);
                    }
                }
                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
            }
        });
        *self.cp_listener.lock().await = Some(handle);
        Ok(())
    }


    pub async fn register_user(&self, zk_user_info: ZKPublicKeyInfo<QEDFelt>) -> anyhow::Result<()> {
        let public_key = zk_user_info.public_key_param;
        let qhash = public_key;

        match check_and_update_user_state(&qhash).await {
            Ok(Registered(user_id)) => {
                info!("🛑 User already registered, user_id = {}", user_id);
                return Ok(());
            }
            Ok(Registering(user_id)) => {
                info!("🕐 User is already in registering process, register_id = {}", user_id);
                return Ok(());
            }
            Err(e) => {
                let err_msg = e.to_string();
                if err_msg.contains("User not registered") {
                    info!("🆕 User not registered yet, start new registration");
                    let register_id = REGISTER_USER_COUNTER.fetch_add(1, Ordering::Relaxed);
                    let user_id = get_user_id_from_registration_id(register_id);
                    REGISTERED_USERS.insert(qhash, Registering(user_id));

                    with_ctx_read_async(|ctx| {
                        let queue = ctx.checkpoint_queue.clone();
                        let zk_user = zk_user_info.clone(); // avoid the lifetime issue

                        async move {
                            info!(
                                "🚀 Pushing new user to drain queue, local_register_id = {}, pub_key = {}",
                                register_id, zk_user.public_key_param
                            );
                            queue.cdq_push_imm(zk_user).await?;
                            info!("✅ Pushed to drain queue.");
                            Ok(())
                        }
                    })
                        .await
                } else {
                    tracing::error!("❌ Unexpected error during user state check: {:?}", e);
                    return Err(e);
                }
            }
        }

    }

    pub async fn get_user_id_logic(&self, qhash: QHashOut<QEDFelt>) -> anyhow::Result<u64> {
        match check_and_update_user_state(&qhash).await {
            Ok(Registered(user_id)) => {
                Ok(user_id)
            }
            Ok(Registering(_)) => {
                anyhow::bail!("User is still registering, user_id not available yet");
            }
            Err(e) => {
                tracing::error!("❌ Failed to check user state: {:?}", e);
                Err(e)
            }
        }
    }

    pub async fn deploy_contract(
        &self,
        contract: QBCDeployContract<QEDFelt>,
    ) -> anyhow::Result<()> {
        let next_checkpoint_id = LATEST_CHECKPOINT_ID.load(Ordering::Relaxed) + 1;
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

        //if circuit type is GUTANoChange, disable the proof
        if input.circuit_type == ProvingJobCircuitType::GUTANoChange {
            info!("⚠️ GUTANoChange proof, disabling it");
            return Ok(());
        }
        tracing::info!("✅ verified guta result proof public input: {:?} ", proof.public_inputs);

        // verify state consistency
        // let old_root = store_reader
        //     .get_user_latest_top_tree_cap_root(config.realm_root_level, input.realm_id)
        //     .await.unwrap();

        // if old_root != input.top_line_proof.old_root && old_root != input.top_line_proof.new_root {
        //     anyhow::bail!("invalid top line proof old value from realm");
        // }

        // build queue item
        let queue_item =
            input.to_queue_item(config.guta_channel_id, config.realm_root_level as u32);
        let proof_id = queue_item.proof_id;

        // write to proof store
        proof_store.set_proof_by_id(proof_id, &proof).await?;
        tracing::info!("✅ wrote guta result to proof store");
        checkpoint_queue.cdq_push_imm(queue_item).await?;
        tracing::info!("✅ wrote guta result to proof store end");

        Ok(())
    }

    pub async fn build_block(&self) -> anyhow::Result<()> {

        let next_checkpoint = LATEST_CHECKPOINT_ID.load(Ordering::Relaxed) + 1;
        info!("🚀 build_block called, want to build {}", next_checkpoint);
        self.notify_queue
            .clone()
            .dispatch(CE_NOTIFICATIONS, CEQueueNotification::StartProduceBlock {next_checkpoint})?;
        info!("✅ build_block cmd have send to CP");
        Ok(())
    }

    pub async fn get_checkpoint_sync_info(
        &self,
        request_checkpoint_id: u64,
    ) -> anyhow::Result<CheckpointSyncInfo> {

        let latest = LATEST_CHECKPOINT_ID.load(Ordering::Relaxed);

        if request_checkpoint_id > latest {
            bail!(
            "Requested checkpoint_id {} exceeds latest local checkpoint_id {}",
            request_checkpoint_id,
            latest
            );
        }
        let compact = with_temp_ctx_read_async::<_, _, _, C, D>(
            self.args.clone(),
            |ctx| async move {
                QEDCoordinatorStoreReaderAsync::get_checkpoint_sync_info_compact(
                    &*ctx.store_reader,
                    request_checkpoint_id,
                )
                    .await
            },
        )
            .await?;

        Ok(build_checkpoint_sync_info(latest, compact))
    }
    // async fn get_contract_leaf_data(&self, contract_id: u64) -> anyhow::Result<QEDContractLeaf<F>>;
    pub async fn get_contract_leaf_data(&self, contract_id: u64) -> anyhow::Result<QEDContractLeaf<QEDFelt>> {
        with_temp_ctx_read_async::<_,_,_,C,D>(self.args.clone(),|ctx| async move {
            QEDCoordinatorStoreReaderAsync::get_contract_leaf_data(&*ctx.store_reader, contract_id).await
        }).await
    }

    // async fn get_contract_leaf_data_f(&self, contract_id: F) -> anyhow::Result<QEDContractLeaf<F>>;
    pub async fn get_contract_leaf_data_f(&self, contract_id: F) -> anyhow::Result<QEDContractLeaf<F>> {
        with_temp_ctx_read_async::<_,_,_,C,D>(self.args.clone(),|ctx| async move {
            QEDCoordinatorStoreReaderAsync::get_contract_leaf_data_f(&*ctx.store_reader, contract_id).await
        }).await
    }
    // async fn get_checkpoint_leaf_data(&self, checkpoint_id: u64) -> anyhow::Result<QEDCheckpointLeaf<F>>;
    pub async fn get_checkpoint_leaf_data(&self, checkpoint_id: u64) -> anyhow::Result<QEDCheckpointLeaf<QEDFelt>> {
        with_temp_ctx_read_async::<_,_,_,C,D>(self.args.clone(),|ctx| async move {
            QEDCoordinatorStoreReaderAsync::get_checkpoint_leaf_data(&*ctx.store_reader, checkpoint_id).await
        }).await
    }
    // async fn get_checkpoint_leaf_data_f(&self, checkpoint_id: F) -> anyhow::Result<QEDCheckpointLeaf<F>>;
    pub async fn get_checkpoint_leaf_data_f(&self, checkpoint_id: F) -> anyhow::Result<QEDCheckpointLeaf<F>> {
        with_temp_ctx_read_async::<_,_,_,C,D>(self.args.clone(),|ctx| async move {
            QEDCoordinatorStoreReaderAsync::get_checkpoint_leaf_data_f(&*ctx.store_reader, checkpoint_id).await
        }).await
    }

    // async fn get_contract_code_definition(&self, contract_id: u64) -> anyhow::Result<ContractCodeDefinition>;
    pub async fn get_contract_code_definition(&self, contract_id: u64) -> anyhow::Result<ContractCodeDefinition> {
        with_temp_ctx_read_async::<_,_,_,C,D>(self.args.clone(),|ctx| async move {
            QEDCoordinatorStoreReaderAsync::get_contract_code_definition(&*ctx.store_reader, contract_id).await
        }).await
    }

    // async fn get_contract_code_definition_f(&self, contract_id: F) -> anyhow::Result<ContractCodeDefinition>;
    pub async fn get_contract_code_definition_f(&self, contract_id: F) -> anyhow::Result<ContractCodeDefinition> {
        with_temp_ctx_read_async::<_,_,_,C,D>(self.args.clone(),|ctx| async move {
            QEDCoordinatorStoreReaderAsync::get_contract_code_definition_f(&*ctx.store_reader, contract_id).await
        }).await
    }

    // async fn get_latest_l2_block_state(&self) -> anyhow::Result<QEDL2BlockState>;
    pub async fn get_latest_l2_block_state(&self) -> anyhow::Result<QEDL2BlockState> {

        with_temp_ctx_read_async::<_,_,_,C,D>(self.args.clone(),|ctx| async move {
            QEDCoordinatorStoreReaderAsync::get_latest_l2_block_state(&*ctx.store_reader).await
        }).await
    }

    // async fn get_l2_block_state(&self, checkpoint_id: u64) -> anyhow::Result<QEDL2BlockState>;

    pub async fn get_l2_block_state(&self, checkpoint_id: u64) -> anyhow::Result<QEDL2BlockState> {
        with_temp_ctx_read_async::<_,_,_,C,D>(self.args.clone(),|ctx| async move {
            QEDCoordinatorStoreReaderAsync::get_l2_block_state(&*ctx.store_reader, checkpoint_id).await
        }).await
    }


    // async fn get_l2_block_state_f(&self, checkpoint_id: F) -> anyhow::Result<QEDL2BlockState>;
    pub async fn get_l2_block_state_f(&self, checkpoint_id: F) -> anyhow::Result<QEDL2BlockState> {
        with_temp_ctx_read_async::<_,_,_,C,D>(self.args.clone(),|ctx| async move {
            QEDCoordinatorStoreReaderAsync::get_l2_block_state_f(&*ctx.store_reader, checkpoint_id).await
        }).await
    }

    // async fn get_user_registration_tree_root(&self, checkpoint_id: u64) -> anyhow::Result<QHashOut<F>>;
    pub async fn get_user_registration_tree_root(&self, checkpoint_id: u64) -> anyhow::Result<QHashOut<F>> {
        with_temp_ctx_read_async::<_,_,_,C,D>(self.args.clone(),|ctx| async move {
            QEDCoordinatorStoreReaderAsync::get_user_registration_tree_root(&*ctx.store_reader, checkpoint_id).await
        }).await
    }
    // async fn get_user_registration_tree_root_f(&self, checkpoint_id: F) -> anyhow::Result<QHashOut<F>>;
    pub async fn get_user_registration_tree_root_f(&self, checkpoint_id: F) -> anyhow::Result<QHashOut<F>> {
        with_temp_ctx_read_async::<_,_,_,C,D>(self.args.clone(),|ctx| async move {
            QEDCoordinatorStoreReaderAsync::get_user_registration_tree_root_f(&*ctx.store_reader, checkpoint_id).await
        }).await
    }
    // async fn get_user_registration_tree_leaf_hash(&self, checkpoint_id: u64, leaf_index: u64) -> anyhow::Result<QHashOut<F>>;
    pub async fn get_user_registration_tree_leaf_hash(&self, checkpoint_id: u64, leaf_index: u64) -> anyhow::Result<QHashOut<F>> {
        with_temp_ctx_read_async::<_,_,_,C,D>(self.args.clone(),|ctx| async move {
            QEDCoordinatorStoreReaderAsync::get_user_registration_tree_leaf_hash(&*ctx.store_reader, checkpoint_id, leaf_index).await
        }).await
    }
    // async fn get_user_registration_tree_leaf_hash_f(&self, checkpoint_id: F, leaf_index: F) -> anyhow::Result<QHashOut<F>>;
    pub async fn get_user_registration_tree_leaf_hash_f(&self, checkpoint_id: F, leaf_index: F) -> anyhow::Result<QHashOut<F>> {
        with_temp_ctx_read_async::<_,_,_,C,D>(self.args.clone(),|ctx| async move {
            QEDCoordinatorStoreReaderAsync::get_user_registration_tree_leaf_hash_f(&*ctx.store_reader, checkpoint_id, leaf_index).await
        }).await
    }

    // async fn get_user_registration_tree_merkle_proof(&self, checkpoint_id: u64, leaf_index: u64) -> anyhow::Result<MerkleProofCore<QHashOut<F>>>;
    pub async fn get_user_registration_tree_merkle_proof(&self, checkpoint_id: u64, leaf_index: u64) -> anyhow::Result<MerkleProofCore<QHashOut<F>>> {
        with_temp_ctx_read_async::<_,_,_,C,D>(self.args.clone(),|ctx| async move {
            QEDCoordinatorStoreReaderAsync::get_user_registration_tree_merkle_proof(&*ctx.store_reader, checkpoint_id, leaf_index).await
        }).await
    }
    // async fn get_user_registration_tree_merkle_proof_f(&self, checkpoint_id: F, leaf_index: F) -> anyhow::Result<MerkleProofCore<QHashOut<F>>>;
    pub async fn get_user_registration_tree_merkle_proof_f(&self, checkpoint_id: F, leaf_index: F) -> anyhow::Result<MerkleProofCore<QHashOut<F>>> {
        with_temp_ctx_read_async::<_,_,_,C,D>(self.args.clone(),|ctx| async move {
            QEDCoordinatorStoreReaderAsync::get_user_registration_tree_merkle_proof_f(&*ctx.store_reader, checkpoint_id, leaf_index).await
        }).await
    }
    //

    // async fn get_user_tree_root(&self, checkpoint_id: u64) -> anyhow::Result<QHashOut<F>>;
    pub async fn get_user_tree_root(&self, checkpoint_id: u64) -> anyhow::Result<QHashOut<F>> {
        with_temp_ctx_read_async::<_,_,_,C,D>(self.args.clone(),|ctx| async move {
            QEDCoordinatorStoreReaderAsync::get_user_tree_root(&*ctx.store_reader, checkpoint_id).await
        }).await
    }
    // async fn get_user_tree_root_f(&self, checkpoint_id: F) -> anyhow::Result<QHashOut<F>>;
    pub async fn get_user_tree_root_f(&self, checkpoint_id: F) -> anyhow::Result<QHashOut<F>> {
        with_temp_ctx_read_async::<_,_,_,C,D>(self.args.clone(),|ctx| async move {
            QEDCoordinatorStoreReaderAsync::get_user_tree_root_f(&*ctx.store_reader, checkpoint_id).await
        }).await
    }
    // async fn get_user_sub_tree_merkle_proof(&self, checkpoint_id: u64, root_level: u8, leaf_level: u8, leaf_index: u64) -> anyhow::Result<MerkleProofCore<QHashOut<F>>>;
    pub async fn get_user_sub_tree_merkle_proof(&self, checkpoint_id: u64, root_level: u8, leaf_level: u8, leaf_index: u64) -> anyhow::Result<MerkleProofCore<QHashOut<F>>> {
        with_temp_ctx_read_async::<_,_,_,C,D>(self.args.clone(),|ctx| async move {
            QEDCoordinatorStoreReaderAsync::get_user_sub_tree_merkle_proof(&*ctx.store_reader, checkpoint_id, root_level, leaf_level, leaf_index).await
        }).await
    }
    // async fn get_user_top_tree_merkle_proof(&self, checkpoint_id: u64, leaf_level: u8, leaf_index: u64) -> anyhow::Result<MerkleProofCore<QHashOut<F>>>;
    pub async fn get_user_top_tree_merkle_proof(&self, checkpoint_id: u64, leaf_level: u8, leaf_index: u64) -> anyhow::Result<MerkleProofCore<QHashOut<F>>> {
        with_temp_ctx_read_async::<_,_,_,C,D>(self.args.clone(),|ctx| async move {
            QEDCoordinatorStoreReaderAsync::get_user_top_tree_merkle_proof(&*ctx.store_reader, checkpoint_id, leaf_level, leaf_index).await
        }).await
    }
    // async fn get_user_top_tree_cap_root(&self, checkpoint_id: u64, cap_level: u8, cap_index: u64) -> anyhow::Result<QHashOut<F>>;
    pub async fn get_user_top_tree_cap_root(&self, checkpoint_id: u64, cap_level: u8, cap_index: u64) -> anyhow::Result<QHashOut<F>> {
        with_temp_ctx_read_async::<_,_,_,C,D>(self.args.clone(),|ctx| async move {
            QEDCoordinatorStoreReaderAsync::get_user_top_tree_cap_root(&*ctx.store_reader, checkpoint_id, cap_level, cap_index).await
        }).await
    }
    // async fn get_user_latest_top_tree_cap_root(&self, cap_level: u8, cap_index: u64) -> anyhow::Result<QHashOut<F>>;
    pub async fn get_user_latest_top_tree_cap_root(&self, cap_level: u8, cap_index: u64) -> anyhow::Result<QHashOut<F>> {
        with_temp_ctx_read_async::<_,_,_,C,D>(self.args.clone(),|ctx| async move {
            QEDCoordinatorStoreReaderAsync::get_user_latest_top_tree_cap_root(&*ctx.store_reader, cap_level, cap_index).await
        }).await
    }
    //
    //
    // async fn get_contract_function_tree_root(&self, checkpoint_id: u64, contract_id: u32) -> anyhow::Result<QHashOut<F>>;
    pub async fn get_contract_function_tree_root(&self, checkpoint_id: u64, contract_id: u32) -> anyhow::Result<QHashOut<F>> {
        with_temp_ctx_read_async::<_,_,_,C,D>(self.args.clone(),|ctx| async move {
            QEDCoordinatorStoreReaderAsync::get_contract_function_tree_root(&*ctx.store_reader, checkpoint_id, contract_id).await
        }).await
    }

    // async fn get_contract_function_tree_root_f(&self, checkpoint_id: F, contract_id: F) -> anyhow::Result<QHashOut<F>>;
    pub async fn get_contract_function_tree_root_f(&self, checkpoint_id: F, contract_id: F) -> anyhow::Result<QHashOut<F>> {
        with_temp_ctx_read_async::<_,_,_,C,D>(self.args.clone(),|ctx| async move {
            QEDCoordinatorStoreReaderAsync::get_contract_function_tree_root_f(&*ctx.store_reader, checkpoint_id, contract_id).await
        }).await
    }
    // async fn get_contract_function_tree_leaf_hash(&self, checkpoint_id: u64, contract_id: u32, function_id: u32) -> anyhow::Result<QHashOut<F>>;
    pub async fn get_contract_function_tree_leaf_hash(&self, checkpoint_id: u64, contract_id: u32, function_id: u32) -> anyhow::Result<QHashOut<F>> {
        with_temp_ctx_read_async::<_,_,_,C,D>(self.args.clone(),|ctx| async move {
            QEDCoordinatorStoreReaderAsync::get_contract_function_tree_leaf_hash(&*ctx.store_reader, checkpoint_id, contract_id, function_id).await
        }).await
    }
    // async fn get_contract_function_tree_leaf_hash_f(&self, checkpoint_id: F, contract_id: F, function_id: F) -> anyhow::Result<QHashOut<F>>;
    pub async fn get_contract_function_tree_leaf_hash_f(&self, checkpoint_id: F, contract_id: F, function_id: F) -> anyhow::Result<QHashOut<F>> {
        with_temp_ctx_read_async::<_,_,_,C,D>(self.args.clone(),|ctx| async move {
            QEDCoordinatorStoreReaderAsync::get_contract_function_tree_leaf_hash_f(&*ctx.store_reader, checkpoint_id, contract_id, function_id).await
        }).await
    }
    // async fn get_contract_function_tree_merkle_proof(&self, checkpoint_id: u64, contract_id: u32, function_id: u32) -> anyhow::Result<MerkleProofCore<QHashOut<F>>>;
    pub async fn get_contract_function_tree_merkle_proof(&self, checkpoint_id: u64, contract_id: u32, function_id: u32) -> anyhow::Result<MerkleProofCore<QHashOut<F>>> {
        with_temp_ctx_read_async::<_,_,_,C,D>(self.args.clone(),|ctx| async move {
            QEDCoordinatorStoreReaderAsync::get_contract_function_tree_merkle_proof(&*ctx.store_reader, checkpoint_id, contract_id, function_id).await
        }).await
    }
    // async fn get_contract_function_tree_merkle_proof_f(&self, checkpoint_id: F, contract_id: F, function_id: F) -> anyhow::Result<MerkleProofCore<QHashOut<F>>>;
    pub async fn get_contract_function_tree_merkle_proof_f(&self, checkpoint_id: F, contract_id: F, function_id: F) -> anyhow::Result<MerkleProofCore<QHashOut<F>>> {
        with_temp_ctx_read_async::<_,_,_,C,D>(self.args.clone(),|ctx| async move {
            QEDCoordinatorStoreReaderAsync::get_contract_function_tree_merkle_proof_f(&*ctx.store_reader, checkpoint_id, contract_id, function_id).await
        }).await
    }
    //
    //

    // async fn get_contract_tree_root(&self, checkpoint_id: u64) -> anyhow::Result<QHashOut<F>>;
    pub async fn get_contract_tree_root(&self, checkpoint_id: u64) -> anyhow::Result<QHashOut<F>> {
        with_temp_ctx_read_async::<_,_,_,C,D>(self.args.clone(),|ctx| async move {
            QEDCoordinatorStoreReaderAsync::get_contract_tree_root(&*ctx.store_reader, checkpoint_id).await
        }).await
    }
    // async fn get_contract_tree_root_f(&self, checkpoint_id: F) -> anyhow::Result<QHashOut<F>>;
    pub async fn get_contract_tree_root_f(&self, checkpoint_id: F) -> anyhow::Result<QHashOut<F>> {
        with_temp_ctx_read_async::<_,_,_,C,D>(self.args.clone(),|ctx| async move {
            QEDCoordinatorStoreReaderAsync::get_contract_tree_root_f(&*ctx.store_reader, checkpoint_id).await
        }).await
    }
    // async fn get_contract_tree_leaf_hash(&self, checkpoint_id: u64, contract_id: u32) -> anyhow::Result<QHashOut<F>>;
    pub async fn get_contract_tree_leaf_hash(&self, checkpoint_id: u64, contract_id: u32) -> anyhow::Result<QHashOut<F>> {
        with_temp_ctx_read_async::<_,_,_,C,D>(self.args.clone(),|ctx| async move {
            QEDCoordinatorStoreReaderAsync::get_contract_tree_leaf_hash(&*ctx.store_reader, checkpoint_id, contract_id).await
        }).await
    }
    // async fn get_contract_tree_leaf_hash_f(&self, checkpoint_id: F, contract_id: F) -> anyhow::Result<QHashOut<F>>;
    pub async fn get_contract_tree_leaf_hash_f(&self, checkpoint_id: F, contract_id: F) -> anyhow::Result<QHashOut<F>> {
        with_temp_ctx_read_async::<_,_,_,C,D>(self.args.clone(),|ctx| async move {
            QEDCoordinatorStoreReaderAsync::get_contract_tree_leaf_hash_f(&*ctx.store_reader, checkpoint_id, contract_id).await
        }).await
    }
    // async fn get_contract_tree_merkle_proof(&self, checkpoint_id: u64, contract_id: u32) -> anyhow::Result<MerkleProofCore<QHashOut<F>>>;
    pub async fn get_contract_tree_merkle_proof(&self, checkpoint_id: u64, contract_id: u32) -> anyhow::Result<MerkleProofCore<QHashOut<F>>> {
        with_temp_ctx_read_async::<_,_,_,C,D>(self.args.clone(),|ctx| async move {
            QEDCoordinatorStoreReaderAsync::get_contract_tree_merkle_proof(&*ctx.store_reader, checkpoint_id, contract_id).await
        }).await
    }
    // async fn get_contract_tree_merkle_proof_f(&self, checkpoint_id: F, contract_id: F) -> anyhow::Result<MerkleProofCore<QHashOut<F>>>;
    pub async fn get_contract_tree_merkle_proof_f(&self, checkpoint_id: F, contract_id: F) -> anyhow::Result<MerkleProofCore<QHashOut<F>>> {
        with_temp_ctx_read_async::<_,_,_,C,D>(self.args.clone(),|ctx| async move {
            QEDCoordinatorStoreReaderAsync::get_contract_tree_merkle_proof_f(&*ctx.store_reader, checkpoint_id, contract_id).await
        }).await
    }

    // async fn get_deposit_tree_root(&self, checkpoint_id: u64) -> anyhow::Result<QHashOut<F>>;
    pub async fn get_deposit_tree_root(&self, checkpoint_id: u64) -> anyhow::Result<QHashOut<F>> {
        with_temp_ctx_read_async::<_,_,_,C,D>(self.args.clone(),|ctx| async move {
            QEDCoordinatorStoreReaderAsync::get_deposit_tree_root(&*ctx.store_reader, checkpoint_id).await
        }).await
    }

    // async fn get_deposit_tree_root_f(&self, checkpoint_id: F) -> anyhow::Result<QHashOut<F>>;
    pub async fn get_deposit_tree_root_f(&self, checkpoint_id: F) -> anyhow::Result<QHashOut<F>> {
        with_temp_ctx_read_async::<_,_,_,C,D>(self.args.clone(),|ctx| async move {
            QEDCoordinatorStoreReaderAsync::get_deposit_tree_root_f(&*ctx.store_reader, checkpoint_id).await
        }).await
    }
    // async fn get_deposit_tree_leaf_hash(&self, checkpoint_id: u64, deposit_id: u32) -> anyhow::Result<QHashOut<F>>;
    pub async fn get_deposit_tree_leaf_hash(&self, checkpoint_id: u64, deposit_id: u32) -> anyhow::Result<QHashOut<F>> {
        with_temp_ctx_read_async::<_,_,_,C,D>(self.args.clone(),|ctx| async move {
            QEDCoordinatorStoreReaderAsync::get_deposit_tree_leaf_hash(&*ctx.store_reader, checkpoint_id, deposit_id).await
        }).await
    }
    // async fn get_deposit_tree_leaf_hash_f(&self, checkpoint_id: F, deposit_id: F) -> anyhow::Result<QHashOut<F>>;
    pub async fn get_deposit_tree_leaf_hash_f(&self, checkpoint_id: F, deposit_id: F) -> anyhow::Result<QHashOut<F>> {
        with_temp_ctx_read_async::<_,_,_,C,D>(self.args.clone(),|ctx| async move {
            QEDCoordinatorStoreReaderAsync::get_deposit_tree_leaf_hash_f(&*ctx.store_reader, checkpoint_id, deposit_id).await
        }).await
    }
    // async fn get_deposit_tree_merkle_proof(&self, checkpoint_id: u64, deposit_id: u32) -> anyhow::Result<MerkleProofCore<QHashOut<F>>>;
    pub async fn get_deposit_tree_merkle_proof(&self, checkpoint_id: u64, deposit_id: u32) -> anyhow::Result<MerkleProofCore<QHashOut<F>>> {
        with_temp_ctx_read_async::<_,_,_,C,D>(self.args.clone(),|ctx| async move {
            QEDCoordinatorStoreReaderAsync::get_deposit_tree_merkle_proof(&*ctx.store_reader, checkpoint_id, deposit_id).await
        }).await
    }
    // async fn get_deposit_tree_merkle_proof_f(&self, checkpoint_id: F, deposit_id: F) -> anyhow::Result<MerkleProofCore<QHashOut<F>>>;
    pub async fn get_deposit_tree_merkle_proof_f(&self, checkpoint_id: F, deposit_id: F) -> anyhow::Result<MerkleProofCore<QHashOut<F>>> {
        with_temp_ctx_read_async::<_,_,_,C,D>(self.args.clone(),|ctx| async move {
            QEDCoordinatorStoreReaderAsync::get_deposit_tree_merkle_proof_f(&*ctx.store_reader, checkpoint_id, deposit_id).await
        }).await
    }
    //
    //
    // async fn get_withdrawal_tree_root(&self, checkpoint_id: u64) -> anyhow::Result<QHashOut<F>>;
    pub async fn get_withdrawal_tree_root(&self, checkpoint_id: u64) -> anyhow::Result<QHashOut<F>> {
        with_temp_ctx_read_async::<_,_,_,C,D>(self.args.clone(),|ctx| async move {
            QEDCoordinatorStoreReaderAsync::get_withdrawal_tree_root(&*ctx.store_reader, checkpoint_id).await
        }).await
    }

    // async fn get_withdrawal_tree_root_f(&self, checkpoint_id: F) -> anyhow::Result<QHashOut<F>>;
    pub async fn get_withdrawal_tree_root_f(&self, checkpoint_id: F) -> anyhow::Result<QHashOut<F>> {
        with_temp_ctx_read_async::<_,_,_,C,D>(self.args.clone(),|ctx| async move {
            QEDCoordinatorStoreReaderAsync::get_withdrawal_tree_root_f(&*ctx.store_reader, checkpoint_id).await
        }).await
    }
    // async fn get_withdrawal_tree_leaf_hash(&self, checkpoint_id: u64, withdrawal_id: u32) -> anyhow::Result<QHashOut<F>>;
    pub async fn get_withdrawal_tree_leaf_hash(&self, checkpoint_id: u64, withdrawal_id: u32) -> anyhow::Result<QHashOut<F>> {
        with_temp_ctx_read_async::<_,_,_,C,D>(self.args.clone(),|ctx| async move {
            QEDCoordinatorStoreReaderAsync::get_withdrawal_tree_leaf_hash(&*ctx.store_reader, checkpoint_id, withdrawal_id).await
        }).await
    }
    // async fn get_withdrawal_tree_leaf_hash_f(&self, checkpoint_id: F, withdrawal_id: F) -> anyhow::Result<QHashOut<F>>;
    pub async fn get_withdrawal_tree_leaf_hash_f(&self, checkpoint_id: F, withdrawal_id: F) -> anyhow::Result<QHashOut<F>> {
        with_temp_ctx_read_async::<_,_,_,C,D>(self.args.clone(),|ctx| async move {
            QEDCoordinatorStoreReaderAsync::get_withdrawal_tree_leaf_hash_f(&*ctx.store_reader, checkpoint_id, withdrawal_id).await
        }).await
    }
    // async fn get_withdrawal_tree_merkle_proof(&self, checkpoint_id: u64, withdrawal_id: u32) -> anyhow::Result<MerkleProofCore<QHashOut<F>>>;
    pub async fn get_withdrawal_tree_merkle_proof(&self, checkpoint_id: u64, withdrawal_id: u32) -> anyhow::Result<MerkleProofCore<QHashOut<F>>> {
        with_temp_ctx_read_async::<_,_,_,C,D>(self.args.clone(),|ctx| async move {
            QEDCoordinatorStoreReaderAsync::get_withdrawal_tree_merkle_proof(&*ctx.store_reader, checkpoint_id, withdrawal_id).await
        }).await
    }
    // async fn get_withdrawal_tree_merkle_proof_f(&self, checkpoint_id: F, withdrawal_id: F) -> anyhow::Result<MerkleProofCore<QHashOut<F>>>;
    pub async fn get_withdrawal_tree_merkle_proof_f(&self, checkpoint_id: F, withdrawal_id: F) -> anyhow::Result<MerkleProofCore<QHashOut<F>>> {
        with_temp_ctx_read_async::<_,_,_,C,D>(self.args.clone(),|ctx| async move {
            QEDCoordinatorStoreReaderAsync::get_withdrawal_tree_merkle_proof_f(&*ctx.store_reader, checkpoint_id, withdrawal_id).await
        }).await
    }
    //
    // async fn get_latest_checkpoint_tree_root(&self) -> anyhow::Result<QHashOut<F>>;
    pub async fn get_latest_checkpoint_tree_root(&self) -> anyhow::Result<QHashOut<F>> {
        with_temp_ctx_read_async::<_,_,_,C,D>(self.args.clone(),|ctx| async move {
            QEDCoordinatorStoreReaderAsync::get_latest_checkpoint_tree_root(&*ctx.store_reader).await
        }).await
    }
    // async fn get_checkpoint_tree_root(&self, checkpoint_id: u64) -> anyhow::Result<QHashOut<F>>;
    pub async fn get_checkpoint_tree_root(&self, checkpoint_id: u64) -> anyhow::Result<QHashOut<F>> {
        with_temp_ctx_read_async::<_,_,_,C,D>(self.args.clone(),|ctx| async move {
            QEDCoordinatorStoreReaderAsync::get_checkpoint_tree_root(&*ctx.store_reader, checkpoint_id).await
        }).await
    }
    // async fn get_checkpoint_tree_root_f(&self, checkpoint_id: F) -> anyhow::Result<QHashOut<F>>;
    pub async fn get_checkpoint_tree_root_f(&self, checkpoint_id: F) -> anyhow::Result<QHashOut<F>> {
        with_temp_ctx_read_async::<_,_,_,C,D>(self.args.clone(),|ctx| async move {
            QEDCoordinatorStoreReaderAsync::get_checkpoint_tree_root_f(&*ctx.store_reader, checkpoint_id).await
        }).await
    }
    // async fn get_checkpoint_tree_leaf_hash(&self, checkpoint_id: u64, leaf_checkpoint_id: u64) -> anyhow::Result<QHashOut<F>>;
    pub async fn get_checkpoint_tree_leaf_hash(&self, checkpoint_id: u64, leaf_checkpoint_id: u64) -> anyhow::Result<QHashOut<F>> {
        with_temp_ctx_read_async::<_,_,_,C,D>(self.args.clone(),|ctx| async move {
            QEDCoordinatorStoreReaderAsync::get_checkpoint_tree_leaf_hash(&*ctx.store_reader, checkpoint_id, leaf_checkpoint_id).await
        }).await
    }
    // async fn get_checkpoint_tree_leaf_hash_f(&self, checkpoint_id: F, leaf_checkpoint_id: F) -> anyhow::Result<QHashOut<F>>;
    pub async fn get_checkpoint_tree_leaf_hash_f(&self, checkpoint_id: F, leaf_checkpoint_id: F) -> anyhow::Result<QHashOut<F>> {
        with_temp_ctx_read_async::<_,_,_,C,D>(self.args.clone(),|ctx| async move {
            QEDCoordinatorStoreReaderAsync::get_checkpoint_tree_leaf_hash_f(&*ctx.store_reader, checkpoint_id, leaf_checkpoint_id).await
        }).await
    }
    // async fn get_checkpoint_tree_merkle_proof(&self, checkpoint_id: u64, leaf_checkpoint_id: u64) -> anyhow::Result<MerkleProofCore<QHashOut<F>>>;
    pub async fn get_checkpoint_tree_merkle_proof(&self, checkpoint_id: u64, leaf_checkpoint_id: u64) -> anyhow::Result<MerkleProofCore<QHashOut<F>>> {
        with_temp_ctx_read_async::<_,_,_,C,D>(self.args.clone(),|ctx| async move {
            QEDCoordinatorStoreReaderAsync::get_checkpoint_tree_merkle_proof(&*ctx.store_reader, checkpoint_id, leaf_checkpoint_id).await
        }).await
    }
    // async fn get_checkpoint_tree_merkle_proof_f(&self, checkpoint_id: F, leaf_checkpoint_id: F) -> anyhow::Result<MerkleProofCore<QHashOut<F>>>;
    pub async fn get_checkpoint_tree_merkle_proof_f(&self, checkpoint_id: F, leaf_checkpoint_id: F) -> anyhow::Result<MerkleProofCore<QHashOut<F>>> {
        with_temp_ctx_read_async::<_,_,_,C,D>(self.args.clone(),|ctx| async move {
            QEDCoordinatorStoreReaderAsync::get_checkpoint_tree_merkle_proof_f(&*ctx.store_reader, checkpoint_id, leaf_checkpoint_id).await
        }).await
    }

    // async fn get_checkpoint_global_state_roots(&self, checkpoint_id: u64) -> anyhow::Result<QEDCheckpointGlobalStateRoots<F>>;
    pub async fn get_checkpoint_global_state_roots(&self, checkpoint_id: u64) -> anyhow::Result<QEDCheckpointGlobalStateRoots<QEDFelt>> {
        with_temp_ctx_read_async::<_,_,_,C,D>(self.args.clone(),|ctx| async move {
            QEDCoordinatorStoreReaderAsync::get_checkpoint_global_state_roots(&*ctx.store_reader, checkpoint_id).await
        }).await
    }
    // async fn get_checkpoint_sync_info_compact(&self, checkpoint_id: u64) -> anyhow::Result<QEDCheckpointSyncInfoCompact<F>>;
    pub async fn get_checkpoint_sync_info_compact(&self, checkpoint_id: u64) -> anyhow::Result<QEDCheckpointSyncInfoCompact<QEDFelt>> {
        with_temp_ctx_read_async::<_,_,_,C,D>(self.args.clone(),|ctx| async move {
            QEDCoordinatorStoreReaderAsync::get_checkpoint_sync_info_compact(&*ctx.store_reader, checkpoint_id).await
        }).await
    }

    pub async fn get_user_leaf_data(&self, checkpoint_id: u64, user_id: u64) -> anyhow::Result<QEDUserLeaf<QEDFelt>> {
        with_temp_ctx_read_async::<_,_,_,C,D>(self.args.clone(),|ctx| async move {
            ctx.store_reader.get_user_leaf_data(checkpoint_id, user_id)
        }).await
    }

    pub async fn get_user_tree_merkle_proof(
        &self,
        checkpoint_id: u64,
        user_id: u64,
    ) -> anyhow::Result<MerkleProofCore<QHashOut<QEDFelt>>> {
        with_temp_ctx_read_async::<_, _, _, C, D>(self.args.clone(),|ctx| async move {
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
        with_temp_ctx_read_async::<_, _, _, C, D>(self.args.clone(),|ctx| async move {
            ctx.store_reader
                .get_user_tree_merkle_proof_f(checkpoint_id, user_id)
        })
        .await
    }
}

fn qhash_from_u64_array(arr: [u64; 4]) -> QHashOut<QEDFelt> {
    let elements = arr.map(QEDFelt::from_canonical_u64);
    QHashOut(HashOut { elements })
}

pub async fn check_and_update_user_state(qhash: &QHashOut<QEDFelt>) -> anyhow::Result<UserRegisterState> {
    let pubkey_hex = hex::encode(qhash.to_bytes()?);

    match REGISTERED_USERS.get_mut(qhash) {
        Some(mut user_state) => match *user_state {
            Registering(user_id) => {
                info!("🛑 User is registering (id: {}), checking status...", user_id);
                let redis_pool = get_node_redis_pool()?;

                match get_user_id_by_pubkey(redis_pool.as_ref(), &pubkey_hex).await {
                    Ok(Some(confirmed_user_id)) => {
                        *user_state = Registered(confirmed_user_id);
                        info!("✅ User registered successfully, updated local state to Registered({})", confirmed_user_id);
                        Ok(Registered(confirmed_user_id))
                    }
                    Ok(None) => {
                        info!("🕐 User is still registering, waiting for confirmation...");
                        Ok(Registering(user_id))
                    }
                    Err(e) => {
                        error!("❌ Database query failed while checking registration: {:?}", e);
                        Err(e)
                    }
                }
            }
            Registered(user_id) => {
                info!("✅ User is already registered: {}", user_id);
                Ok(Registered(user_id))
            }
        },
        None => {
            tracing::warn!("🛑 User not found in local cache: {}, then search the database...", pubkey_hex);
            let redis_pool = get_node_redis_pool()?;
            match get_user_id_by_pubkey(redis_pool.as_ref(), &pubkey_hex).await {
                Ok(Some(user_id)) => {
                    REGISTERED_USERS.insert(*qhash, Registered(user_id));
                    tracing::info!("✅ Found user in database. Inserted Registered({}) into cache", user_id);
                    Ok(Registered(user_id))
                }
                Ok(None) => {
                    tracing::info!("🆕 User not found in database either.");
                    anyhow::bail!("User not registered");
                }
                Err(e) => {
                    tracing::error!("❌ database query failed: {:?}", e);
                    Err(e)
                }
            }
        }
    }
}


pub async fn get_latest_status_from_global_queue() -> anyhow::Result<Option<GlobalCoordinatorStatus>> {
    let drain_queue = GLOBAL_DRAIN_QUEUE
        .get()
        .ok_or_else(|| anyhow::anyhow!("GLOBAL_DRAIN_QUEUE is not initialized"))?;

    get_latest_global_coordinator_status(drain_queue).await
}