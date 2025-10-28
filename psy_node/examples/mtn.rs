use fred::prelude::*;
use psy_core::{
    job::{id::{ProvingJobCircuitType, QProvingJobDataID}, traits::QProofStoreWriterAsyncImm, worker_queue::WorkerEventTransmitterAsyncImm},
    utils::debug_timer::DebugTimer,
};
use psy_store::queue::ProofStoreFred;
use std::time::Duration;
use psy_store::queue::new_fred_pool;

fn gen_jobs_ids(checkpoint_id: u64, height: usize) -> Vec<Vec<QProvingJobDataID>> {
    let mut jobs = Vec::with_capacity(height);
    for h in 0..=height {
        let level = height-h;

        let num_nodes = 1usize<<level;
        let mut level_jobs = Vec::with_capacity(num_nodes);
        if h == 0 {
            for i in 0..num_nodes {
                let id = QProvingJobDataID::guta_two_end_cap_witness(checkpoint_id, 0, ProvingJobCircuitType::GUTATwoEndCap.to_circuit_group_id(), h as u32, i as u32);
                level_jobs.push(id);
            }
        }else{
            for i in 0..num_nodes {
                let id = QProvingJobDataID::guta_two_agg_witness(checkpoint_id, 0, ProvingJobCircuitType::GUTATwoGUTA.to_circuit_group_id(), h as u32, i as u32);
                level_jobs.push(id);
            }
        }

         jobs.push(level_jobs);
    }
    jobs

}


async fn run_fred_test3() -> anyhow::Result<()> {


    let mut timer = DebugTimer::new("dq_rust_2v2");
    timer.lap("start");

    let pool = new_fred_pool("redis://127.0.0.1:6379",8).await?;
    timer.lap("connected to redis");

    let q = ProofStoreFred::new(pool, "wq1".to_string());

    timer.lap("started up");
    let checkpoint_id: u64 = 13;
    let jobs = gen_jobs_ids(checkpoint_id, 15);
    timer.lap("generated jobs");
    q.write_multidimensional_jobs(&jobs, &[QProvingJobDataID::notify_block_complete(checkpoint_id, 0, 0)]).await?;
    timer.lap("wrote to proof store");
    //println!("jobs: {:?}",jobs);
    q.enqueue_jobs_imm(&jobs[0]).await?;
    timer.lap("enqueued jobs");
    //worker_event_processor.job_queue.channel.close(reply_code, reply_text)
    q.wait_for_block_proving_jobs_imm(checkpoint_id, None).await?;

    timer.lap("finished jobs");

    Ok(())
}
#[tokio::main]
async fn main() {
    run_fred_test3().await.unwrap();
}
