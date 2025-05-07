use crate::error::RpcError;
use crate::rpc::{CheckpointSyncInfo, RealmEdgeRpcServer};
use crate::{RealmInternalQueue, C, D, F, H};
use async_trait::async_trait;
use jsonrpsee::core::{client::ClientT, RpcResult};
use jsonrpsee::rpc_params;
use plonky2::plonk::config::PoseidonGoldilocksConfig;
use plonky2::{
    field::{goldilocks_field::GoldilocksField, types::PrimeField64},
    plonk::proof::ProofWithPublicInputs,
};
use qed_core::config::network_constants::REALM_USER_TREE_HEIGHT;
use qed_core::job::id::ProvingJobDataId;
use qed_core::{
    config::network_constants::GLOBAL_USER_TREE_HEIGHT,
    data::qhashout::QHashOut,
    job::{
        drain_queue::CheckpointDrainQueueEmitterAsyncImm,
        id::{ProvingJobCircuitType, ProvingJobDataType, QJobTopic, QProvingJobDataID},
        traits::QProofStoreAsyncImm,
    },
};
use qed_crypto::hash::merkle::core::MerkleProofCore;
use qed_crypto::{
    common::generic_circuit_verifier::GenericCircuitVerifier,
    hash::traits::{
        hasher::{MerkleZeroHasher, PoseidonHasher},
        qhashable::QFieldHashable,
    },
};
use qed_data::guta::api::{GUTARealmCheckpointResult, SubmitGUTARealmResultAPINoProofInput};
use qed_data::guta::{
    api::{SimpleContractHeightCache, UserEndCapNonProofCoreInputQueueItem},
    end_cap_input::SubmitUserEndCapNonProofInput,
};
use qed_data::qdata::checkpoint::{
    QEDCheckpointGlobalStateRoots, QEDCheckpointLeaf, QEDL2BlockState,
};
use qed_data::qdata::user::QEDUserLeaf;
use qed_node::nimpl::proof_store_fred::ProofStoreFred;
use qed_node::realm::state::processor::RealmConfig;
use qed_store::config::store_config::UserTreeStore;
use qed_store::models::kvq_merkle::model::KVQFixedConfigMerkleTreeModelReaderCore;
use qed_store::config::store_config::QEDFelt;
use qed_store::{
    config::store_config::QCheckpointSyncInfoCompact, node::realm::QEDRealmStoreReaderAsync,
};
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, error, info, warn};

#[derive(Clone)]
pub struct RealmEdgeContext<
    SR: QEDRealmStoreReaderAsync<F>,
    DQ: CheckpointDrainQueueEmitterAsyncImm,
    PS: QProofStoreAsyncImm,
    IQ: RealmInternalQueue,
> {
    pub store_reader: Arc<SR>,
    pub checkpoint_queue: Arc<DQ>,
    pub proof_store: Arc<PS>,
    pub proof_verifier: Arc<GenericCircuitVerifier<C, D>>,
    pub realm_config: RealmConfig,
    pub interval_sync_queue: Arc<IQ>,
}

