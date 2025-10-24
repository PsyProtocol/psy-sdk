pub mod rsmq_queue;
pub mod pool;
pub mod fred_queue;
pub mod redis_queue;
pub mod task_queue;
pub mod resilient_redis;
pub mod tx_pool;

pub use pool::{new_fred_pool, new_redis_async_pool, new_resilient_redis_connection};
pub use resilient_redis::{ResilientRedisConnection, ConnectionStats, CommandBuilder};
pub use rsmq_queue::*;
pub use fred_queue::{DrainQueueFred, ProofStoreFred};
pub use redis_queue::{ProofStoreRedisAsync, BizKey, QueuePrefixKey, QPendingUserStoreAsyncImm};
pub mod worker_queue_redis {
    pub mod redis_queue {
        pub use crate::queue::rsmq_queue::{
            RedisQueue, QueueCmd, QueueNotification, CEQueueNotification,
            Q_RPC_TOKEN_TRANSFER, Q_RPC_CLAIM_DEPOSIT, Q_RPC_ADD_WITHDRAWAL, Q_RPC_REGISTER_USER,
            Q_CMD, Q_JOB, Q_NOTIFICATIONS, CE_NOTIFICATIONS, Q_HIDDEN, Q_DELAY, Q_CAP
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
