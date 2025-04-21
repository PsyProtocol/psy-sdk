use std::sync::Arc;

use plonky2::{
    field::{goldilocks_field::GoldilocksField, types::PrimeField64},
    plonk::{config::PoseidonGoldilocksConfig, proof::ProofWithPublicInputs},
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
use qed_store::{
    config::store_config::{QCheckpointSyncInfoCompact, QEDFelt, QEDHasher},
    node::realm::QEDRealmStoreReaderAsync,
};
use tracing::debug;

use super::{contract_reader::ContractReader, error, realm_config::RealmConfig};

type F = QEDFelt;
type C = PoseidonGoldilocksConfig;
const D: usize = 2;
type H = QEDHasher;

#[derive(Clone)]
pub struct RealmEdgeContext<
    SR: QEDRealmStoreReaderAsync<F>,
    DQ: CheckpointDrainQueueEmitterAsyncImm,
    PS: QProofStoreAsyncImm,
> {
    pub contract_reader: ContractReader<SR>,
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
            contract_reader: ContractReader::new(store_reader),
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

    pub async fn get_contract_height(&self, contract_id: u64) -> error::Result<u8> {
        self.contract_reader.get_contract_height(contract_id).await
    }

    pub async fn get_contract_zero_hash(&self, contract_id: u64) -> error::Result<QHashOut<F>> {
        self.contract_reader
            .get_contract_zero_hash(contract_id)
            .await
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
            // todo: check if this is correct
            // Get actual contract heights from storage for validation
            // let actual_height = self.get_contract_height(contract_id).await?;
            // if actual_height as usize != insecure_unvalidated_user_provided_cst_height {
            //     anyhow::bail!(
            //         "invalid contract height for contract {}: expected {}, got {}",
            //         contract_id,
            //         actual_height,
            //         insecure_unvalidated_user_provided_cst_height
            //     );
            // }

            // todo: check if this is correct
            let qh: QHashOut<GoldilocksField> =
                PoseidonHasher::get_zero_hash(insecure_unvalidated_user_provided_cst_height);
            // let zero_hash = self.get_contract_zero_hash(contract_id).await?;
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