impl<
        SR: QEDRealmStoreReaderAsync<F>,
        DQ: CheckpointDrainQueueEmitterAsyncImm,
        PS: QProofStoreAsyncImm,
        IQ: RealmInternalQueue,
    > RealmEdgeContext<SR, DQ, PS, IQ>
{
    pub async fn new(
        realm_config: RealmConfig,
        store_reader: Arc<SR>,
        checkpoint_queue: Arc<DQ>,
        proof_store: Arc<PS>,
        proof_verifier: Arc<GenericCircuitVerifier<C, D>>,
        interval_sync_queue: Arc<IQ>,
    ) -> anyhow::Result<Self> {
        Ok(Self {
            realm_config,
            store_reader: store_reader.clone(),
            checkpoint_queue,
            proof_store,
            proof_verifier,
            interval_sync_queue,
        })
    }

    pub fn includes_user_id(&self, id: u64) -> bool {
        self.realm_config.includes_user_id(id)
    }

    pub fn verify_proof_of_type(
        &self,
        circuit_type: ProvingJobCircuitType,
        proof: &ProofWithPublicInputs<F, C, D>,
    ) -> anyhow::Result<()> {
        self.proof_verifier
            .verify_proof_of_type(circuit_type, proof)
    }

    pub async fn get_checkpoint_id_async(&self) -> anyhow::Result<u64> {
        Ok(self
            .store_reader
            .get_latest_l2_block_state()
            .await?
            .checkpoint_id)
    }

    pub async fn ensure_checkpoint_hash_valid(
        &self,
        checkpoint_id: F,
        checkpoint_root_hash: QHashOut<F>,
    ) -> anyhow::Result<()> {
        let expected = self
            .store_reader
            .get_checkpoint_tree_root_f(checkpoint_id)
            .await?;
        if expected != checkpoint_root_hash {
            anyhow::bail!("invalid checkpoint_root_hash");
        }
        Ok(())
    }

    pub async fn handle_recv_end_cap_from_user(
        &self,
        input: SubmitUserEndCapNonProofInput<F>,
        proof: &ProofWithPublicInputs<F, C, D>,
    ) -> anyhow::Result<()> {
        eprintln!("DEBUGPRINT[578]: context.rs:123: input={}", serde_json::to_string_pretty(&input).unwrap());
        // start validation
        if proof.public_inputs.len() != 4 {
            anyhow::bail!("invalid proof");
        }
        if input.contract_state_updates.len() == 0 {
            anyhow::bail!("invalid contract_state_updates: cannot be empty");
        }
        let proof_public_inputs_hash: QHashOut<GoldilocksField> =
            QHashOut::from_felt_slice(&proof.public_inputs);

        let user_id_u64 = input.core.new_user_leaf.user_id.to_canonical_u64();
        if !self.includes_user_id(user_id_u64) {
            anyhow::bail!("user id {} is not in this realm", user_id_u64);
        }

        // Build contract height cache and validate
        let mut contracts_helper = SimpleContractHeightCache::<F>::new();
        for (contract_id, insecure_unvalidated_user_provided_cst_height) in
            input.get_needed_contract_zero_hashes()
        {
            let qh: QHashOut<GoldilocksField> =
                PoseidonHasher::get_zero_hash(insecure_unvalidated_user_provided_cst_height);
            contracts_helper.add_contract(
                contract_id,
                insecure_unvalidated_user_provided_cst_height as u8,
                qh,
            );
        }

        input.ensure_simple_self_consistent::<H>(proof_public_inputs_hash, &contracts_helper)?;

        let end_cap_checkpoint_id = input.core.checkpoint_id.to_canonical_u64();
        let checkpoint_id = self.get_checkpoint_id_async().await?;
        let next_checkpoint_id = checkpoint_id + 2;
        if end_cap_checkpoint_id > checkpoint_id {
            anyhow::bail!("invalid checkpoint id");
        }

        let checkpoint_tree_proof = self
            .store_reader
            .get_checkpoint_tree_merkle_proof(checkpoint_id, end_cap_checkpoint_id)
            .await?;
        if checkpoint_tree_proof.root != input.core.state_transition.checkpoint_tree_root_hash {
            anyhow::bail!("invalid checkpoint_root_hash");
        }

        let user_leaf = self
            .store_reader
            .get_user_leaf_data(checkpoint_id, user_id_u64)
            .await?;
        let expected_start_user_leaf_hash = user_leaf.qfhash::<H>();
        if expected_start_user_leaf_hash != input.core.state_transition.start_user_leaf_hash {
            anyhow::bail!("invalid start user leaf state, potentially submitted a separate end cap while proving the current one");
        }

        if user_leaf.last_checkpoint_id.to_canonical_u64()
            > input.core.checkpoint_id.to_canonical_u64()
        {
            anyhow::bail!("invalid checkpoint in proving session: cannot go backward");
        }

        if user_leaf.nonce.to_canonical_u64() > input.core.new_user_leaf.nonce.to_canonical_u64() {
            anyhow::bail!("invalid checkpoint in proving session: cannot go backward");
        }

        let old_user_state_tree_root = user_leaf.user_state_tree_root;
        debug!(
            "old_user_state_tree_root: {}",
            serde_json::to_string(&old_user_state_tree_root).unwrap()
        );

        let cst_user_update =
            input.verify_and_generate_cst_updates::<H>(next_checkpoint_id, old_user_state_tree_root)?;

        self.verify_proof_of_type(ProvingJobCircuitType::UserEndCap, proof)?;

        // end validation

        let proof_id = QProvingJobDataID::new(
            QJobTopic::GenerateStandardProof,
            checkpoint_id,
            ProvingJobCircuitType::UserEndCap.to_circuit_group_id(),
            GLOBAL_USER_TREE_HEIGHT as u32,
            user_id_u64 as u32,
            ProvingJobCircuitType::UserEndCap,
            ProvingJobDataType::OutputProof,
            0,
        );

        if self.proof_store.contains_id(proof_id).await? {
            anyhow::bail!("already submitted proof for this block");
        }

        tracing::info!("input proof_id: {:?}", proof_id);

        //self.proof_store.set_bytes_by_id(proof_id.get_input_witness_id(), data)
        self.proof_store.set_proof_by_id(proof_id, proof).await?;
        let queue_item = UserEndCapNonProofCoreInputQueueItem {
            input: input.core,
            proof_id,
            checkpoint_tree_proof,
            checkpoint_id: next_checkpoint_id,
            channel_id: self.realm_config.guta_channel_id,
        };

        tracing::info!("queue item: {:?}", queue_item);
        tracing::info!("queue item pretty: {}", serde_json::to_string_pretty(&queue_item).unwrap());

        eprintln!("DEBUGPRINT[573]: context.rs:231: cst_user_update={}", serde_json::to_string_pretty(&cst_user_update).unwrap());
        self.checkpoint_queue.cdq_push_imm(cst_user_update).await?;
        self.checkpoint_queue.cdq_push_imm(queue_item).await?;

        debug!("enqueued queue item successfully");

        Ok(())
    }

    pub async fn handle_recv_checkpoint_sync(
        &self,
        input: QCheckpointSyncInfoCompact,
    ) -> anyhow::Result<()> {
        self.checkpoint_queue.cdq_push_imm(input).await?;
        Ok(())
    }
}

