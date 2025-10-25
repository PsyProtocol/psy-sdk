// constants.rs - All constants in one place
use std::time::Duration;

// Queue Configuration
pub const WATCHER_RSMQ: &str = "waq";
pub const MAX_CONCURRENT_TASKS: usize = 100;
pub const MAX_BATCH_SIZE: usize = 100;

// Retry Configuration
pub const MAX_RETRY_ATTEMPTS: u32 = 3;
pub const RETRY_ATTEMPT_TTL_SECS: u64 = 3600;
pub const CHECKPOINT_LEAF_FETCH_RETRY_COUNT: u32 = 3;
pub const CHECKPOINT_LEAF_FETCH_RETRY_DELAY_SECS: u64 = 5;

// Timing Configuration
pub const MAX_SINGLE_MESSAGE_PROCESSING_TIME: Duration = Duration::from_secs(10);
pub const SLEEP_TIME_IF_NO_MSG: Duration = Duration::from_millis(1000);
pub const SLEEP_TIME_IF_HAVE_MSG: Duration = Duration::from_millis(100);
pub const TASK_MONITOR_INTERVAL_SECS: u64 = 5;
pub const BLOCK_SYNC_TIMEOUT_SECS: u64 = 10;

// Backoff Configuration
pub const FAILURE_BACKOFF_THRESHOLD: u32 = 3;
pub const FAILURE_BACKOFF_DURATION: Duration = Duration::from_secs(10);

// Block Metadata Configuration
pub const BLOCK_METADATA_FINALIZATION_DELAY: u64 = 5;

// Redis Configuration
pub const REDIS_LOCK_TTL_SECS: usize = 10;
pub const REDIS_DEFAULT_POOL_SIZE: usize = 20;

// HTTP Configuration
pub const DEFAULT_HTTP_TIMEOUT: Duration = Duration::from_secs(30);
pub const DEFAULT_RETRY_DELAY: Duration = Duration::from_secs(1);

// JWT Configuration
pub const JWT_EXPIRATION_HOURS: i64 = 24;
pub const JWT_REFRESH_BUFFER_MINS: i64 = 5;

// Scheduled Task Configuration
pub const SCHEDULED_TASK_TTL_SECS: u64 = 7 * 24 * 3600; // 7 days
pub const SCHEDULED_TASK_MAX_RETRIES: u32 = 3;
pub const SCHEDULED_TASK_RETRY_DELAY_SECS: u64 = 60;
pub const SCHEDULED_TASK_RETRY_DELAY_BLOCKS: u64 = 5;
