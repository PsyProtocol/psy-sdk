pub mod rsmq_queue;
pub mod pool;
pub mod fred_queue;
pub mod redis_queue;
pub mod task_queue;

// Re-export commonly used items
pub use pool::{new_fred_pool, new_redis_async_pool};
pub use rsmq_queue::*;
// Re-export from fred_queue for backward compatibility
pub use fred_queue::{DrainQueueFred, ProofStoreFred, SyncProofQueue};
// Re-export from redis_queue
pub use redis_queue::{ProofStoreRedisAsync, Queue, SyncCheckpointQueue, BizKey, QueuePrefixKey};
// Re-export worker_queue_redis types from rsmq_queue
pub mod worker_queue_redis {
    pub mod redis_queue {
        pub use crate::queue::rsmq_queue::{
            RedisQueue, QueueCmd, QueueNotification, CEQueueNotification,
            Q_RPC_TOKEN_TRANSFER, Q_RPC_CLAIM_DEPOSIT, Q_RPC_ADD_WITHDRAWAL, Q_RPC_REGISTER_USER,
            Q_CMD, Q_JOB, Q_NOTIFICATIONS, CE_NOTIFICATIONS, Q_HIDDEN, Q_DELAY, Q_CAP
        };
    }
    pub mod wq_mut {
        pub use crate::queue::rsmq_queue::{
            QEDArcImmutableEventProcessorWrapper, QEDRedisEventProcessor
        };
    }
}
