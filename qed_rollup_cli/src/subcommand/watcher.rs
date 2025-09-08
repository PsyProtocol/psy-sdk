use anyhow::Result;
use tracing::info;
use qed_node::watcher::config::{WatcherArgs, WatcherConfig};
use qed_node::watcher::watcher_service::WatcherService;

pub async fn run(args: WatcherArgs) -> Result<()> {
    info!("Starting watcher service");

    // Convert args to config
    let config = WatcherConfig::from_args(args)?;

    info!("Configuration: {:#?}", config);

    // Create and run service
    let service = std::sync::Arc::new(WatcherService::new(config).await?);
    service.run().await
}