#[async_trait]
impl<SR, DQ, PS, IQ> RealmEdgeRpcServer for RealmEdgeContext<SR, DQ, PS, IQ>
where
    SR: QEDRealmStoreReaderAsync<F> + Sync + Send + 'static,
    DQ: CheckpointDrainQueueEmitterAsyncImm + Sync + Send + 'static,
    PS: QProofStoreAsyncImm + Sync + Send + 'static,
    IQ: RealmInternalQueue + Sync + Send + 'static,
{
    async fn check_user_id_in_realm(&self, user_id: u64) -> RpcResult<bool> {
        Ok(self.includes_user_id(user_id))
    }

    async fn submit_user_end_cap(
        &self,
        user_ec_input: SubmitUserEndCapNonProofInput<F>,
        proof: ProofWithPublicInputs<F, C, D>,
    ) -> RpcResult<String> {
        Ok(self
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
            .store_reader
            .get_checkpoint_leaf_data(checkpoint_id.to_canonical_u64())
            .await
            .map_err(RpcError::Anyhow)?)
    }

    async fn get_latest_l2_block_state(&self) -> RpcResult<QEDL2BlockState> {
        Ok(self
            .store_reader
            .get_latest_l2_block_state()
            .await
            .map_err(RpcError::Anyhow)?)
    }

    async fn get_l2_block_state(&self, checkpoint_id: u64) -> RpcResult<QEDL2BlockState> {
        Ok(self
            .store_reader
            .get_l2_block_state(checkpoint_id)
            .await
            .map_err(RpcError::Anyhow)?)
    }

    async fn get_l2_block_state_f(&self, checkpoint_id: F) -> RpcResult<QEDL2BlockState> {
        Ok(self
            .store_reader
            .get_l2_block_state(checkpoint_id.to_canonical_u64())
            .await
            .map_err(RpcError::Anyhow)?)
    }

    async fn get_user_registration_tree_root(&self, checkpoint_id: u64) -> RpcResult<QHashOut<F>> {
        Ok(self
            .store_reader
            .get_user_registration_tree_root(checkpoint_id)
            .await
            .map_err(RpcError::Anyhow)?)
    }

    async fn get_latest_checkpoint_tree_root(&self) -> RpcResult<QHashOut<F>> {
        Ok(self
            .store_reader
            .get_latest_checkpoint_tree_root()
            .await
            .map_err(RpcError::Anyhow)?)
    }

    async fn get_checkpoint_tree_root(&self, checkpoint_id: u64) -> RpcResult<QHashOut<F>> {
        Ok(self
            .store_reader
            .get_checkpoint_tree_root(checkpoint_id)
            .await
            .map_err(RpcError::Anyhow)?)
    }

    async fn get_checkpoint_tree_root_f(&self, checkpoint_id: F) -> RpcResult<QHashOut<F>> {
        Ok(self
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
            .store_reader
            .get_user_contract_tree_merkle_proof_f(checkpoint_id, user_id, contract_id)
            .await
            .map_err(RpcError::Anyhow)?)
    }

    async fn get_user_tree_root(&self, checkpoint_id: u64) -> RpcResult<QHashOut<F>> {
        Ok(self
            .store_reader
            .get_user_tree_root(checkpoint_id)
            .await
            .map_err(RpcError::Anyhow)?)
    }

    async fn get_user_tree_root_f(&self, checkpoint_id: F) -> RpcResult<QHashOut<F>> {
        Ok(self
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
        Ok(self.store_reader
            .get_user_tree_merkle_proof(checkpoint_id, user_id)
            .await.map_err(RpcError::Anyhow)?)
        
    }
}

pub async fn spawn_realm_job_update_task(
    proof_store: Arc<ProofStoreFred>,
    realm_id: u64,
    coordinator_addr: String,
) -> anyhow::Result<()> {
    info!("realm job listener spawned");
    tokio::spawn(async move {
        loop {
            match proof_store.consume_proof().await {
                Ok(job_id) => {
                    info!(?job_id, "Received proof from realm processor");
                    // if job_id.job_id.circuit_type != GUTANoChange {
                    send_realm_proof(proof_store.clone(), job_id, realm_id, &coordinator_addr)
                        .await;
                    // }
                }
                Err(err) => {
                    error!("Error getting job_id from redis: {:?}", err);
                }
            }
            // Avoid busy waiting
            tokio::time::sleep(Duration::from_millis(500)).await;
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
        Ok(proof) => proof,
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
    while retry_count < 5 {
        info!("Sending job to coordinator, retry_count = {}", retry_count);
        match jsonrpsee::http_client::HttpClientBuilder::default().build(coordinator_addr) {
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
