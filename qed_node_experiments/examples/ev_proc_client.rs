
use qed_core::{job::{id::QProvingJobDataID, traits::QProofStoreWriterSync, worker_queue::WorkerEventTransmitterSync}, utils::debug_timer::DebugTimer};
use qed_node::nimpl::{proof_store_redis::RedisStore, worker_queue_redis::{redis_queue::RedisQueue, wq_mut::QEDRedisEventProcessor}};


fn gen_jobs_ids(checkpoint_id: u64, height: usize) -> Vec<Vec<QProvingJobDataID>> {
    let mut jobs = Vec::with_capacity(height);
    for h in 0..=height {
        let level = height-h;

        let num_nodes = 1usize<<level;
        let mut level_jobs = Vec::with_capacity(num_nodes);
        if h == 0 {
            for i in 0..num_nodes {
                let id = QProvingJobDataID::guta_two_end_cap_witness(checkpoint_id, h as u32, i as u32);
                level_jobs.push(id);
            }
        }else{
            for i in 0..num_nodes {
                let id = QProvingJobDataID::guta_two_agg_witness(checkpoint_id, h as u32, i as u32);
                level_jobs.push(id);
            }
        }

         jobs.push(level_jobs);
    }
    jobs

}
fn run_client() -> anyhow::Result<()> {


    let mut timer = DebugTimer::new("prove_block_demo");
    let mut proof_store = RedisStore::new("redis://localhost:6379/0")?;
    let redis_queue = RedisQueue::new("redis://localhost:6379/0")?;
    let mut worker_event_processor = QEDRedisEventProcessor::new(redis_queue.clone());
    
    timer.lap("started up");
    let jobs = gen_jobs_ids(1, 13);
    timer.lap("generated jobs");
    proof_store.write_multidimensional_jobs(&jobs, &[QProvingJobDataID::notify_block_complete(1)])?;
    timer.lap("wrote to proof store");
    worker_event_processor.enqueue_jobs_mut(&jobs[0])?;
    timer.lap("enqueued jobs");

    worker_event_processor.wait_for_block_proving_jobs_mut(1)?;

    timer.lap("finished jobs");



    Ok(())
}

fn main() {

    run_client().unwrap();

}