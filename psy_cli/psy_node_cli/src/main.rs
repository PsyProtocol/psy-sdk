mod subcommand;

use clap::Parser;

use crate::subcommand::{
    api_service, coordinator_edge, coordinator_processor, realm_edge, realm_edge_v2, realm_processor, realm_processor_v2, watcher, worker, Cli,
    Commands,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();

    let cli = Cli::parse();
    psy_common::setup_logging()?;
    match cli.command {
        Commands::CoordinatorProcessor(args) => {
            coordinator_processor::run(args).await?;
        }
        Commands::CoordinatorEdge(args) => {
            coordinator_edge::run(args).await?;
        }
        Commands::RealmProcessor { config } => {
            realm_processor::run(config).await?;
        }
        Commands::RealmEdge { config } => {
            realm_edge::run(config).await?;
        }
        Commands::Worker {
            config,
            private_key,
            keystore_path,
            wallet_password,
            recipient,
        } => {
            worker::run(config, private_key, keystore_path, wallet_password, recipient).await?;
        }
        Commands::ApiServices {
            host,
            port,
            database_url,
            max_connections,
        } => {
            api_service::run_api_service(host, port, database_url, max_connections).await?;
        }
        Commands::Watcher(args) => {
            watcher::run(args).await?;
        }
        Commands::CoordinatorProcessorSync {
            checkpoint,
            aws_bucket,
            backend_config,
            config_path,
        } => {
            psy_node::coordinator::recovery::run_sync_command(checkpoint, aws_bucket, backend_config, config_path).await?;
        }
        Commands::RealmProcessorSync {
            realm_id,
            checkpoint,
            aws_bucket,
            redis_uri,
            queue_biz_key,
            backend_config,
            config_path,
        } => {
            psy_node::realm::recovery::run_realm_sync_command(
                realm_id,
                checkpoint,
                aws_bucket,
                backend_config,
                redis_uri,
                queue_biz_key,
                config_path,
            )
            .await?;
        }
    };
    Ok::<_, anyhow::Error>(())
}
