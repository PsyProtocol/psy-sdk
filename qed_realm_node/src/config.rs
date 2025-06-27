use clap::Parser;
use serde::{Deserialize, Serialize};

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
    #[arg(long, env = "REALM_QUEUE_WORKER_QUEUE_SUFFIX", default_value = "rwq0")]
    pub worker_queue_suffix: String,

    /// Notifications queue suffix
    #[arg(
        long,
        env = "REALM_QUEUE_NOTIFICATIONS_QUEUE_SUFFIX",
        default_value = "rnq0"
    )]
    pub notifications_queue_suffix: String,

    #[arg(long, env = "REALM_PROOF_STORE_KEY_SUFFIX", default_value = "RP0")]
    pub proof_store_key_suffix: String,
}

impl Default for QueueConfig {
    fn default() -> Self {
        Self {
            worker_queue_suffix: "rwq0".to_string(),
            notifications_queue_suffix: "rnq0".to_string(),
            proof_store_key_suffix: "RP0".to_string(),
        }
    }
}


#[derive(Clone, Debug, Deserialize, Serialize, Parser)]
#[serde(default)]
pub struct ScyllaDBConfig {
    /// ScyllaDB URI
    #[arg(long, env = "REALM_SCYLLA_URI", default_value = "127.0.0.1:9042")]
    pub scylla_uri: String,
    
    /// ScyllaDB Keyspace
    #[arg(long, env = "REALM_SCYLLA_KEYSPACE", default_value = "qed_realm")]
    pub scylla_keyspace: String,
    
    
    /// Replication factor for ScyllaDB keyspace
    #[arg(long, env = "REALM_SCYLLA_REPLICATION_FACTOR", default_value_t = 3)]
    pub replication_factor: u32,
    
    /// Replication strategy for ScyllaDB keyspace
    #[arg(long, env = "REALM_SCYLLA_REPLICATION_STRATEGY", default_value = "SimpleStrategy")]
    pub replication_strategy: String,
    
    /// Data center replication settings (for NetworkTopologyStrategy)
    #[arg(long, env = "REALM_SCYLLA_DATACENTER_REPLICAS", default_value = "")]
    pub datacenter_replicas: String,
}

impl Default for ScyllaDBConfig {
    fn default() -> Self {
        Self {
            scylla_uri: "127.0.0.1:9042".to_string(),
            scylla_keyspace: "qed_realm".to_string(),
            replication_factor: 3,
            replication_strategy: "SimpleStrategy".to_string(),
            datacenter_replicas: "".to_string(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, Parser)]
#[serde(default)]
pub struct RPCConfig {
    /// RPC listen address
    #[arg(long, env = "REALM_EDGE_LISTEN_ADDR", default_value = "0.0.0.0:8546")]
    pub listen_addr: String,

    /// Coordinator RPC listen address
    #[arg(
        long,
        env = "COORDINATOR_EDGE_ADDR",
        default_value = "http://0.0.0.0:8545"
    )]
    pub coordinator_addr: String,
}

impl Default for RPCConfig {
    fn default() -> Self {
        Self {
            listen_addr: "0.0.0.0:8546".to_string(),
            coordinator_addr: "0.0.0.0:8545".to_string(),
        }
    }
}

#[derive(Default, Clone, Debug, Deserialize, Serialize, Parser)]
#[serde(default)]
pub struct RealmNodeConfig {
    /// Realm configuration
    #[command(flatten)]
    pub realm: RealmConfig,

    /// ScyllaDB configuration
    #[command(flatten)]
    pub scylla: ScyllaDBConfig,

    /// Redis configuration for queue and proof storage
    #[command(flatten)]
    pub redis: RedisConfig,

    /// Queue configuration
    #[command(flatten)]
    pub queue: QueueConfig,
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

    /// ScyllaDB configuration
    #[command(flatten)]
    pub scylla: ScyllaDBConfig,

    /// Redis configuration for queue and proof storage
    #[command(flatten)]
    pub redis: RedisConfig,

    /// Queue configuration
    #[command(flatten)]
    pub queue: QueueConfig,
}
