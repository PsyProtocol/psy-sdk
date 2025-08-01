use async_trait::async_trait;
use anyhow::Result;
use rsmq::{PoolOptions, PooledRsmq, RedisBytes, RsmqConnection, RsmqError, RsmqOptions, RsmqMessage};
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use futures::future::join_all;
use tokio::sync::Semaphore;
use tracing::{debug, error, info, trace};

// ============================================================================
// Constants
// ============================================================================

/// Maximum number of concurrent message sends
const MAX_CONCURRENT_SENDS: usize = 50;

/// Batch size for sending messages
const BATCH_SEND_SIZE: usize = 100;

// ============================================================================
// RsmqTaskQueue Implementation
// ============================================================================

/// Task queue implementation using RSMQ without QueueId dependency
pub struct RsmqTaskQueue {
    /// Connection pool for RSMQ operations
    pub pool: PooledRsmq,
    /// Semaphore to limit concurrent send operations
    send_semaphore: Arc<Semaphore>,
}

impl RsmqTaskQueue {
    /// Creates a new RsmqTaskQueue instance
    ///
    /// # Arguments
    /// * `redis_url` - Redis connection URL (e.g., "redis://localhost:6379/0")
    /// * `pool_size` - Number of connections to maintain in the pool
    pub async fn new(redis_url: &str, pool_size: usize) -> Result<Self> {
        let pool = Self::create_rsmq_pool(redis_url, pool_size).await?;

        Ok(Self {
            pool,
            send_semaphore: Arc::new(Semaphore::new(MAX_CONCURRENT_SENDS)),
        })
    }

    /// Creates an RSMQ connection pool from Redis URL
    async fn create_rsmq_pool(redis_url: &str, pool_size: usize) -> Result<PooledRsmq> {
        let url = url::Url::parse(redis_url)?;
        let mut rsmq_options = RsmqOptions::default();

        // Extract host
        if let Some(host) = url.host() {
            rsmq_options.host = host.to_string();
        }

        // Extract port
        if let Some(port) = url.port() {
            rsmq_options.port = port;
        }

        // Extract database index from path (e.g., /0, /1, etc.)
        let path = url.path();
        if path.starts_with('/') && path.len() > 1 {
            let db_index_str = &path[1..];
            let db = u8::from_str(db_index_str)?;
            rsmq_options.db = db;
        }

        info!(
            "Creating RSMQ pool - URL: {}, Host: {}, Port: {}, DB: {}, Pool size: {}",
            redis_url, rsmq_options.host, rsmq_options.port, rsmq_options.db, pool_size
        );

        let pool_options = PoolOptions {
            max_size: Some(pool_size as u32),
            min_idle: Some((pool_size / 2) as u32), // Keep half the connections warm
        };

        let pool = PooledRsmq::new(rsmq_options, pool_options).await?;
        Ok(pool)
    }

    /// Creates a queue if it doesn't already exist
    ///
    /// # Arguments
    /// * `queue_name` - Name of the queue to create
    pub async fn create_queue_if_not_exists(&self, queue_name: &str) -> Result<()> {
        match self.pool.get_queue_attributes(queue_name).await {
            Ok(_) => {
                trace!("Queue '{}' already exists", queue_name);
                Ok(())
            },
            Err(RsmqError::QueueNotFound) => {
                debug!("Creating queue '{}'", queue_name);
                match self.pool.create_queue(queue_name, None, None, None).await {
                    Ok(()) => {
                        info!("Created queue '{}'", queue_name);
                        Ok(())
                    },
                    Err(RsmqError::QueueExists) => {
                        // Race condition - queue was created by another process
                        trace!("Queue '{}' was created by another process", queue_name);
                        Ok(())
                    },
                    Err(err) => Err(err.into()),
                }
            },
            Err(err) => Err(err.into()),
        }
    }

    /// Sends a single message to a queue
    ///
    /// # Arguments
    /// * `queue_name` - Name of the target queue
    /// * `message` - Message to send
    pub async fn send_message<E: Into<RedisBytes> + Send>(
        &self,
        queue_name: &str,
        message: E,
    ) -> Result<()> {
        self.create_queue_if_not_exists(queue_name).await?;

        let bytes = message.into();
        self.pool.send_message(queue_name, bytes, None).await?;

        trace!("Sent message to queue '{}'", queue_name);
        Ok(())
    }

