use fred::prelude::*;
use psy_core::{
    job::{id::QProvingJobDataID, traits::{QProofStoreAsyncImm, QProofStoreReaderAsync, QWorkerGenericProverAsyncMut}, worker_queue::WorkerEventReceiverAsyncImm},
    utils::debug_timer::DebugTimer,
};
use psy_store::queue::ProofStoreFred;
use std::time::Duration;

use async_trait::async_trait;

use plonky2::plonk::{circuit_data::{CommonCircuitData, VerifierOnlyCircuitData}, config::{GenericConfig, PoseidonGoldilocksConfig}, proof::ProofWithPublicInputs};
use psy_core::{data::qhashout::QHashOut, job::{id::{ProvingJobCircuitType, QJobTopic, QWorkerModeFilter}, mode::QWorkerMode, traits::{QProofStore, QWorkerGenericProverMut, QWorkerVerifyHelper}}};
use psy_store::queue::new_fred_pool;

#[derive(Debug, Clone)]
pub struct QEDFakeProver {
    pub x: u32,
}

impl<
C: GenericConfig<D>,
const D: usize,
> QWorkerVerifyHelper<C, D> for QEDFakeProver {
    fn get_verifier_triplet_for_circuit_type(
        &self,
        _circuit_type: ProvingJobCircuitType,
    ) -> (
        &CommonCircuitData<C::F, D>,
        &VerifierOnlyCircuitData<C, D>,
        QHashOut<C::F>,
    ) {
        todo!()
    }
}
impl<PS: QProofStore,
C: GenericConfig<D>,
const D: usize,
>QWorkerGenericProverMut<PS, C, D> for QEDFakeProver {
    fn worker_prove_mut(
        &mut self,
        _store: &PS,
        _job_id: QProvingJobDataID,
    ) -> anyhow::Result<ProofWithPublicInputs<C::F, C, D>> {
        todo!()
    }
}

#[async_trait]
impl<PS: QProofStoreReaderAsync,
C: GenericConfig<D>,
const D: usize,
>QWorkerGenericProverAsyncMut<PS, C, D> for QEDFakeProver {
    async fn worker_prove_mut(
        &mut self,
        _store: &PS,
        _job_id: QProvingJobDataID,
    ) -> anyhow::Result<ProofWithPublicInputs<C::F, C, D>> {
        todo!()
    }
}

pub struct SimpleActorWorker {}
impl SimpleActorWorker {
    pub async fn run_worker<
        PS: QProofStoreAsyncImm + Send + Sync,
        ER: WorkerEventReceiverAsyncImm,
        G: QWorkerGenericProverAsyncMut<PS, C, D>,
        C: GenericConfig<D>,
        const D: usize,
    >(
        store: &PS,
        event_receiver: &ER,
        prover: &mut G,
    ) -> anyhow::Result<()> {
        let mut start_job = 0u64;
        let mut timer = DebugTimer::new("process_next_job");

        loop {

            Self::process_next_job(store, event_receiver, prover, QWorkerMode::All).await?;
            start_job+=1;
            if start_job % 1000 == 0 {
                timer.lap("processed 1k jobs");
            }
        }
    }
    pub async fn process_next_job<
        PS: QProofStoreAsyncImm + Send + Sync,
        ER: WorkerEventReceiverAsyncImm,
        G: QWorkerGenericProverAsyncMut<PS, C, D>,
        C: GenericConfig<D>,
        const D: usize,
    >(
        store: &PS,
        event_receiver: &ER,
        prover: &mut G,
        mode: QWorkerMode,

    ) -> anyhow::Result<()> {
        let job = event_receiver.wait_for_next_job_imm().await?;
        if mode.can_process_job(job) {
            //println!("job: {:?}", job);
            Self::process_job(store, event_receiver, prover, job).await?;
            //timer.lap("processed next job");
        } else {
            println!("cannot process");
            event_receiver.enqueue_jobs_imm(&[job]).await?;
            std::thread::sleep(Duration::from_millis(750));
        }
        Ok(())
    }
    async fn process_job<
        PS: QProofStoreAsyncImm + Send + Sync,
        ER: WorkerEventReceiverAsyncImm,
        G: QWorkerGenericProverAsyncMut<PS, C, D>,
        C: GenericConfig<D>,
        const D: usize,
    >(
        store: &PS,
        event_receiver: &ER,
        _prover: &mut G,
        job_id: QProvingJobDataID,
    ) -> anyhow::Result<()> {
        //let mut timer = DebugTimer::new("process_job");
        if job_id.topic == QJobTopic::GenerateStandardProof {
            //let start_time = std::time::Instant::now();
            /*
            let _ = match job_id.circuit_type {
                _ => {
                    let proof = prover.worker_prove_mut(store, job_id)?;
                    let output_id = job_id.get_output_id();
                    store.set_proof_by_id(output_id, &proof)?;
                    output_id
                }
            };*/
            //let duration = start_time.elapsed().as_millis() as u64;
            //event_receiver.record_job_bench_mut(job_id, duration)?;
        }
        if job_id.is_notify_complete() {
            event_receiver.notify_core_goal_completed_imm(job_id).await?;
            return Ok(());
        }
        tracing::info!("processing job {:?}, to get goal by id", job_id.topic);
        let goal_counter = store.get_goal_by_job_id(job_id).await?;
        tracing::info!("goal_counter: {}", goal_counter);
        if goal_counter != 0 {
            let result = store.inc_counter_by_id(job_id.get_sub_group_counter_id()).await?;
            tracing::info!("result: {}", result);
            if result == goal_counter {
                tracing::info!("job {:?} is done, enqueueing jobs", job_id.topic);
                let jobs = store.get_next_jobs_by_job_id(job_id).await?;
                tracing::info!("jobs: {:?}", jobs);
                //tracing::info!("[{:?}] enqueuing_jobs: {:?}", job_id, jobs);
                event_receiver.enqueue_jobs_imm(&jobs).await?;
            }
        }
        /*timer.event(format!(
            "processed job {} ({:?})",
            hex::encode(job_id.to_fixed_bytes()),
            job_id
        ));*/

        Ok(())
    }
}


async fn run_fred_test3() -> anyhow::Result<()> {
    let mut timer = DebugTimer::new("dq_rust_2v2");
    timer.lap("start");

    let pool = new_fred_pool("redis://127.0.0.1:6379",8).await?;

    timer.lap("connected to redis");

    let q = ProofStoreFred::new(pool, "wq1".to_string());

    //let worker_count = 16usize;
    //let items_per_worker = 2000usize;



    timer.lap("started up");
    let mut fake_prover = QEDFakeProver{x: 1};

    const D: usize = 2;
    type C = PoseidonGoldilocksConfig;

    SimpleActorWorker::run_worker::<_,_,_, C,D>(
        &q,
        &q,
        &mut fake_prover,
    ).await?;
    timer.lap("finished jobs");


    Ok(())
}
#[tokio::main]
async fn main() {
    run_fred_test3().await.unwrap();
}
