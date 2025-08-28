use clap::Parser;
use serde::{Deserialize, Serialize};
use qed_store::store::backend::BackendConfig;

#[derive(Clone, Debug, Deserialize, Serialize, Parser)]
#[serde(default)]
pub struct RedisConfig {
    /// Redis URL
    #[arg(
        long,
        env = "REALM_REDIS_URI",
        default_value = "redis://127.0.0.1:6379"
    )]
    pub redis_uri: String,

    /// Redis connection pool size
    #[arg(long, env = "REALM_REDIS_POOL_SIZE")]
    pub pool_size: Option<usize>,
}

impl Default for RedisConfig {
    fn default() -> Self {
        Self {
            redis_uri: "redis://127.0.0.1:6379".to_string(),
            pool_size: Some(10),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, Parser)]
#[serde(default)]
pub struct RealmConfig {
    /// Node ID
    #[arg(long, env = "REALM_NODE_ID", default_value_t = 1)]
    pub node_id: u32,
    /// Realm ID
    #[arg(long, env = "REALM_REALM_ID", default_value_t = 0)]
    pub realm_id: u32,
}

impl Default for RealmConfig {
    fn default() -> Self {
        Self {
            node_id: 1,
            realm_id: 0,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, Parser)]
#[serde(default)]
pub struct QueueConfig {
    /// Worker queue suffix
    #[arg(long, env = "REALM_QUEUE_BIZ_KEY", default_value = "rwq0")]
    pub queue_biz_key: String,
}

impl Default for QueueConfig {
    fn default() -> Self {
        Self {
            queue_biz_key: "rwq0".to_string(),
        }
    }
}



#[derive(Clone, Debug, Deserialize, Serialize, Parser)]
#[serde(default)]
pub struct RPCConfig {
    /// RPC listen address
    #[arg(long, env = "REALM_EDGE_LISTEN_ADDR", default_value = "0.0.0.0:8546")]
    pub listen_addr: String,

    /// Coordinator RPC address
    #[arg(
        long,
        env = "COORDINATOR_EDGE_ADDR",
        default_value = "http://127.0.0.1:8545"
    )]
    pub coordinator_addr: String,
}

impl Default for RPCConfig {
    fn default() -> Self {
        Self {
            listen_addr: "0.0.0.0:8546".to_string(),
            coordinator_addr: "http://127.0.0.1:8545".to_string(),
        }
    }
}

#[derive(Default, Clone, Debug, Deserialize, Serialize, Parser)]
#[serde(default)]
pub struct RealmNodeConfig {
    /// Realm configuration
    #[command(flatten)]
    pub realm: RealmConfig,

    /// Store backend configuration
    #[command(flatten)]
    pub backend: BackendConfig,

    /// Redis configuration for queue and proof storage
    #[command(flatten)]
    pub redis: RedisConfig,

    /// Queue configuration
    #[command(flatten)]
    pub queue: QueueConfig,

    /// Path to configuration file
    #[arg(long, help = "Path to configuration file", default_value = "config.json")]
    pub config_path: String,
}

#[derive(Default, Clone, Debug, Deserialize, Serialize, Parser)]
#[serde(default)]
pub struct RealmEdgeConfig {
    /// RPC configuration
    #[command(flatten)]
    pub rpc: RPCConfig,

    /// Realm configuration
    #[command(flatten)]
    pub realm: RealmConfig,

    /// Store backend configuration
    #[command(flatten)]
    pub backend: BackendConfig,

    /// Redis configuration for queue and proof storage
    #[command(flatten)]
    pub redis: RedisConfig,

    /// Queue configuration
    #[command(flatten)]
    pub queue: QueueConfig,

    //worker white list file path
    #[arg(long, help = "Path to configuration file", default_value = "config.json")]
    pub config_path: String,
}

impl RealmEdgeConfig {
    pub fn queue_biz_key(&self) -> String {
        if self.queue.queue_biz_key.is_empty() {
            format!("{}",self.realm.realm_id)
        }else{
            self.queue.queue_biz_key.clone()
        }
    }
}

impl RealmNodeConfig {
    pub fn queue_biz_key(&self) -> String {
        if self.queue.queue_biz_key.is_empty() {
            format!("{}",self.realm.realm_id)
        }else{
            self.queue.queue_biz_key.clone()
        }
    }
}