    /// Sends multiple messages in batches with concurrent execution
    ///
    /// # Arguments
    /// * `queue_name` - Name of the target queue
    /// * `messages` - Vector of messages to send
    ///
    /// # Returns
    /// Number of successfully sent messages
    pub async fn send_messages_batch<E>(
        &self,
        queue_name: &str,
        messages: Vec<E>,
    ) -> Result<usize>
        where
            E: Into<RedisBytes> + Send + Clone + 'static,
    {
        if messages.is_empty() {
            debug!("No messages to send to queue '{}'", queue_name);
            return Ok(0);
        }

        let start_time = std::time::Instant::now();
        info!("Starting batch send of {} messages to queue '{}'", messages.len(), queue_name);

        // Ensure queue exists
        self.create_queue_if_not_exists(queue_name).await?;

        // Split messages into chunks for concurrent processing
        let chunks: Vec<Vec<E>> = messages
            .chunks(BATCH_SEND_SIZE)
            .map(|chunk| chunk.to_vec())
            .collect();

        debug!(
            "Split {} messages into {} chunks of up to {} messages each",
            messages.len(), chunks.len(), BATCH_SEND_SIZE
        );

        // Process chunks concurrently
        let mut tasks = Vec::new();

        for (chunk_idx, chunk) in chunks.into_iter().enumerate() {
            let pool = self.pool.clone();
            let queue_name = queue_name.to_string();
            let semaphore = self.send_semaphore.clone();
            let chunk_size = chunk.len();

            let task = tokio::spawn(async move {
                // Acquire semaphore to limit concurrent operations
                let _permit = semaphore.acquire().await
                    .map_err(|e| anyhow::anyhow!("Failed to acquire semaphore: {}", e))?;

                trace!("Processing chunk {} with {} messages", chunk_idx, chunk_size);

                let mut success_count = 0;
                let mut first_error = None;

                for (msg_idx, message) in chunk.into_iter().enumerate() {
                    match pool.send_message(&queue_name, message.into(), None).await {
                        Ok(_) => success_count += 1,
                        Err(e) => {
                            error!(
                                "Failed to send message {}/{} in chunk {}: {}",
                                msg_idx + 1, chunk_size, chunk_idx, e
                            );
                            if first_error.is_none() {
                                first_error = Some(e);
                            }
                        }
                    }
                }

                if let Some(err) = first_error {
                    if success_count == 0 {
                        return Err(anyhow::anyhow!("All messages in chunk {} failed: {}", chunk_idx, err));
                    }
                }

                Ok::<usize, anyhow::Error>(success_count)
            });

            tasks.push(task);
        }

        // Wait for all tasks to complete
        let results = join_all(tasks).await;

        let mut total_success = 0;
        let mut total_errors = 0;
        let mut chunk_failures = Vec::new();

        for (idx, result) in results.iter().enumerate() {
            match result {
                Ok(Ok(count)) => {
                    total_success += count;
                    trace!("Chunk {} succeeded with {} messages", idx, count);
                },
                Ok(Err(e)) => {
                    error!("Chunk {} failed: {}", idx, e);
                    chunk_failures.push(idx);
                    total_errors += 1;
                },
                Err(e) => {
                    error!("Task {} panicked: {}", idx, e);
                    chunk_failures.push(idx);
                    total_errors += 1;
                }
            }
        }

        let elapsed = start_time.elapsed();

        if total_errors > 0 {
            //todo!: need add retry logic for failed chunks
            error!(
                "Batch send to '{}' partially failed in {:?} - Success: {}/{}, Failed chunks: {:?}",
                queue_name, elapsed, total_success, messages.len(), chunk_failures
            );
            Err(anyhow::anyhow!(
                "Batch send partially failed: {} chunks failed out of {}",
                total_errors, results.len()
            ))
        } else {
            info!(
                "Batch send to '{}' completed in {:?} - Success: {}/{}",
                queue_name, elapsed, total_success, messages.len()
            );
            Ok(total_success)
        }
    }

    /// Receives a message from the queue (without message ID)
    ///
    /// # Arguments
    /// * `queue_name` - Name of the queue
    /// * `hidden` - How long the message should be hidden from other consumers
    pub async fn receive_message(
        &self,
        queue_name: &str,
        hidden: Option<Duration>,
    ) -> Result<Option<Vec<u8>>> {
        self.create_queue_if_not_exists(queue_name).await?;

        let message = self.pool
            .receive_message::<Vec<u8>>(queue_name, hidden)
            .await?;

        if message.is_some() {
            trace!("Received message from queue '{}'", queue_name);
        }

        Ok(message.map(|msg| msg.message))
    }

    /// Receives a message with its ID for later deletion
    ///
    /// # Arguments
    /// * `queue_name` - Name of the queue
    /// * `hidden` - How long the message should be hidden from other consumers
    pub async fn receive_message_with_id(
        &self,
        queue_name: &str,
        hidden: Option<Duration>,
    ) -> Result<Option<RsmqMessage<Vec<u8>>>> {
        self.create_queue_if_not_exists(queue_name).await?;

        let message = self.pool
            .receive_message::<Vec<u8>>(queue_name, hidden)
            .await?;

        if let Some(ref msg) = message {
            trace!("Received message with ID '{}' from queue '{}'", msg.id, queue_name);
        }

        Ok(message)
    }

