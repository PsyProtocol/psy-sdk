use clap::Parser;
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Clone, Debug, Serialize, Deserialize, Parser)]
#[serde(default)]
pub struct ScyllaDBConfig {
    #[arg(
        long = "scylla-uri",
        env = "SCYLLA_URI",
        default_value = "127.0.0.1:9042"
    )]
    pub uri: String,

    #[arg(
        long = "scylla-keyspace",
        env = "SCYLLA_KEYSPACE",
        default_value = "qed_storage"
    )]
    pub keyspace: String,

    #[arg(
        long = "scylla-consistency-level",
        env = "SCYLLA_CONSISTENCY_LEVEL",
        default_value = "ONE"
    )]
    pub consistency_level: String,

    #[arg(
        long = "scylla-timeout-ms",
        env = "SCYLLA_TIMEOUT_MS",
        default_value_t = 30000
    )]
    pub timeout_ms: u64,

    #[arg(
        long = "scylla-max-retries",
        env = "SCYLLA_MAX_RETRIES",
        default_value_t = 3
    )]
    pub max_retries: u32,

    #[arg(
        long = "scylla-retry-interval-ms",
        env = "SCYLLA_RETRY_INTERVAL_MS",
        default_value_t = 1000
    )]
    pub retry_interval_ms: u64,

    #[arg(
        long = "scylla-pool-size",
        env = "SCYLLA_POOL_SIZE",
        default_value_t = 10
    )]
    pub pool_size: u32,

    #[arg(
        long = "scylla-replication-class",
        env = "SCYLLA_REPLICATION_CLASS",
        default_value = "SimpleStrategy"
    )]
    pub replication_class: String,

    #[arg(
        long = "scylla-replication-factor",
        env = "SCYLLA_REPLICATION_FACTOR",
        default_value_t = 1
    )]
    pub replication_factor: u32,
}

impl Default for ScyllaDBConfig {
    fn default() -> Self {
        Self {
            uri: "127.0.0.1:9042".to_string(),
            keyspace: "qed_storage".to_string(),
            consistency_level: "ONE".to_string(),
            timeout_ms: 30000,
            max_retries: 3,
            retry_interval_ms: 1000,
            pool_size: 10,
            replication_class: "SimpleStrategy".to_string(),
            replication_factor: 1,
        }
    }
}

impl ScyllaDBConfig {
    pub fn new(uri: String, keyspace: String) -> Self {
        Self {
            uri,
            keyspace,
            ..Default::default()
        }
    }

    pub fn with_consistency_level(mut self, level: String) -> Self {
        self.consistency_level = level;
        self
    }

    pub fn with_timeout(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = timeout_ms;
        self
    }

    pub fn with_retry_config(mut self, max_retries: u32, retry_interval_ms: u64) -> Self {
        self.max_retries = max_retries;
        self.retry_interval_ms = retry_interval_ms;
        self
    }

    pub fn with_pool_size(mut self, pool_size: u32) -> Self {
        self.pool_size = pool_size;
        self
    }

    pub fn with_replication(mut self, replication_class: String, replication_factor: u32) -> Self {
        self.replication_class = replication_class;
        self.replication_factor = replication_factor;
        self
    }

    pub fn timeout_duration(&self) -> Duration {
        Duration::from_millis(self.timeout_ms)
    }

    pub fn retry_interval_duration(&self) -> Duration {
        Duration::from_millis(self.retry_interval_ms)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StoreConfig {
    pub coordinator_scylla: ScyllaDBConfig,

    pub realm_scylla: ScyllaDBConfig,
}

impl Default for StoreConfig {
    fn default() -> Self {
        Self {
            coordinator_scylla: ScyllaDBConfig::new(
                "127.0.0.1:9042".to_string(),
                "qed_coordinator".to_string(),
            ),
            realm_scylla: ScyllaDBConfig::new(
                "127.0.0.1:9042".to_string(),
                "qed_realm".to_string(),
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test_scylla_config_default() {
        let config = ScyllaDBConfig::default();
        assert_eq!(config.uri, "127.0.0.1:9042");
        assert_eq!(config.keyspace, "qed_storage");
        assert_eq!(config.consistency_level, "ONE");
        assert_eq!(config.timeout_ms, 30000);
    }

    #[test]
    fn test_scylla_config_builder() {
        let config = ScyllaDBConfig::new("localhost:9042".to_string(), "test_ks".to_string())
            .with_consistency_level("QUORUM".to_string())
            .with_timeout(60000)
            .with_retry_config(5, 2000)
            .with_pool_size(20);

        assert_eq!(config.uri, "localhost:9042");
        assert_eq!(config.keyspace, "test_ks");
        assert_eq!(config.consistency_level, "QUORUM");
        assert_eq!(config.timeout_ms, 60000);
        assert_eq!(config.max_retries, 5);
        assert_eq!(config.retry_interval_ms, 2000);
        assert_eq!(config.pool_size, 20);
    }

    #[test]
    fn test_store_config_serialization() {
        let config = StoreConfig::default();
        let json = serde_json::to_string_pretty(&config).unwrap();
        let deserialized: StoreConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(
            config.coordinator_scylla.uri,
            deserialized.coordinator_scylla.uri
        );
        assert_eq!(
            config.realm_scylla.keyspace,
            deserialized.realm_scylla.keyspace
        );
    }
}
