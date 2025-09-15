mod subcommand;

use clap::Parser;

use crate::subcommand::{coordinator_edge, coordinator_processor, realm_edge, realm_processor, watcher, worker, Cli, Commands};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();

    let cli = Cli::parse();
    qed_common::setup_logging()?;
    match cli.command {
        Commands::CoordinatorEdge(args) => {
            coordinator_edge::run(args).await?;
        }
        Commands::CoordinatorProcessor(args) => {
            coordinator_processor::run(args).await?;
        }
        Commands::RealmEdge { config } => {
            realm_edge::run(config).await?;
        }
        Commands::RealmProcessor { config } => {
            realm_processor::run(config).await?;
        }
        Commands::Worker { config, private_key, keystore_path, wallet_password } => {
            worker::run(config, private_key, keystore_path, wallet_password).await?;
        }
        Commands::Watcher(args) => {
            watcher::run(args).await?;
        }
        Commands::CoordinatorProcessorSync {
            checkpoint,
            aws_bucket,
            backend_config,
        } => {
            qed_node::coordinator::recovery::run_sync_command(checkpoint, aws_bucket, backend_config).await?;
        }
        Commands::RealmProcessorSync {
            realm_id,
            checkpoint,
            aws_bucket,
            redis_uri,
            queue_biz_key,
            pool_size,
            backend_config,
        } => {
            qed_node::realm::recovery::run_realm_sync_command(
                realm_id,
                checkpoint,
                aws_bucket,
                backend_config,
                redis_uri,
                queue_biz_key,
                Some(pool_size),
            )
            .await?;
        }
    };
    Ok::<_, anyhow::Error>(())
}
