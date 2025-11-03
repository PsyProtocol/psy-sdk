use anyhow::Result;
use psy_node::watcher::{
    config::{WatcherArgs, WatcherConfig},
    watcher_service::WatcherService,
};
use tracing::info;

pub async fn run(args: WatcherArgs) -> Result<()> {
    info!("Starting watcher service");

    // Convert args to config
    let config = WatcherConfig::from_args(args)?;

    info!("Configuration: {:#?}", config);

    // Create and run service
    let service = WatcherService::new(config).await?;
    service.run().await
}
