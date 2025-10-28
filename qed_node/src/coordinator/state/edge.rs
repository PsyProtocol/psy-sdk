use std::sync::Arc;

use plonky2::plonk::{config::PoseidonGoldilocksConfig, proof::ProofWithPublicInputs};
use psy_core::job::{
    drain_queue::{CheckpointDrainQueueEmitterAsyncImm, WithDrainQueueMetadata}, id::ProvingJobCircuitType,
    traits::QProofStoreAsyncImm,
};
use psy_crypto::{common::generic_circuit_verifier::GenericCircuitVerifier, signature::zk::data::ZKPublicKeyInfo};
use psy_data::{guta::api::SubmitGUTARealmResultAPINoProofInput, qblock::cmds::deploy_contract::QBCDeployContract};
use psy_data::config::store_config::{QEDFelt, QEDHasher};
use qed_store::node::coordinator::QEDCoordinatorStoreReaderAsync;
use rand::{thread_rng, RngCore};

use super::processor::CoordinatorConfig;

type F = QEDFelt;
type C = PoseidonGoldilocksConfig;
const D: usize = 2;
#[derive(Clone)]
pub struct CoordinatorEdgeContext<
    SR: QEDCoordinatorStoreReaderAsync<F>,
    DQ: CheckpointDrainQueueEmitterAsyncImm,
    PS: QProofStoreAsyncImm,
> {
    pub store_reader: Arc<SR>,
    pub checkpoint_queue: Arc<DQ>,
    pub proof_store: Arc<PS>,
    pub proof_verifier: Arc<GenericCircuitVerifier<C, D>>,

    pub coordinator_config: CoordinatorConfig,
    pub last_chkpnt_id: u64,

    //pub end_cap_verifier_data: VerifierOnlyCircuitData<C, D>,
}

impl<
        SR: QEDCoordinatorStoreReaderAsync<F>,
        DQ: CheckpointDrainQueueEmitterAsyncImm,
        PS: QProofStoreAsyncImm,
    > CoordinatorEdgeContext<SR, DQ, PS>
{
    pub async fn new(
        coordinator_config: CoordinatorConfig,
        store_reader: Arc<SR>,
        checkpoint_queue: Arc<DQ>,
        proof_store: Arc<PS>,
        proof_verifier: Arc<GenericCircuitVerifier<C, D>>,
    ) -> anyhow::Result<Self> {
        let latest = store_reader.get_latest_l2_block_state().await?;
        Ok(Self {
            coordinator_config,
            store_reader,
            checkpoint_queue,
            proof_store,
            proof_verifier,
           last_chkpnt_id: latest.checkpoint_id,
        })
    }


    pub fn verify_proof_of_type(
        &self,
        circuit_type: ProvingJobCircuitType,
        proof: &ProofWithPublicInputs<F, C, D>,
    ) -> anyhow::Result<()> {
        self.proof_verifier
            .verify_proof_of_type(circuit_type, proof)
    }
    pub async fn get_last_checkpoint_id_async(&self) -> anyhow::Result<u64> {
        Ok(self.last_chkpnt_id)
    }
    pub async fn get_next_checkpoint_id_async(&self) -> anyhow::Result<u64> {
        Ok(self.last_chkpnt_id+1)
    }

    pub async fn handle_process_regsiter_user(&self, zk_user_info: ZKPublicKeyInfo<F>) -> anyhow::Result<()> {
        self.checkpoint_queue.cdq_push_imm(zk_user_info).await?;
        Ok(())
    }

    pub async fn handle_deploy_contract(&self, contract_data: QBCDeployContract<F>) -> anyhow::Result<()> {
        let checkpoint_id = self.get_next_checkpoint_id_async().await?;
        let with_root = contract_data.into_with_whitelist_root::<QEDHasher>()?;

        let cd_for_queue = WithDrainQueueMetadata::new_params(self.coordinator_config.deploy_contract_channel_id, checkpoint_id, thread_rng().next_u64(), with_root);

        self.checkpoint_queue.cdq_push_imm(cd_for_queue).await?;
        Ok(())
    }

    pub async fn handle_recv_guta_from_realm(
        &self,
        input: SubmitGUTARealmResultAPINoProofInput<F>,
        proof: &ProofWithPublicInputs<F, C, D>,
    ) -> anyhow::Result<()> {
        if !input.top_line_proof.verify::<QEDHasher>() {
            anyhow::bail!("invalid top line proof from realm");
        }

        if input.top_line_proof.new_root != input.top_line_proof.new_value {
            anyhow::bail!("top line not currently supported for guta proofs");
        }
        self.verify_proof_of_type(input.proof_id.circuit_type, proof)?;

        let old_value = self
            .store_reader
            .get_user_latest_top_tree_cap_root(self.coordinator_config.realm_root_level, input.realm_id)
            .await?;
        if old_value != input.top_line_proof.old_root &&(old_value != input.top_line_proof.new_root) {
            // anyhow::bail!("invalid top line proof old value from realm");
            tracing::warn!("invalid top line proof old value from realm");
        }
        //let checkpoint_id = input.checkpoint_id;
        let queue_item = input.to_queue_item(
            self.coordinator_config.guta_channel_id,
            self.coordinator_config.realm_root_level as u32,
        );
        let proof_id = queue_item.proof_id;
        //let input_id = proof_id.get_input_witness_id();
        self.proof_store.set_proof_by_id(proof_id, proof).await?;
        /*self.proof_store.set_bytes_by_id(input_id, &bincode::serialize(&queue_item).map_err(|e| anyhow::anyhow!("{:?}",e))?).await?;

        let d = WithDrainQueueMetadata::<QProvingJobDataID>::new_params(
            self.guta_channel_id,
            checkpoint_id,
            queue_item.realm_id,
            input_id,
        );*/

        self.checkpoint_queue.cdq_push_imm(queue_item).await?;

        Ok(())
    }
}
