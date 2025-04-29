use crate::error::RpcError;
use crate::rpc::RealmEdgeRpcServer;
use crate::{C, D, F, H};
use async_trait::async_trait;
use jsonrpsee::core::RpcResult;
use plonky2::{
    field::{goldilocks_field::GoldilocksField, types::PrimeField64},
    plonk::proof::ProofWithPublicInputs,
};
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
use qed_data::guta::{
    api::{SimpleContractHeightCache, UserEndCapNonProofCoreInputQueueItem},
    end_cap_input::SubmitUserEndCapNonProofInput,
};
use qed_data::qdata::checkpoint::{
    QEDCheckpointGlobalStateRoots, QEDCheckpointLeaf, QEDL2BlockState,
};
use qed_data::qdata::user::QEDUserLeaf;
use qed_node::realm::state::processor::RealmConfig;
use qed_store::{
    config::store_config::QCheckpointSyncInfoCompact, node::realm::QEDRealmStoreReaderAsync,
};
use std::sync::Arc;
use tracing::debug;

use super::request::QSubmitEndCapRPCRequest;

#[derive(Clone)]
pub struct RealmEdgeContext<
    SR: QEDRealmStoreReaderAsync<F>,
    DQ: CheckpointDrainQueueEmitterAsyncImm,
    PS: QProofStoreAsyncImm,
> {
    pub store_reader: Arc<SR>,
    pub checkpoint_queue: Arc<DQ>,
    pub proof_store: Arc<PS>,
    pub proof_verifier: Arc<GenericCircuitVerifier<C, D>>,
    pub realm_config: RealmConfig,
}

impl<
        SR: QEDRealmStoreReaderAsync<F>,
        DQ: CheckpointDrainQueueEmitterAsyncImm,
        PS: QProofStoreAsyncImm,
    > RealmEdgeContext<SR, DQ, PS>
{
    pub async fn new(
        realm_config: RealmConfig,
        store_reader: Arc<SR>,
        checkpoint_queue: Arc<DQ>,
        proof_store: Arc<PS>,
        proof_verifier: Arc<GenericCircuitVerifier<C, D>>,
    ) -> anyhow::Result<Self> {
        Ok(Self {
            realm_config,
            store_reader: store_reader.clone(),
            checkpoint_queue,
            proof_store,
            proof_verifier,
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
            input.verify_and_generate_cst_updates::<H>(checkpoint_id, old_user_state_tree_root)?;

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

        debug!("input proof_id: {:?}", proof_id);

        let next_checkpoint_id = checkpoint_id + 1;
        //self.proof_store.set_bytes_by_id(proof_id.get_input_witness_id(), data)
        self.proof_store.set_proof_by_id(proof_id, proof).await?;
        let queue_item = UserEndCapNonProofCoreInputQueueItem {
            input: input.core,
            proof_id,
            checkpoint_tree_proof,
            checkpoint_id: next_checkpoint_id,
            channel_id: self.realm_config.guta_channel_id,
        };

        self.checkpoint_queue.cdq_push_imm(cst_user_update).await?;
        self.checkpoint_queue.cdq_push_imm(queue_item).await?;

        debug!("enqueued queue item");

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
impl<SR, DQ, PS> RealmEdgeRpcServer for RealmEdgeContext<SR, DQ, PS>
where
    SR: QEDRealmStoreReaderAsync<F> + Sync + Send + 'static,
    DQ: CheckpointDrainQueueEmitterAsyncImm + Sync + Send + 'static,
    PS: QProofStoreAsyncImm + Sync + Send + 'static,
{
    async fn check_user_id_in_realm(&self, user_id: u64) -> RpcResult<bool> {
        Ok(self.includes_user_id(user_id))
    }

    async fn submit_user_end_cap(
        &self,
        user_ec_input: SubmitUserEndCapNonProofInput<F>,
        proof: ProofWithPublicInputs<F, C, D>,
    ) -> RpcResult<bool> {
        Ok(self
            .handle_recv_end_cap_from_user(user_ec_input, &proof)
            .await
            .map(|_| true)
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
}
