
use broccoli_queue::{queue::BroccoliQueue, brokers::broker::BrokerMessage};
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct JobPayload {
    id: String,
    task_name: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize the queue
    let queue = BroccoliQueue::builder("redis://localhost:6379")
        .pool_connections(5)
        .failed_message_retry_strategy(Default::default()) // Optional: Retry strategy (max retries, etc)
        .build()
        .await?;

    // Process messages
    queue.process_messages(
        "jobs", // Queue name
        Some(4), // Optional: Number of worker threads to spin up
        None,
        |message: BrokerMessage<JobPayload>| async move { // Message handler
            //println!("Processing job: {:?}", message);
            Ok(())
        },
    ).await?;

    Ok(())
}