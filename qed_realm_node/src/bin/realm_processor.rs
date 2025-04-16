use qed_realm_node::{RealmProcessor, RealmProcessorConfig};
use tracing::Level;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(Level::DEBUG)
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    dotenv::dotenv().ok();
    let config =
        envy::from_env::<RealmProcessorConfig>().expect("Failed to load configuration from env");
    let realm_processor = RealmProcessor::new(config).await?;
    let handle = realm_processor.start().await?;

    tokio::select! {
        _ = handle => {
            panic!("Realm processor stopped");
        }
        _ = tokio::signal::ctrl_c() => {
            tracing::info!("Received Ctrl-C, shutting down...");
        }
    }
    Ok(())
}
