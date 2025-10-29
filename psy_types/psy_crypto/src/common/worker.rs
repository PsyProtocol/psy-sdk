use async_trait::async_trait;
use plonky2::plonk::{config::GenericConfig, proof::ProofWithPublicInputs};
use psy_core::job::{id::QProvingJobDataID, traits::QProofStoreReaderAsync};

use super::circuit_library::CircuitInfoLibrary;

pub trait QNextGenWorkerGenericInfo {
    fn can_process_job(&self, job_id: QProvingJobDataID) -> bool;
}
#[async_trait]
pub trait QNextGenWorkerGenericProverAsyncMut<
    S: QProofStoreReaderAsync + Send + Sync,
    L: CircuitInfoLibrary<C, D> + Send + Sync,
    C: GenericConfig<D>,
    const D: usize,
>: QNextGenWorkerGenericInfo
{
    async fn worker_prove_mut_async(&self, store: &S, library: &L, job_id: QProvingJobDataID) -> anyhow::Result<ProofWithPublicInputs<C::F, C, D>>;
}
