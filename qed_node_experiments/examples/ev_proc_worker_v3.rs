
use lapin::{Connection, ConnectionProperties};
use qed_core::{job::{id::QProvingJobDataID, traits::QProofStoreWriterSync, worker_queue::{WorkerEventReceiverAsync, WorkerEventTransmitterAsync}}, utils::debug_timer::DebugTimer};
use qed_node::nimpl::{proof_store_redis::RedisStore, worker_queue_rabbit::{rabbit_mq_queue::RabbitMQQueue, wq_mut::QEDRabbitMQEventProcessor}, worker_queue_rabbit_stream::{rabbit_stream_queue::RabbitStreamQueue, wq_mut::QEDRabbitStreamEventProcessor}, worker_queue_redis::redis_queue::RedisQueue};

use std::{fmt::Debug, time::Duration};

use plonky2::plonk::{circuit_data::{CommonCircuitData, VerifierOnlyCircuitData}, config::{GenericConfig, PoseidonGoldilocksConfig}, proof::ProofWithPublicInputs};
use qed_core::{data::qhashout::QHashOut, job::{id::{ProvingJobCircuitType, QJobTopic, QWorkerModeFilter}, mode::QWorkerMode, traits::{QProofStore, QWorkerGenericProverMut, QWorkerVerifyHelper}, worker_queue::WorkerEventReceiverSync}};
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
    pub async fn run_worker<
        PS: QProofStore,
        ER: WorkerEventReceiverAsync,
        G: QWorkerGenericProverMut<PS, C, D>,
        C: GenericConfig<D>,
        const D: usize,
    >(
        store: &mut PS,
        event_receiver: &mut ER,
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
        PS: QProofStore,
        ER: WorkerEventReceiverAsync,
        G: QWorkerGenericProverMut<PS, C, D>,
        C: GenericConfig<D>,
        const D: usize,
    >(
        store: &mut PS,
        event_receiver: &mut ER,
        prover: &mut G,        
        mode: QWorkerMode,

    ) -> anyhow::Result<()> {
        let job = event_receiver.wait_for_next_job_mut().await?;
        if mode.can_process_job(job) {
            //println!("job: {:?}", job);
            Self::process_job(store, event_receiver, prover, job).await?;
            //timer.lap("processed next job");
        } else {
            println!("cannot process");
            event_receiver.enqueue_jobs_mut(&[job]).await?;
            std::thread::sleep(Duration::from_millis(750));
        }
        Ok(())
    }
    async fn process_job<
        PS: QProofStore,
        ER: WorkerEventReceiverAsync,
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
            event_receiver.notify_core_goal_completed_mut(job_id).await?;
            return Ok(());
        }

        let goal_counter = store.get_goal_by_job_id(job_id)?;
        //tracing::info!("goal_counter: {}", goal_counter);
        if goal_counter != 0 {
            let result = store.inc_counter_by_id(job_id.get_sub_group_counter_id())?;
            if result == goal_counter {
                let jobs = store.get_next_jobs_by_job_id(job_id)?;
                //tracing::info!("[{:?}] enqueuing_jobs: {:?}", job_id, jobs);
                event_receiver.enqueue_jobs_mut(&jobs).await?;
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


#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {


    let mut timer = DebugTimer::new("prove_block_demo");
    let mut proof_store = RedisStore::new("redis://localhost:6379/0")?;
    let redis_queue = RedisQueue::new("redis://localhost:6379/0")?;


    
    let uri = "amqp://localhost:5672";
    let options = ConnectionProperties::default()
        // Use tokio executor and reactor.
        // At the moment the reactor is only available for unix.
        .with_executor(tokio_executor_trait::Tokio::current())
        .with_reactor(tokio_reactor_trait::Tokio);

    let connection = Connection::connect(uri, options).await.unwrap();
    let channel = connection.create_channel().await.unwrap();


    
    let rabbit_stream_queue = RabbitMQQueue::new(channel, "job_prover9").await?;
    let mut worker_event_processor = QEDRabbitMQEventProcessor::new(rabbit_stream_queue, redis_queue);
    let mut fake_prover = QEDFakeProver{};

    const D: usize = 2;
    type C = PoseidonGoldilocksConfig;

    timer.lap("started up");

    SimpleActorWorker::run_worker::<_,_,_, C,D>(
        &mut proof_store,
        &mut worker_event_processor,
        &mut fake_prover,
    ).await?;
    timer.lap("finished jobs");

    Ok(())
}