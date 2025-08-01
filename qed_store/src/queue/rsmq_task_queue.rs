use anyhow::Result;
use rsmq::{PoolOptions, PooledRsmq, RedisBytes, RsmqConnection, RsmqError, RsmqOptions, RsmqMessage};
use std::str::FromStr;
use std::time::Duration;
use tracing::{debug, error, info, trace};

/// Task queue implementation using RSMQ
pub struct RsmqTaskQueue {
    /// Connection pool for RSMQ operations
    pool: PooledRsmq,
}

impl RsmqTaskQueue {
    /// Creates a new RsmqTaskQueue instance
    pub async fn new(redis_url: &str, pool_size: usize) -> Result<Self> {
        let pool = Self::create_rsmq_pool(redis_url, pool_size).await?;
        Ok(Self { pool })
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
            "Creating RSMQ pool - Host: {}, Port: {}, DB: {}, Pool size: {}",
            rsmq_options.host, rsmq_options.port, rsmq_options.db, pool_size
        );

        let pool_options = PoolOptions {
            max_size: Some(pool_size as u32),
            min_idle: Some((pool_size / 2) as u32), // Keep half the connections warm
        };

        Ok(PooledRsmq::new(rsmq_options, pool_options).await?)
    }

    /// Creates a queue if it doesn't already exist
    pub async fn create_queue_if_not_exists(&self, queue_name: &str) -> Result<()> {
        match self.pool.get_queue_attributes(queue_name).await {
            Ok(_) => {
                trace!("Queue '{}' already exists", queue_name);
                Ok(())
            }
            Err(RsmqError::QueueNotFound) => {
                debug!("Creating queue '{}'", queue_name);
                match self.pool.create_queue(queue_name, None, None, None).await {
                    Ok(()) => {
                        info!("Created queue '{}'", queue_name);
                        Ok(())
                    }
                    Err(RsmqError::QueueExists) => {
                        // Race condition - queue was created by another process
                        trace!("Queue '{}' was created by another process", queue_name);
                        Ok(())
                    }
                    Err(err) => Err(err.into()),
                }
            }
            Err(err) => Err(err.into()),
        }
    }

    /// Sends a single message to a queue
    pub async fn send_message<E>(&self, queue_name: &str, message: E) -> Result<()>
    where
        E: Into<RedisBytes> + Send,
    {
        self.pool
            .send_message(queue_name, message, None)
            .await
            .map(|_| ())
            .map_err(|e| anyhow::anyhow!("Failed to send message to queue '{}': {}", queue_name, e))
    }
    
    /// Receives a message with its ID for later deletion
    pub async fn receive_message_with_id(
        &self,
        queue_name: &str,
        hidden: Option<Duration>,
    ) -> Result<Option<RsmqMessage<Vec<u8>>>> {
        self.create_queue_if_not_exists(queue_name).await?;

        match self.pool.receive_message::<Vec<u8>>(queue_name, hidden).await {
            Ok(Some(msg)) => {
                trace!("Received message with ID '{}' from queue '{}'", msg.id, queue_name);
                Ok(Some(msg))
            }
            Ok(None) => Ok(None),
            Err(e) => Err(anyhow::anyhow!("Failed to receive message from queue '{}': {}", queue_name, e)),
        }
    }

    /// Deletes a message from the queue
    pub async fn delete_message(&self, queue_name: &str, message_id: &str) -> Result<()> {
        self.pool
            .delete_message(queue_name, message_id)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to delete message '{}' from queue '{}': {}", message_id, queue_name, e))?;

        trace!("Deleted message '{}' from queue '{}'", message_id, queue_name);
        Ok(())
    }

    /// Gets the number of messages in a queue
    pub async fn get_queue_length(&self, queue_name: &str) -> Result<u64> {
        match self.pool.get_queue_attributes(queue_name).await {
            Ok(attr) => {
                debug!("Queue '{}' contains {} messages", queue_name, attr.msgs);
                Ok(attr.msgs)
            }
            Err(RsmqError::QueueNotFound) => {
                debug!("Queue '{}' not found, returning 0", queue_name);
                Ok(0)
            }
            Err(err) => Err(anyhow::anyhow!("Failed to get queue length for '{}': {}", queue_name, err)),
        }
    }

    /// Lists all queues
    pub async fn list_queues(&self) -> Result<Vec<String>> {
        let queues = self.pool
            .list_queues()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to list queues: {}", e))?;

        info!("Found {} queues", queues.len());
        Ok(queues)
    }

    /// Deletes a queue
    pub async fn delete_queue(&self, queue_name: &str) -> Result<()> {
        self.pool
            .delete_queue(queue_name)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to delete queue '{}': {}", queue_name, e))?;

        info!("Deleted queue '{}'", queue_name);
        Ok(())
    }

    /// Gets detailed statistics for a queue
    pub async fn get_queue_stats(&self, queue_name: &str) -> Result<QueueStats> {
        match self.pool.get_queue_attributes(queue_name).await {
            Ok(attr) => Ok(QueueStats {
                queue_name: queue_name.to_string(),
                total_messages: attr.msgs,
                hidden_messages: attr.hiddenmsgs,
                total_sent: attr.totalsent,
                total_received: attr.totalrecv,
                created_at: attr.created,
                modified_at: attr.modified,
            }),
            Err(RsmqError::QueueNotFound) => Ok(QueueStats {
                queue_name: queue_name.to_string(),
                ..Default::default()
            }),
            Err(err) => Err(anyhow::anyhow!("Failed to get queue stats for '{}': {}", queue_name, err)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_queue_operations() -> Result<()> {
        let queue = RsmqTaskQueue::new("redis://localhost:6379/0", 10).await?;
        let queue_name = "test_queue";

        // Send a single message
        queue.send_message(queue_name, b"Hello World".to_vec()).await?;
        
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

/// Queue statistics
#[derive(Debug, Clone, Default)]
pub struct QueueStats {
    pub queue_name: String,
    pub total_messages: u64,
    pub hidden_messages: u64,
    pub total_sent: u64,
    pub total_received: u64,
    pub created_at: u64,
    pub modified_at: u64,
}