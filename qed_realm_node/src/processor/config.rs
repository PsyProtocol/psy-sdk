use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct RealmProcessorConfig {
    #[serde(default = "default_rpc_node_id")]
    pub rpc_node_id: u32,
    #[serde(default = "default_realm_id")]
    pub realm_id: u32,
    #[serde(default = "default_redis_url")]
    pub redis_url: String,
    #[serde(default = "default_worker_queue_suffix")]
    pub worker_queue_suffix: String,
    #[serde(default = "default_notifications_queue_suffix")]
    pub notifications_queue_suffix: String,
}

fn default_rpc_node_id() -> u32 {
    1
}

fn default_realm_id() -> u32 {
    0
}

fn default_redis_url() -> String {
    "redis://127.0.0.1:6379".to_string()
}

fn default_worker_queue_suffix() -> String {
    "rwq1".to_string()
}

fn default_notifications_queue_suffix() -> String {
    "rnq1".to_string()
}
