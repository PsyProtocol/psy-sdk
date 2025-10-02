use clap::{command, Parser, Subcommand};

pub mod api_service;
pub mod coordinator_edge;
pub mod coordinator_processor;
pub mod realm_edge;
pub mod realm_processor;
pub mod watcher;
pub mod worker;

#[derive(Parser)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    #[command(about = "Run the coordinator edge node")]
    CoordinatorEdge(qed_node::coordinator::CoordinatorEdgeArgs),
    #[command(about = "Run the coordinator processor node")]
    CoordinatorProcessor(qed_node::coordinator::CoordinatorProcessorArgs),
    #[command(about = "Run the realm edge node")]
    RealmEdge {
        #[command(flatten)]
        config: qed_node::realm::RealmEdgeConfig,
    },
    #[command(about = "Run the realm processor node")]
    RealmProcessor {
        #[command(flatten)]
        config: qed_node::realm::RealmNodeConfig,
    },
    #[command(about = "Run the worker node")]
    Worker {
        #[arg(long = "config", default_value = "./config.json", help = "Path to config.json file")]
        config: String,

        #[arg(long = "private-key", env = "PRIVATE_KEY", help = "Private key hex string")]
        private_key: Option<String>,

        #[arg(long = "keystore-path", env = "KEYSTORE_PATH", help = "Path to wallet keystore file")]
        keystore_path: Option<String>,

        #[arg(long = "wallet-password", env = "WALLET_PASSWORD", help = "Wallet password")]
        wallet_password: Option<String>,

        #[arg(long = "recipient", help = "Recipient user ID for rewards (defaults to private key derived user ID)")]
        recipient: Option<u64>,
    },
    #[command(about = "Run the API service")]
    ApiServices {
        #[arg(long, default_value = "0.0.0.0", help = "Server host")]
        host: String,

        #[arg(long, default_value_t = 3000, help = "Server port")]
        port: u16,

        #[arg(long, default_value = "postgres://postgres:password@localhost:5432/postgres", help = "Database URL")]
        database_url: String,

        #[arg(long, default_value_t = 10, help = "Maximum database connections")]
        max_connections: u32,
    },
    #[command(about = "Run the watcher service for monitoring and reporting node status")]
    Watcher(qed_node::watcher::WatcherArgs),
    #[command(about = "Sync coordinator processor from S3 backup")]
    CoordinatorProcessorSync {
        #[arg(long = "checkpoint", help = "Target checkpoint to sync to (defaults to latest)")]
        checkpoint: Option<u64>,
        #[arg(long = "aws-bucket", help = "AWS S3 bucket for backup storage")]
        aws_bucket: String,
        #[command(flatten)]
        backend_config: qed_store::store::backend::BackendConfig,
        #[arg(long, help = "Path to configuration file", default_value = "config.json")]
        config_path: String,
    },
    #[command(about = "Sync realm processor from S3 backup")]
    RealmProcessorSync {
        #[arg(long, env = "REALM_REALM_ID", default_value_t = 0)]
        realm_id: u32,
        #[arg(long = "checkpoint", help = "Target checkpoint to sync to (defaults to latest)")]
        checkpoint: Option<u64>,
        #[arg(long = "aws-bucket", help = "AWS S3 bucket for backup storage")]
        aws_bucket: String,
        #[arg(long, env = "REALM_REDIS_URI", default_value = "redis://127.0.0.1:6379")]
        redis_uri: String,
        #[arg(long, env = "REALM_QUEUE_BIZ_KEY", default_value = "rwq0")]
        queue_biz_key: String,
        #[command(flatten)]
        backend_config: qed_store::store::backend::BackendConfig,
        #[arg(long, help = "Path to configuration file", default_value = "config.json")]
        config_path: String,
    },
}
