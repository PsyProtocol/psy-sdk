use std::sync::Arc;

use anyhow::ensure;
use plonky2::{field::{goldilocks_field::GoldilocksField, types::{Field, PrimeField64}}, plonk::proof::ProofWithPublicInputs};
use qed_core::{config::network_constants::GLOBAL_USER_TREE_HEIGHT, data::qhashout::QHashOut, job::{drain_queue::CheckpointDrainQueueEmitterAsyncImm, id::{ProvingJobCircuitType, ProvingJobDataType, QJobTopic, QProvingJobDataID}, traits::QProofStoreAsyncImm}};
use qed_crypto::{common::generic_circuit_verifier::GenericCircuitVerifier, hash::traits::{hasher::{MerkleZeroHasher, PoseidonHasher}, qhashable::QFieldHashable}};
use qed_data::{config::store_config::{QCheckpointSyncInfoCompact, QEDHasher}, guta::{api::{SimpleContractHeightCache, UserEndCapNonProofCoreInputQueueItem}, end_cap_input::SubmitUserEndCapNonProofInput}};
use qed_prover::session::TxStatus;
use qed_store::node::realm::QEDRealmStoreReaderAsync;
use tracing::debug;
use crate::realm::{C, D, F, H};
use qed_core::job::history_queue::CheckpointHistoryQueueEmitterAsyncImm;
use qed_crypto::hash::traits::hasher::FieldQHasher;
use qed_crypto::hash::merkle::core::compute_historical_and_current_merkle_roots_core_gt;

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
        debug!("Processing user end cap input: {}", serde_json::to_string_pretty(&input).unwrap());
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
            .get_checkpoint_tree_merkle_proof(checkpoint_id, end_cap_checkpoint_id)
            .await?;
        let (historical_root, current_root) = compute_historical_and_current_merkle_roots_core_gt::<QHashOut<F>, QEDHasher>(&checkpoint_tree_proof);
        ensure!(current_root == checkpoint_tree_proof.root);
        ensure!(
            historical_root == input.core.state_transition.checkpoint_tree_root_hash,
            "user endcap checkpoint root {} != checkpoint tree proof's historical root {}",
            input.core.state_transition.checkpoint_tree_root_hash,
            historical_root
        );

        if checkpoint_tree_proof.root != input.core.state_transition.checkpoint_tree_root_hash {
            tracing::warn!(
                "ensure checkpoint_tree_proof: {} == input.core.state_transition.checkpoint_tree_root_hash {}",
                checkpoint_tree_proof.root,
                input.core.state_transition.checkpoint_tree_root_hash,
            );
            // anyhow::bail!("invalid checkpoint_root_hash");
        }

        tracing::info!(
            "get user{} data at checkpoint {}",
            user_id_u64,
            checkpoint_id
        );
        let user_leaf = self
            .store_reader
            .get_user_leaf_data(checkpoint_id, user_id_u64)
            .await?;
        let expected_start_user_leaf_hash = user_leaf.qfhash::<H>();
        if expected_start_user_leaf_hash != input.core.state_transition.start_user_leaf_hash {
            tracing::error!(
                "ensure expected_start_user_leaf_hash: {} == input.core.state_transition.start_user_leaf_hash {}",
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
        ensure!(
            input.core.new_user_leaf.nonce == user_leaf.nonce + F::ONE,
            "user endcap nonce {} must be onchain nonce {} + 1",
            input.core.new_user_leaf.nonce,
            user_leaf.nonce
        );

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

        // check new leaf hash
        let computed_new_leaf_hash = input.core.new_user_leaf.qfhash::<H>();
        if computed_new_leaf_hash != input.core.state_transition.end_user_leaf_hash {
            tracing::error!(
                "ensure computed_new_leaf_hash: {} == state_transition.end_user_leaf_hash {}",
                computed_new_leaf_hash,
                input.core.state_transition.end_user_leaf_hash,
            );
            anyhow::bail!("invalid new user leaf hash");
        }

        // verify witness
        let expected_proof_public_inputs_hash = input.core.get_proof_public_inputs_hash::<QEDHasher>();
        if expected_proof_public_inputs_hash != proof_public_inputs_hash {
            tracing::error!(
                "ensure expected_proof_public_inputs_hash: {} == proof.public_inputs {}",
                expected_proof_public_inputs_hash,
                proof_public_inputs_hash,
            );
            anyhow::bail!("invalid user endcap proof public inputs hash");
        }

        // end validation

        let proof_id = QProvingJobDataID::new(
            QJobTopic::GenerateStandardProof,
            u64::MAX,
            0,
            self.realm_config.realm_id,
            user_id_u64 as u32,
            input.core.new_user_leaf.nonce.to_canonical_u64() as u32,
            ProvingJobCircuitType::UserEndCap,
            ProvingJobDataType::OutputProof,
            0,
        );

        if self.proof_store.contains_id(proof_id).await? {
            tracing::warn!("already submitted proof {:?} for this block", proof_id);
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

        debug!("Enqueuing contract state tree update for user {}", cst_user_update.user_id);
        self.checkpoint_queue.cdq_push_imm(cst_user_update).await?;
        self.checkpoint_queue.cdq_push_imm(queue_item).await?;

        debug!("enqueued queue item successfully");

        Ok(())
    }

    pub async fn get_tx_status(
        &self,
        user_id: u64,
        nonce: u64,
    ) -> anyhow::Result<TxStatus> {
        let latest_checkpoint_id = self.store_reader.get_latest_l2_block_state().await?.checkpoint_id;
        let onchain_nonce = self
            .store_reader
            .get_user_leaf_data(latest_checkpoint_id, user_id)
            .await?
            .nonce
            .to_canonical_u64();
        tracing::debug!("get user {} tx status at nonce {}, onchain_nonce {}", user_id, nonce, onchain_nonce);

        let proof_id = QProvingJobDataID::new(
            QJobTopic::GenerateStandardProof,
            u64::MAX,
            0,
            self.realm_config.realm_id,
            user_id as u32,
            (onchain_nonce + 1) as u32,
            ProvingJobCircuitType::UserEndCap,
            ProvingJobDataType::OutputProof,
            0,
        );

        if nonce != onchain_nonce + 1 {
            tracing::warn!("nonce {} != onchain_nonce {}", nonce, onchain_nonce);
            Ok(TxStatus::Confirmed)
        } else if self.proof_store.contains_id(proof_id).await? {
            Ok(TxStatus::Pending)
        } else {
            Ok(TxStatus::Submittable)
        } 

    }

    pub async fn handle_recv_checkpoint_sync(
        &self,
        input: QCheckpointSyncInfoCompact,
    ) -> anyhow::Result<()> {
        self.checkpoint_queue.cdq_push_imm(input).await?;
        Ok(())
    }
}
