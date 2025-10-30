use crate::watcher::constant::WATCHER_RSMQ;

pub fn get_queue_name(biz_key: &str) -> String {
    format!("{}:{}", biz_key, WATCHER_RSMQ)
}