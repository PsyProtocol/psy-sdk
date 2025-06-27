use serde::{Deserialize, Serialize};
use std::time::Duration;

/// ScyllaDB connection configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ScyllaDBConfig {
    /// ScyllaDB node address, e.g. "127.0.0.1:9042"
    pub uri: String,

    /// Keyspace name
    pub keyspace: String,

    /// Default consistency level
    #[serde(default = "default_consistency_level")]
    pub consistency_level: String,

    /// Connection timeout (milliseconds)
    #[serde(default = "default_timeout_ms")]
    pub timeout_ms: u64,

    /// Maximum retry attempts
    #[serde(default = "default_max_retries")]
    pub max_retries: u32,

    /// Retry interval (milliseconds)
    #[serde(default = "default_retry_interval_ms")]
    pub retry_interval_ms: u64,

    /// Connection pool size
    #[serde(default = "default_pool_size")]
    pub pool_size: u32,

    /// Replication strategy class (e.g. 'SimpleStrategy' or 'NetworkTopologyStrategy')
    #[serde(default = "default_replication_class")]
    pub replication_class: String,

    /// Replication factor
    #[serde(default = "default_replication_factor")]
    pub replication_factor: u32,
}

impl Default for ScyllaDBConfig {
    fn default() -> Self {
        Self {
            uri: "127.0.0.1:9042".to_string(),
            keyspace: "qed_storage".to_string(),
            consistency_level: default_consistency_level(),
            timeout_ms: default_timeout_ms(),
            max_retries: default_max_retries(),
            retry_interval_ms: default_retry_interval_ms(),
            pool_size: default_pool_size(),
            replication_class: default_replication_class(),
            replication_factor: default_replication_factor(),
        }
    }
}

impl ScyllaDBConfig {
    /// Create new ScyllaDB configuration
    pub fn new(uri: String, keyspace: String) -> Self {
        Self {
            uri,
            keyspace,
            ..Default::default()
        }
    }

    /// Set consistency level
    pub fn with_consistency_level(mut self, level: String) -> Self {
        self.consistency_level = level;
        self
    }

    /// Set timeout
    pub fn with_timeout(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = timeout_ms;
        self
    }

    /// Set retry configuration
    pub fn with_retry_config(mut self, max_retries: u32, retry_interval_ms: u64) -> Self {
        self.max_retries = max_retries;
        self.retry_interval_ms = retry_interval_ms;
        self
    }

    /// Set connection pool size
    pub fn with_pool_size(mut self, pool_size: u32) -> Self {
        self.pool_size = pool_size;
        self
    }

    /// Set replication strategy
    pub fn with_replication(mut self, replication_class: String, replication_factor: u32) -> Self {
        self.replication_class = replication_class;
        self.replication_factor = replication_factor;
        self
    }

    /// Get timeout as Duration
    pub fn timeout_duration(&self) -> Duration {
        Duration::from_millis(self.timeout_ms)
    }

    /// Get retry interval as Duration
    pub fn retry_interval_duration(&self) -> Duration {
        Duration::from_millis(self.retry_interval_ms)
    }

    /// Validate configuration
    pub fn validate(&self) -> anyhow::Result<()> {
        if self.uri.is_empty() {
            return Err(anyhow::anyhow!("ScyllaDB URI cannot be empty"));
        }

        if self.keyspace.is_empty() {
            return Err(anyhow::anyhow!("Keyspace cannot be empty"));
        }

        if self.timeout_ms == 0 {
            return Err(anyhow::anyhow!("Timeout must be greater than 0"));
        }

        if self.pool_size == 0 {
            return Err(anyhow::anyhow!("Pool size must be greater than 0"));
        }

        Ok(())
    }
}

/// Storage configuration, replacing the original path-based configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StoreConfig {
    /// Coordinator storage configuration
    pub coordinator_scylla: ScyllaDBConfig,

    /// Realm storage configuration
    pub realm_scylla: ScyllaDBConfig,

    /// Whether to enable debug mode
    #[serde(default)]
    pub debug_mode: bool,

    /// Whether to enable metrics collection
    #[serde(default)]
    pub enable_metrics: bool,
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
            debug_mode: false,
            enable_metrics: false,
        }
    }
}

impl StoreConfig {
    /// Load configuration from environment variables
    pub fn from_env() -> anyhow::Result<Self> {
        let mut config = Self::default();

        // Coordinator configuration
        if let Ok(uri) = std::env::var("QED_COORDINATOR_SCYLLA_URI") {
            config.coordinator_scylla.uri = uri;
        }
        if let Ok(keyspace) = std::env::var("QED_COORDINATOR_KEYSPACE") {
            config.coordinator_scylla.keyspace = keyspace;
        }

        // Realm configuration
        if let Ok(uri) = std::env::var("QED_REALM_SCYLLA_URI") {
            config.realm_scylla.uri = uri;
        }
        if let Ok(keyspace) = std::env::var("QED_REALM_KEYSPACE") {
            config.realm_scylla.keyspace = keyspace;
        }

        // Debug mode
        if let Ok(debug) = std::env::var("QED_STORE_DEBUG") {
            config.debug_mode = debug.parse().unwrap_or(false);
        }

        // Metrics collection
        if let Ok(metrics) = std::env::var("QED_STORE_METRICS") {
            config.enable_metrics = metrics.parse().unwrap_or(false);
        }

        config.validate()?;
        Ok(config)
    }

    /// Validate configuration
    pub fn validate(&self) -> anyhow::Result<()> {
        self.coordinator_scylla
            .validate()
            .map_err(|e| anyhow::anyhow!("Coordinator ScyllaDB config error: {}", e))?;
        self.realm_scylla
            .validate()
            .map_err(|e| anyhow::anyhow!("Realm ScyllaDB config error: {}", e))?;
        Ok(())
    }

    /// Save configuration to file
    pub fn save_to_file(&self, path: &str) -> anyhow::Result<()> {
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    /// Load configuration from file
    pub fn load_from_file(path: &str) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: Self = serde_json::from_str(&content)?;
        config.validate()?;
        Ok(config)
    }
}

// Default value functions
fn default_consistency_level() -> String {
    "ONE".to_string()
}

fn default_timeout_ms() -> u64 {
    30000 // 30 seconds
}

fn default_max_retries() -> u32 {
    3
}

fn default_retry_interval_ms() -> u64 {
    1000 // 1 second
}

fn default_pool_size() -> u32 {
    10
}

fn default_replication_class() -> String {
    "SimpleStrategy".to_string()
}

fn default_replication_factor() -> u32 {
    1
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
    fn test_config_validation() {
        let mut config = ScyllaDBConfig::default();
        assert!(config.validate().is_ok());

        config.uri = "".to_string();
        assert!(config.validate().is_err());

        config.uri = "localhost:9042".to_string();
        config.keyspace = "".to_string();
        assert!(config.validate().is_err());

        config.keyspace = "test".to_string();
        config.timeout_ms = 0;
        assert!(config.validate().is_err());
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

    #[test]
    fn test_store_config_file_operations() {
        let config = StoreConfig::default();
        let temp_file = NamedTempFile::new().unwrap();
        let file_path = temp_file.path().to_str().unwrap();

        // Save configuration
        config.save_to_file(file_path).unwrap();

        // Load configuration
        let loaded_config = StoreConfig::load_from_file(file_path).unwrap();

        assert_eq!(
            config.coordinator_scylla.uri,
            loaded_config.coordinator_scylla.uri
        );
        assert_eq!(
            config.realm_scylla.keyspace,
            loaded_config.realm_scylla.keyspace
        );
    }
}
