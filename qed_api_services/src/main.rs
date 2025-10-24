use clap::Parser;
use qed_api_services::{config::Config, run};
use tracing_subscriber::{self, EnvFilter};

/// QED API Service - Zero Knowledge Proof Backend
#[derive(Parser, Debug)]
#[command(
    name = "qed-api",
    version,
    about = "Run the QED API Service with configurable parameters",
    long_about = None
)]
struct Cli {
    /// Listen address (e.g. 0.0.0.0)
    #[arg(long = "listen-addr", default_value = "0.0.0.0")]
    listen_addr: String,

    /// Listening port
    #[arg(long, default_value_t = 3000)]
    port: u16,

    /// Database connection URL
    #[arg(long = "database-url", default_value = "postgres://postgres:password@localhost:5432/postgres")]
    database_url: String,

    /// Database connection pool size
    #[arg(long = "max-connections", default_value_t = 20)]
    max_connections: u32,

    /// Log level (e.g. info, debug, trace)
    #[arg(long = "log-level", default_value = "info")]
    log_level: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::new(&cli.log_level))
        .init();

    let config = Config {
        server: qed_api_services::config::ServerConfig {
            host: cli.listen_addr.clone(),
            port: cli.port,
        },
        database: qed_api_services::config::DatabaseConfig {
            url: cli.database_url.clone(),
            max_connections: cli.max_connections,
        },
    };

    tracing::info!(
        "Starting QED API server at {}:{} (DB: {}, max_conn={})",
        cli.listen_addr,
        cli.port,
        cli.database_url,
        cli.max_connections
    );

    run(config).await
}