use std::time::Duration;
pub const WATCHER_RSMQ: &str = "waq"; // watcher queue
pub const MAX_CONCURRENT_TASKS: usize = 1000;
pub const MAX_RETRY_ATTEMPTS: u32 = 3;
pub const MAX_SINGLE_MESSAGE_PROCESSING_TIME_SECS: Duration = Duration::from_secs(10);
pub const SLEEP_TIME_IF_NO_MSG_MILLIS: u64 = 100;
pub const SLEEP_TIME_IF_HAVE_MSG_MILLIS: u64 = 10;


pub fn get_queue_name(biz_key: &str) -> String {
    format!("{}:{}", biz_key, WATCHER_RSMQ)
}