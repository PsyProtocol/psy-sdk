pub mod rsmq_queue;
pub mod pool;
pub mod fred_queue;
pub mod redis_queue;
pub mod task_queue;

// Re-export commonly used items
pub use pool::{new_fred_pool, new_redis_async_pool};
pub use rsmq_queue::*;
// Re-export from fred_queue for backward compatibility
pub use fred_queue::{DrainQueueFred, ProofStoreFred};
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

pub const PROOF_STORE_KEY_PREFIX_1: &'static str = "PSV1";
pub const PROOF_STORE_COUNTERS_PREFIX_1: &'static str = "proof_counters";

pub const PS_DRAIN_QUEUE_KEY_PREFIX: &'static str = "PSDQV1_";
pub const PS_WORKER_QUEUE_KEY_PREFIX: &'static str = "PSWQV1";
pub const PS_NOTIFICATIONS_QUEUE_KEY_PREFIX: &'static str = "PSNQV1";
pub const PS_HISTORY_QUEUE_KEY_PREFIX: &'static str = "PSHQV1";
pub const PS_REAML_CHECKPOINT_QUEUE_KEY_PREFIX: &'static str = "PSSQV1";
