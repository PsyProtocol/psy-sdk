
use lapin::{Connection, ConnectionProperties};
use qed_core::{job::{id::QProvingJobDataID, traits::QProofStoreWriterSync, worker_queue::WorkerEventTransmitterAsync}, utils::debug_timer::DebugTimer};
use qed_node::nimpl::{proof_store_redis::RedisStore, worker_queue_rabbit::{rabbit_mq_queue::RabbitMQQueue, wq_mut::QEDRabbitMQEventProcessor}, worker_queue_rabbit_stream::{rabbit_stream_queue::RabbitStreamQueue, wq_mut::QEDRabbitStreamEventProcessor}, worker_queue_redis::redis_queue::RedisQueue};





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

    timer.lap("started up");
    let checkpoint_id: u64 = 11;
    let jobs = gen_jobs_ids(checkpoint_id, 15);
    timer.lap("generated jobs");
    proof_store.write_multidimensional_jobs(&jobs, &[QProvingJobDataID::notify_block_complete(checkpoint_id)])?;
    timer.lap("wrote to proof store");
    //println!("jobs: {:?}",jobs);
    worker_event_processor.enqueue_jobs_mut(&jobs[0]).await?;
    timer.lap("enqueued jobs");
    //worker_event_processor.job_queue.channel.close(reply_code, reply_text)
    worker_event_processor.wait_for_block_proving_jobs_mut(checkpoint_id).await?;

    timer.lap("finished jobs");

    Ok(())
}