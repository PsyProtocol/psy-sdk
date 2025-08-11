use qed_core::{
    data::qhashout::QHashOut,
    job::{
        drain_queue::CheckpointDrainQueueConsumerAsyncImm, history_queue::CheckpointHistoryQueueEmitterAsyncImm, id::{ProvingJobCircuitType, QProvingJobDataID}, traits::{QProofStore, QProofStoreAsyncImm, QProofStoreReaderSync, QProofStoreWriterSync}, worker_queue::WorkerEventTransmitterAsyncImm
    },
};
use qed_crypto::hash::merkle::{
    spiderman::SpidermanUpdateProof, treeprover::data::CircuitInputWithJobId,
};
use qed_data::{
    proof_store::builder::ProofStoreBuilder,
    protocol::circuit_inputs::{
        append_user_registration_tree::QCAppendUserRegistrationTreeCircuitInput,
        deploy_contracts::QCBatchDeployContractsCircuitInput,
    },
    qdata::contract::QEDContractLeaf,
};
use qed_data::config::store_config::QEDFelt;
use qed_store::{
    node::coordinator::{
        QEDCoordinatorStoreReaderAsync, QEDCoordinatorStoreWriterAsyncImm,
    },
    queue::task_queue::JobTaskStore,
};

use super::processor::CoordinatorProcessorContext;

type F = QEDFelt;
impl<
        SR: QEDCoordinatorStoreWriterAsyncImm<F> + QEDCoordinatorStoreReaderAsync<F>,
        DQ: CheckpointDrainQueueConsumerAsyncImm,
        HQ: CheckpointHistoryQueueEmitterAsyncImm,

        WQ: WorkerEventTransmitterAsyncImm,

        PS: QProofStoreAsyncImm,
        JTS: JobTaskStore,
    > CoordinatorProcessorContext<SR, DQ, HQ, WQ, PS, JTS>
{
    pub fn push_user_registration_request(
        &self,
        task_index: u32,
        checkpoint_id: u64,
        psb: &mut ProofStoreBuilder,
        update_proofs: Vec<SpidermanUpdateProof<QHashOut<F>>>,
    ) -> anyhow::Result<CircuitInputWithJobId<QCAppendUserRegistrationTreeCircuitInput<F>>> {
        let op_result = QCAppendUserRegistrationTreeCircuitInput {
            register_users_circuit_whitelist: self
                .coordinator_config
                .register_users_circuit_whitelist,
            spiderman_append_proofs: update_proofs,
        };
        let job_id = QProvingJobDataID::core_op_witness(
            ProvingJobCircuitType::AppendUserRegistrationTree,
            checkpoint_id,
            task_index,
        );
        psb.set_bytes_by_id(job_id, &bincode::serialize(&op_result)?)?;

        Ok(CircuitInputWithJobId::new(op_result, job_id))
    }
    pub fn push_deploy_contracts_request(
        &self,
        task_index: u32,
        checkpoint_id: u64,
        psb: &mut ProofStoreBuilder,
        update_proof: SpidermanUpdateProof<QHashOut<F>>,
        contract_leaves: Vec<QEDContractLeaf<F>>,
    ) -> anyhow::Result<CircuitInputWithJobId<QCBatchDeployContractsCircuitInput<F>>> {
        let op_result = QCBatchDeployContractsCircuitInput {
            deploy_contract_circuit_whitelist: self
                .coordinator_config
                .deploy_contracts_circuit_whitelist,
            spiderman_append_proof: update_proof,
            contract_leaves,
        };
        let job_id = QProvingJobDataID::core_op_witness(
            ProvingJobCircuitType::BatchDeployContracts,
            checkpoint_id,
            task_index,
        );
        psb.set_bytes_by_id(job_id, &bincode::serialize(&op_result)?)?;

        Ok(CircuitInputWithJobId::new(op_result, job_id))
    }
}
