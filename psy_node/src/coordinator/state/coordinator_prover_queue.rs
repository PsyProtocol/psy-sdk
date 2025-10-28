use super::processor::CoordinatorProcessorContext;
use psy_core::{
    data::qhashout::QHashOut,
    job::{
        history_queue::CheckpointHistoryQueueEmitterAsyncImm, id::{ProvingJobCircuitType, QProvingJobDataID}, traits::{QProofStoreAsyncImm, QProofStoreWriterSync}, worker_queue::WorkerEventTransmitterAsyncImm
    },
};
use psy_crypto::hash::merkle::{
    spiderman::SpidermanUpdateProof, treeprover::data::CircuitInputWithJobId,
};
use psy_data::config::store_config::QEDFelt;
use psy_data::{
    proof_store::builder::ProofStoreBuilder,
    protocol::circuit_inputs::{
        append_user_registration_tree::QCAppendUserRegistrationTreeCircuitInput,
        deploy_contracts::QCBatchDeployContractsCircuitInput,
    },
    qdata::contract::QEDContractLeaf,
};
use psy_store::queue::redis_queue::CheckpointDrainQueueConsumerAsyncImmWithPosition;
use psy_store::{
    node::coordinator::{
        QEDCoordinatorStoreReaderAsync, QEDCoordinatorStoreWriterAsyncImm,
    },
    queue::task_queue::QProvingTaskStore,
};
use psy_store::store::journal::Journal;

type F = QEDFelt;
impl<
        SR: QEDCoordinatorStoreWriterAsyncImm<F> + QEDCoordinatorStoreReaderAsync<F>+ Journal,
        DQ: CheckpointDrainQueueConsumerAsyncImmWithPosition,
        HQ: CheckpointHistoryQueueEmitterAsyncImm,

        WQ: WorkerEventTransmitterAsyncImm,

        PS: QProofStoreAsyncImm,
        TS: QProvingTaskStore,
    > CoordinatorProcessorContext<SR, DQ, HQ, WQ, PS, TS>
{
    pub fn push_user_registration_request(
        &self,
        checkpoint_id: u64,
        slot_id: u64,
        group_id: u32,
        task_index: u32,
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
            checkpoint_id,
            slot_id,
            group_id,
            ProvingJobCircuitType::AppendUserRegistrationTree,
            0,
            task_index,
        );
        psb.set_bytes_by_id(job_id, &bincode::serialize(&op_result)?)?;

        Ok(CircuitInputWithJobId::new(op_result, job_id))
    }
    pub fn push_deploy_contracts_request(
        &self,
        checkpoint_id: u64,
        slot_id: u64,
        group_id: u32,
        task_index: u32,
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
            checkpoint_id,
            slot_id,
            group_id,
            ProvingJobCircuitType::BatchDeployContracts,
            0,
            task_index,
        );
        psb.set_bytes_by_id(job_id, &bincode::serialize(&op_result)?)?;

        Ok(CircuitInputWithJobId::new(op_result, job_id))
    }
}
