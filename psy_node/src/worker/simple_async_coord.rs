use std::time::Duration;

use plonky2::plonk::config::GenericConfig;
use psy_core::{
    job::{
        self,
        id::{ProvingJobCircuitType, QJobTopic, QProvingJobDataID, QWorkerModeFilter},
        mode::QWorkerMode,
        traits::QProofStoreAsyncImm,
        worker_queue::WorkerEventReceiverAsyncImm,
    },
    utils::trace_timer::TraceTimer,
};
use psy_crypto::common::{circuit_library::CircuitInfoLibrary, worker::QNextGenWorkerGenericProverAsyncMut};
use tracing::debug;

#[derive(Clone)]
pub struct SimpleAsyncCoordinatorWorker {}
impl SimpleAsyncCoordinatorWorker {
    pub async fn run_worker<
        PS: QProofStoreAsyncImm + Send + Sync,
        ER: WorkerEventReceiverAsyncImm,
        L: CircuitInfoLibrary<C, D> + Send + Sync,
        G: QNextGenWorkerGenericProverAsyncMut<PS, L, C, D>,
        C: GenericConfig<D> + 'static,
        const D: usize,
    >(
        store: &PS,
        event_receiver: &ER,
        prover: &G,
        library: &L,
    ) -> anyhow::Result<()> {
        loop {
            Self::process_next_job(store, event_receiver, prover, library, QWorkerMode::All).await?;
        }
    }
    pub async fn run_worker_until_done<
        PS: QProofStoreAsyncImm + Send + Sync,
        ER: WorkerEventReceiverAsyncImm,
        L: CircuitInfoLibrary<C, D> + Send + Sync,
        G: QNextGenWorkerGenericProverAsyncMut<PS, L, C, D>,
        C: GenericConfig<D> + 'static,
        const D: usize,
    >(
        store: &PS,
        event_receiver: &ER,
        prover: &G,
        library: &L,
    ) -> anyhow::Result<QProvingJobDataID> {
        let mut job = Self::process_next_job(store, event_receiver, prover, library, QWorkerMode::All).await?;

        while job.circuit_type != ProvingJobCircuitType::GenerateRollupStateTransitionProof && job.topic != QJobTopic::NotifyCoordinatorComplete {
            job = Self::process_next_job(store, event_receiver, prover, library, QWorkerMode::All).await?;
        }

        Ok(job)
    }
    pub async fn process_next_job<
        PS: QProofStoreAsyncImm + Send + Sync,
        ER: WorkerEventReceiverAsyncImm,
        L: CircuitInfoLibrary<C, D> + Send + Sync,
        G: QNextGenWorkerGenericProverAsyncMut<PS, L, C, D>,
        C: GenericConfig<D> + 'static,
        const D: usize,
    >(
        store: &PS,
        event_receiver: &ER,
        prover: &G,
        library: &L,
        mode: QWorkerMode,
    ) -> anyhow::Result<QProvingJobDataID> {
        //let mut timer = TraceTimer::new("process_next_job");
        let job = event_receiver.wait_for_next_job_imm().await?;
        if mode.can_process_job(job) {
            debug!("Processing job: {:?}", job);
            return Self::process_job(store, event_receiver, prover, library, job).await;
            //timer.lap("processed next job");
        } else {
            event_receiver.enqueue_jobs_imm(&[job]).await?;
            tokio::time::sleep(Duration::from_millis(750)).await;
        }
        Ok(job)
    }
    async fn process_job<
        PS: QProofStoreAsyncImm + Send + Sync,
        ER: WorkerEventReceiverAsyncImm,
        L: CircuitInfoLibrary<C, D> + Send + Sync,
        G: QNextGenWorkerGenericProverAsyncMut<PS, L, C, D>,
        C: GenericConfig<D> + 'static,
        const D: usize,
    >(
        store: &PS,
        event_receiver: &ER,
        prover: &G,
        library: &L,
        job_id: QProvingJobDataID,
    ) -> anyhow::Result<QProvingJobDataID> {
        let mut timer = TraceTimer::new("process_job");
        timer.event(format!("STARTED job {} ({:?})", hex::encode(job_id.to_fixed_bytes()), job_id));
        if job_id.is_notify_complete() {
            event_receiver.notify_core_goal_completed_imm(job_id).await?;
            return Ok(job_id);
        }

        if job_id.topic == QJobTopic::GenerateStandardProof {
            //let start_time = std::time::Instant::now();
            let _ = match job_id.circuit_type {
                ProvingJobCircuitType::WrapFinalSigHashProofBLS12381 => {
                    todo!("impl bls12381");
                }
                _ => {
                    let proof = prover.worker_prove_mut_async(&store, library, job_id).await?;

                    let output_id = job_id.get_output_id();

                    store.set_proof_by_id(output_id, &proof).await?;

                    output_id
                }
            };
            //let duration = start_time.elapsed().as_millis() as u64;
            //event_receiver.record_job_bench(job_id, duration)?;
        }

        let goal_counter = store.get_goal_by_job_id(job_id).await?;
        debug!("Goal counter: {}", goal_counter);
        if goal_counter != 0 {
            let result = store.inc_counter_by_id(job_id.get_sub_group_counter_id()).await?;
            if result == goal_counter {
                let jobs = store.get_next_jobs_by_job_id(job_id).await?;
                println!("[{:?}] enqueuing_jobs: {:?}", job_id, jobs);
                event_receiver.enqueue_jobs_imm(&jobs).await?;
            }
        }
        timer.event(format!("FINISHED job {} ({:?})", hex::encode(job_id.to_fixed_bytes()), job_id));

        Ok(job_id)
    }
}
