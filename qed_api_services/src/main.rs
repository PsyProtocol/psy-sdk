use qed_api_services::{config::Config, run};
use tracing_subscriber::{self, EnvFilter};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing with configurable log level
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::new(std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string())))
        .init();

    let config = Config::default();

    run(config).await
}