    /// Deletes a message from the queue
    ///
    /// # Arguments
    /// * `queue_name` - Name of the queue
    /// * `message_id` - ID of the message to delete
    pub async fn delete_message(&self, queue_name: &str, message_id: &str) -> Result<()> {
        self.pool.delete_message(queue_name, message_id).await?;
        trace!("Deleted message '{}' from queue '{}'", message_id, queue_name);
        Ok(())
    }


    /// Gets the number of messages in a queue
    ///
    /// # Arguments
    /// * `queue_name` - Name of the queue
    pub async fn count_queue_len(&self, queue_name: &str) -> Result<u64> {
        match self.pool.get_queue_attributes(queue_name).await {
            Ok(attr) => {
                debug!("Queue '{}' has {} messages", queue_name, attr.msgs);
                Ok(attr.msgs)
            },
            Err(RsmqError::QueueNotFound) => {
                debug!("Queue '{}' not found, returning 0", queue_name);
                Ok(0)
            },
            Err(err) => Err(err.into()),
        }
    }

    /// Gets detailed statistics for a queue
    ///
    /// # Arguments
    /// * `queue_name` - Name of the queue
    pub async fn get_queue_stats(&self, queue_name: &str) -> Result<QueueStats> {
        match self.pool.get_queue_attributes(queue_name).await {
            Ok(attr) => {
                Ok(QueueStats {
                    queue_name: queue_name.to_string(),
                    total_messages: attr.msgs,
                    hidden_messages: attr.hiddenmsgs,
                    total_sent: attr.totalsent,
                    total_received: attr.totalrecv,
                    created_at: attr.created,
                    modified_at: attr.modified,
                })
            },
            Err(RsmqError::QueueNotFound) => {
                Ok(QueueStats {
                    queue_name: queue_name.to_string(),
                    total_messages: 0,
                    hidden_messages: 0,
                    total_sent: 0,
                    total_received: 0,
                    created_at: 0,
                    modified_at: 0,
                })
            },
            Err(err) => Err(err.into()),
        }
    }

    /// Lists all queues
    pub async fn list_queues(&self) -> Result<Vec<String>> {
        let queues = self.pool.list_queues().await?;
        info!("Found {} queues", queues.len());
        Ok(queues)
    }

    /// Deletes a queue
    ///
    /// # Arguments
    /// * `queue_name` - Name of the queue to delete
    pub async fn delete_queue(&self, queue_name: &str) -> Result<()> {
        self.pool.delete_queue(queue_name).await?;
        info!("Deleted queue '{}'", queue_name);
        Ok(())
    }

    /// Changes the visibility timeout of a message
    ///
    /// # Arguments
    /// * `queue_name` - Name of the queue
    /// * `message_id` - ID of the message
    /// * `hidden` - New visibility timeout duration
    pub async fn change_message_visibility(
        &self,
        queue_name: &str,
        message_id: &str,
        hidden: Duration,
    ) -> Result<()> {
        self.pool.change_message_visibility(queue_name, message_id, hidden).await?;
        trace!(
            "Changed visibility of message '{}' in queue '{}' to {:?}",
            message_id, queue_name, hidden
        );
        Ok(())
    }
}

// ============================================================================
// Data Structures
// ============================================================================

/// Queue statistics
#[derive(Debug, Clone)]
pub struct QueueStats {
    pub queue_name: String,
    pub total_messages: u64,
    pub hidden_messages: u64,
    pub total_sent: u64,
    pub total_received: u64,
    pub created_at: u64,
    pub modified_at: u64,
}

// ============================================================================
// Usage Examples
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_queue_operations() -> Result<()> {
        let queue = RsmqTaskQueue::new("redis://localhost:6379/0", 10).await?;
        let queue_name = "test_queue";

        // Send a single message
        queue.send_message(queue_name, b"Hello World".to_vec()).await?;

        // Send multiple messages
        let messages = vec![
            b"Message 1".to_vec(),
            b"Message 2".to_vec(),
            b"Message 3".to_vec(),
        ];
        let sent = queue.send_messages_batch(queue_name, messages).await?;
        assert_eq!(sent, 3);

        // Receive a message with ID
        if let Some(msg) = queue.receive_message_with_id(queue_name, Some(Duration::from_secs(30))).await? {
            println!("Received: {:?}", String::from_utf8_lossy(&msg.message));

            // Delete the message
            queue.delete_message(queue_name, &msg.id).await?;
        }

        // Get queue stats
        let stats = queue.get_queue_stats(queue_name).await?;
        println!("Queue stats: {:?}", stats);

        Ok(())
    }
}