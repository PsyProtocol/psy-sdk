use super::id::QProvingJobDataID;
use super::traits::QProofStoreReaderAsync;
use async_trait::async_trait;
use plonky2::plonk::{config::GenericConfig, proof::ProofWithPublicInputs};
use std::time::Duration;

#[async_trait]
pub trait WorkerEventReceiverAsyncImm {
    async fn wait_for_next_job_imm(&self) -> anyhow::Result<QProvingJobDataID>;
    async fn enqueue_jobs_imm(&self, jobs: &[QProvingJobDataID]) -> anyhow::Result<()>;
    async fn notify_core_goal_completed_imm(&self, job: QProvingJobDataID) -> anyhow::Result<()>;
}

#[async_trait]
pub trait WorkerEventTransmitterAsyncImm: QProofStoreReaderAsync {
    async fn enqueue_jobs_imm(&self, jobs: &[QProvingJobDataID]) -> anyhow::Result<()>;
    async fn wait_for_block_proving_jobs_imm(&self, checkpoint_id: u64) -> anyhow::Result<QProvingJobDataID>;

    async fn wait_for_job_proof<C: GenericConfig<D> + 'static, const D: usize>(
        &self,
        job_id: QProvingJobDataID
    ) -> anyhow::Result<ProofWithPublicInputs<C::F, C, D>>
    where
        C::Hasher: plonky2::plonk::config::AlgebraicHasher<C::F>;
}
