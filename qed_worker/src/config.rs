use clap::Parser;
use serde::{Deserialize, Serialize};
use std::io;
use std::path::PathBuf;
use tracing::Level;
use tracing_appender::rolling;
use tracing_subscriber::{
    fmt::writer::MakeWriterExt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter,
};

#[derive(Clone, Debug, Deserialize, Serialize, Parser)]
pub struct LogConfig {
    #[arg(long, env = "QED_LOG_LEVEL", default_value = "info")]
    pub level: String,

    #[arg(long, env = "QED_LOG_DIR", default_value = "./logs")]
    pub directory: PathBuf,

    #[arg(long, env = "QED_LOG_FILE_NAME", default_value = "app.log")]
    pub file_name: String,

    #[arg(long, env = "QED_LOG_ENABLE_FILE")]
    pub enable_file: bool,
}

impl Default for LogConfig {
    fn default() -> Self {
        LogConfig {
            level: "info".to_string(),
            directory: "./logs".into(),
            file_name: "app.log".to_string(),
            enable_file: false,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, Parser)]
#[serde(default)]
pub struct RedisConfig {
    /// Redis URL
    #[arg(long, env = "QED_REDIS_URL", default_value = "redis://localhost:6379")]
    pub url: String,

    /// Redis connection pool size
    #[arg(long, env = "QED_REDIS_POOL_SIZE")]
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

#[derive(Clone, Debug, Deserialize, Serialize, Parser)]
#[serde(default)]
pub struct QueueConfig {
    /// Worker queue suffix
    #[arg(long, env = "QED_QUEUE_WORKER_QUEUE_SUFFIX", default_value = "rwq1")]
    pub worker_queue_suffix: String,

    /// Notifications queue suffix
    #[arg(
        long,
        env = "QED_QUEUE_NOTIFICATIONS_QUEUE_SUFFIX",
        default_value = "rnq1"
    )]
    pub notifications_queue_suffix: String,
}

impl Default for QueueConfig {
    fn default() -> Self {
        Self {
            worker_queue_suffix: "rwq1".to_string(),
            notifications_queue_suffix: "rnq1".to_string(),
        }
    }
}

/// Worker configuration
#[derive(Default, Clone, Debug, Deserialize, Serialize, Parser)]
#[serde(default)]
pub struct WorkerConfig {
    /// Redis configuration for queue and proof storage
    #[command(flatten)]
    pub redis: RedisConfig,

    /// Queue configuration
    #[command(flatten)]
    pub queue: QueueConfig,
}

pub fn setup_logging(config: &LogConfig) -> anyhow::Result<()> {
    let log_level = config.level.parse::<Level>().unwrap_or(Level::INFO);

    // Setup subscriber with filtering
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(format!("{}", log_level)));

    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_thread_ids(true)
        .with_thread_names(true)
        .with_file(true)
        .with_line_number(true)
        .with_target(true);

    if config.enable_file {
        std::fs::create_dir_all(&config.directory)?;
        let file_appender = rolling::daily(&config.directory, &config.file_name);
        tracing_subscriber::registry()
            .with(env_filter)
            .with(
                fmt_layer
                    .with_writer(file_appender.and(io::stdout))
                    .with_ansi(false),
            )
            .init();
    } else {
        tracing_subscriber::registry()
            .with(env_filter)
            .with(fmt_layer.with_ansi(true))
            .init();
    }

    Ok(())
}
