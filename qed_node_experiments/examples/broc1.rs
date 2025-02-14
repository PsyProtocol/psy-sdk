use broccoli_queue::{queue::BroccoliQueue, brokers::broker::BrokerMessage};
use qed_core::utils::debug_timer::DebugTimer;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct JobPayload {
    id: String,
    task_name: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {

    let mut timer = DebugTimer::new("pub");
    // Initialize the queue
    let queue = BroccoliQueue::builder("redis://localhost:6379")
        .pool_connections(32) // Optional: Number of connections to pool
        .failed_message_retry_strategy(Default::default()) // Optional: Retry strategy (max retries, etc)
        .build()
        .await?;
    timer.lap("built queue");
    // Create some example jobs
    let jobs = (0..10000).map(|i|{
        JobPayload {
            id: format!("job-{}",i),
            task_name: "process_data".to_string(),
        }
    }).collect::<Vec<_>>();
    timer.lap("generated jobs");

    // Publish jobs in batch
    queue.publish_batch(
        "jobs", // Queue name
        None,
         jobs // Jobs to publish
         ,None
    ).await?;
    timer.lap("published jobs");

    Ok(())
}