use std::sync::Arc;

use plonky2::{field::{goldilocks_field::GoldilocksField, types::PrimeField64}, plonk::proof::ProofWithPublicInputs};
use qed_core::{config::network_constants::GLOBAL_USER_TREE_HEIGHT, data::qhashout::QHashOut, job::{drain_queue::CheckpointDrainQueueEmitterAsyncImm, id::{ProvingJobCircuitType, ProvingJobDataType, QJobTopic, QProvingJobDataID}, traits::QProofStoreAsyncImm}};
use qed_crypto::{common::generic_circuit_verifier::GenericCircuitVerifier, hash::traits::{hasher::{MerkleZeroHasher, PoseidonHasher}, qhashable::QFieldHashable}};
use qed_data::{config::store_config::QCheckpointSyncInfoCompact, guta::{api::{SimpleContractHeightCache, UserEndCapNonProofCoreInputQueueItem}, end_cap_input::SubmitUserEndCapNonProofInput}};
use qed_store::node::realm::QEDRealmStoreReaderAsync;
use tracing::debug;
use crate::realm::{C, D, F, H};
use qed_core::job::history_queue::CheckpointHistoryQueueEmitterAsyncImm;

use super::processor::RealmConfig;

#[derive(Clone)]
pub struct RealmEdgeContext<
    SR: QEDRealmStoreReaderAsync<F> + Sync,
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
        SR: QEDRealmStoreReaderAsync<F> + Sync,
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

    pub async fn handle_recv_end_cap_from_user(
        &self,
        input: SubmitUserEndCapNonProofInput<F>,
        proof: &ProofWithPublicInputs<F, C, D>,
    ) -> anyhow::Result<()> {
        eprintln!(
            "DEBUGPRINT[578]: context.rs:123: input={}",
            serde_json::to_string_pretty(&input).unwrap()
        );
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
            anyhow::bail!(
                "user id {} is not in this realm {}",
                user_id_u64,
                self.realm_config.realm_id
            );
        }

        // Build contract height cache and validate
        tracing::info!("build contract height cache and validate");
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

        tracing::info!("ensure simple self consistent");
        input.ensure_simple_self_consistent::<H>(proof_public_inputs_hash, &contracts_helper)?;

        let end_cap_checkpoint_id = input.core.checkpoint_id.to_canonical_u64();
        let checkpoint_id = self.get_checkpoint_id_async().await?;
        let next_checkpoint_id = checkpoint_id + 1;//todo fix bug?
        if end_cap_checkpoint_id > checkpoint_id {
            tracing::info!("ensure end cap checkpoint id: {} {} {}", checkpoint_id, end_cap_checkpoint_id, next_checkpoint_id);
            anyhow::bail!("invalid checkpoint id");
        }

        tracing::info!("get_checkpoint_tree_merkle_proof, checkpoint_id={}, end_cap_checkpoint_id={}", checkpoint_id, end_cap_checkpoint_id);
        let checkpoint_tree_proof = self
            .store_reader
            .get_checkpoint_tree_merkle_proof(end_cap_checkpoint_id, end_cap_checkpoint_id)
            .await?;

        if checkpoint_tree_proof.root != input.core.state_transition.checkpoint_tree_root_hash {
            tracing::error!(
                "ensure checkpoint_tree_proof: {:?} == input.core.state_transition.checkpoint_tree_root_hash {:?}",
                checkpoint_tree_proof.root,
                input.core.state_transition.checkpoint_tree_root_hash,
            );
            anyhow::bail!("invalid checkpoint_root_hash");
        }

        tracing::info!(
            "get user{} data at checkpoint {}",
            user_id_u64,
            checkpoint_id
        );
        let user_leaf = self
            .store_reader
            .get_user_leaf_data(end_cap_checkpoint_id, user_id_u64)
            .await?;
        let expected_start_user_leaf_hash = user_leaf.qfhash::<H>();
        if expected_start_user_leaf_hash != input.core.state_transition.start_user_leaf_hash {
            tracing::error!(
                "ensure expected_start_user_leaf_hash: {:?} == input.core.state_transition.start_user_leaf_hash {:?}",
                expected_start_user_leaf_hash,
                input.core.state_transition.start_user_leaf_hash,
            );
            anyhow::bail!("invalid start user leaf state, potentially submitted a separate end cap while proving the current one");
        }

        if user_leaf.last_checkpoint_id.to_canonical_u64()
            > input.core.checkpoint_id.to_canonical_u64()
        {
            anyhow::bail!(
                "invalid checkpoint {}, expected {} in proving session: cannot go backward",
                user_leaf.last_checkpoint_id,
                input.core.checkpoint_id
            );
        }

        if user_leaf.nonce.to_canonical_u64() > input.core.new_user_leaf.nonce.to_canonical_u64() {
            anyhow::bail!(
                "invalid nonce {}, expected {} in proving session: cannot go backward",
                user_leaf.nonce.to_canonical_u64(),
                input.core.new_user_leaf.nonce.to_canonical()
            );
        }

        let old_user_state_tree_root = user_leaf.user_state_tree_root;
        debug!(
            "old_user_state_tree_root: {}",
            serde_json::to_string(&old_user_state_tree_root).unwrap()
        );

        tracing::info!(
            "verify_and_generate_cst_updates, next_checkpoint_id={}, old_user_state_tree_root={}",
            next_checkpoint_id,
            old_user_state_tree_root.to_string()
        );
        let cst_user_update = input
            .verify_and_generate_cst_updates::<H>(next_checkpoint_id, old_user_state_tree_root)?;
        tracing::info!(
            "verify UserEndCap proof, proof.public_inputs={:?}",
            proof.public_inputs
        );
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
            anyhow::bail!("already submitted proof {:?} for this block", proof_id);
        }

        tracing::info!("input proof_id: {:?}", proof_id);

        //self.proof_store.set_bytes_by_id(proof_id.get_input_witness_id(), data)
        tracing::info!(
            "set proof by id: {:?}, proof.public_inputs={:?}",
            proof_id,
            proof.public_inputs
        );
        self.proof_store.set_proof_by_id(proof_id, proof).await?;
        let queue_item = UserEndCapNonProofCoreInputQueueItem {
            input: input.core,
            proof_id,
            checkpoint_tree_proof,
            checkpoint_id: next_checkpoint_id,
            channel_id: self.realm_config.guta_channel_id,
        };

        tracing::info!(
            "queue item pretty: {}",
            serde_json::to_string_pretty(&queue_item).unwrap()
        );

        eprintln!(
            "DEBUGPRINT[573]: context.rs:231: cst_user_update={}",
            serde_json::to_string_pretty(&cst_user_update).unwrap()
        );
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
