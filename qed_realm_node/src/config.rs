use serde::Deserialize;
use std::path::PathBuf;
use tracing::Level;
use tracing_appender::rolling;
use tracing_subscriber::{
    fmt::writer::MakeWriterExt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter,
};

#[derive(Clone, Debug, Deserialize)]
pub struct LogConfig {
    pub level: String,
    pub directory: PathBuf,
    pub file_name: String,
}

impl Default for LogConfig {
    fn default() -> Self {
        LogConfig {
            level: "info".to_string(),
            directory: "./logs".into(),
            file_name: "app.log".to_string(),
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct RedisConfig {
    pub url: String,
    pub pool_size: Option<usize>,
}

impl Default for RedisConfig {
    fn default() -> Self {
        Self {
            url: "redis://localhost:6379".to_string(),
            pool_size: Some(10),
        }
    }
}

#[derive(Default, Clone, Debug, Deserialize)]
pub struct RealmConfig {
    /// Node ID
    pub node_id: u32,
    /// Realm ID
    pub realm_id: u32,
}

#[derive(Clone, Debug, Deserialize)]
pub struct QueueConfig {
    pub worker_queue_suffix: String,
    pub notifications_queue_suffix: String,
}

impl Default for QueueConfig {
    fn default() -> Self {
        Self {
            worker_queue_suffix: "worker".to_string(),
            notifications_queue_suffix: "notifications".to_string(),
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct DBConfig {
    pub db_path: String,
}

impl Default for DBConfig {
    fn default() -> Self {
        Self {
            db_path: "./data".to_string(),
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct RPCConfig {
    pub listen_addr: String,
}

impl Default for RPCConfig {
    fn default() -> Self {
        Self {
            listen_addr: "127.0.0.1:8545".to_string(),
        }
    }
}

/// Realm node configuration
#[derive(Default, Clone, Debug, Deserialize)]
#[serde(default)]
pub struct RealmNodeConfig {
    /// Log config
    pub log: LogConfig,
    /// RPC config
    pub rpc: RPCConfig,
    /// Realm config
    pub realm: RealmConfig,
    /// Database config
    pub db: DBConfig,
    /// Redis URL for queue and proof storage
    pub redis: RedisConfig,
    /// Queue config
    pub queue: QueueConfig,
    /// Whether it's an edge node
    #[serde(default = "default_is_edge")]
    pub is_edge: bool,
}

fn default_is_edge() -> bool {
    true
}

impl RealmNodeConfig {
    pub fn load() -> Result<Self, config::ConfigError> {
        let config = config::Config::builder()
            .add_source(
                config::File::with_name("./config.toml")
                    .format(config::FileFormat::Toml)
                    .required(false),
            )
            .add_source(
                config::Environment::with_prefix("QED")
                    .separator("_")
                    .try_parsing(true),
            )
            .build()?;
        config.try_deserialize::<Self>()
    }
}

pub fn setup_logging(config: &LogConfig) -> anyhow::Result<()> {
    // Create log directory if it doesn't exist
    std::fs::create_dir_all(&config.directory)?;

    // Setup file appender
    let file_appender = rolling::daily(&config.directory, &config.file_name);

    // Parse log level
    let log_level = config.level.parse::<Level>().unwrap_or(Level::INFO);

    // Setup subscriber with filtering
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(format!("{}", log_level)));

    tracing_subscriber::registry()
        .with(env_filter)
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(file_appender.and(std::io::stdout))
                .with_ansi(false)
                .with_thread_ids(true)
                .with_thread_names(true)
                .with_file(true)
                .with_line_number(true)
                .with_target(true),
        )
        .init();

    Ok(())
}
