use std::time::Duration;

use plonky2::plonk::{circuit_data::{CommonCircuitData, VerifierOnlyCircuitData}, config::{GenericConfig, PoseidonGoldilocksConfig}, proof::ProofWithPublicInputs};
use qed_core::{data::qhashout::QHashOut, job::{id::{ProvingJobCircuitType, QJobTopic, QProvingJobDataID, QWorkerModeFilter}, mode::QWorkerMode, traits::{QProofStore, QWorkerGenericProverMut, QWorkerVerifyHelper}, worker_queue::WorkerEventReceiverSync}, utils::debug_timer::DebugTimer};
use qed_node::nimpl::{proof_store_redis::RedisStore, worker_queue_redis::{redis_queue::RedisQueue, wq_mut::QEDRedisEventProcessor}};
pub struct QEDFakeProver {
    
}

impl<
C: GenericConfig<D>,
const D: usize,
> QWorkerVerifyHelper<C, D> for QEDFakeProver {
    fn get_verifier_triplet_for_circuit_type(
        &self,
        circuit_type: ProvingJobCircuitType,
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
        store: &PS,
        job_id: QProvingJobDataID,
    ) -> anyhow::Result<ProofWithPublicInputs<C::F, C, D>> {
        todo!()
    }
}

pub struct SimpleActorWorker {}
impl SimpleActorWorker {
    pub fn run_worker<
        PS: QProofStore,
        ER: WorkerEventReceiverSync,
        G: QWorkerGenericProverMut<PS, C, D>,
        C: GenericConfig<D>,
        const D: usize,
    >(
        store: &mut PS,
        event_receiver: &mut ER,
        prover: &mut G,
    ) -> anyhow::Result<()> {
        loop {
            Self::process_next_job(store, event_receiver, prover, QWorkerMode::All)?;
        }
    }
    pub fn process_next_job<
        PS: QProofStore,
        ER: WorkerEventReceiverSync,
        G: QWorkerGenericProverMut<PS, C, D>,
        C: GenericConfig<D>,
        const D: usize,
    >(
        store: &mut PS,
        event_receiver: &mut ER,
        prover: &mut G,        
        mode: QWorkerMode,

    ) -> anyhow::Result<()> {
        //let mut timer = TraceTimer::new("process_next_job");
        let job = event_receiver.wait_for_next_job_mut()?;
        if mode.can_process_job(job) {
            //tracing::info!("job: {:?}", job);
            Self::process_job(store, event_receiver, prover, job)?;
            //timer.lap("processed next job");
        } else {
            event_receiver.enqueue_jobs_mut(&[job])?;
            std::thread::sleep(Duration::from_millis(750));
        }
        Ok(())
    }
    fn process_job<
        PS: QProofStore,
        ER: WorkerEventReceiverSync,
        G: QWorkerGenericProverMut<PS, C, D>,
        C: GenericConfig<D>,
        const D: usize,
    >(
        store: &mut PS,
        event_receiver: &mut ER,
        prover: &mut G,
        job_id: QProvingJobDataID,
    ) -> anyhow::Result<()> {
        //let mut timer = DebugTimer::new("process_job");
        if job_id.topic == QJobTopic::GenerateStandardProof {
            let start_time = std::time::Instant::now();
            /* 
            let _ = match job_id.circuit_type {
                _ => {
                    let proof = prover.worker_prove_mut(store, job_id)?;
                    let output_id = job_id.get_output_id();
                    store.set_proof_by_id(output_id, &proof)?;
                    output_id
                }
            };*/
            let duration = start_time.elapsed().as_millis() as u64;
            event_receiver.record_job_bench_mut(job_id, duration)?;
        }
        if job_id.topic == QJobTopic::NotifyOrchestratorComplete {
            event_receiver.notify_core_goal_completed_mut(job_id)?;
            return Ok(());
        }

        let goal_counter = store.get_goal_by_job_id(job_id)?;
        //tracing::info!("goal_counter: {}", goal_counter);
        if goal_counter != 0 {
            let result = store.inc_counter_by_id(job_id.get_sub_group_counter_id())?;
            if result == goal_counter {
                let jobs = store.get_next_jobs_by_job_id(job_id)?;
                //tracing::info!("[{:?}] enqueuing_jobs: {:?}", job_id, jobs);
                event_receiver.enqueue_jobs_mut(&jobs)?;
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

fn run_worker() -> anyhow::Result<()> {


    let mut timer = DebugTimer::new("prove_block_demo");
    let mut proof_store = RedisStore::new("redis://localhost:6379/0")?;
    let redis_queue = RedisQueue::new("redis://localhost:6379/0")?;
    let mut worker_event_processor = QEDRedisEventProcessor::new(redis_queue.clone());
    let mut fake_prover = QEDFakeProver{};

    const D: usize = 2;
    type C = PoseidonGoldilocksConfig;

    SimpleActorWorker::run_worker::<_,_,_, C,D>(
        &mut proof_store,
        &mut worker_event_processor,
        &mut fake_prover,
    )?;


    Ok(())
}

fn main() {

    run_worker().unwrap();

}