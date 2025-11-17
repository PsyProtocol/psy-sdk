use clap::{command, Parser, Subcommand};
pub use get_user_id_from_registration_id::GetUserIdFromRegistrationIdArgs;
pub use job::JobArgs;

pub mod check_registered_users;
pub mod generate;
pub mod generate_token;
pub mod get_job_proof;
pub mod get_user_id_from_registration_id;
pub mod job;
pub mod launch;
pub mod produce_block;
pub mod qhash;
pub mod realm_status;
pub mod register_user;
pub mod store;
pub mod stress_test;
#[derive(Parser)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    #[command(about = "Generate JWT access token")]
    GenerateToken(GenerateTokenArgs),

    #[command(about = "Produce a new block")]
    ProduceBlock(ProduceBlockArgs),

    #[command(about = "Register a new user")]
    RegisterUser(RegisterUserArgs),

    #[command(about = "Register random users in batch")]
    RandomRegisterUserBatch(RandomArgs),

    #[command(about = "Get user ID and realm from registration ID")]
    GetUserIdFromRegistrationId(GetUserIdFromRegistrationIdArgs),

    #[command(about = "Generate deployment configurations from config.json")]
    Generate(GenerateArgs),

    #[command(about = "Run the entire Psy network locally")]
    Run(RunArgs),

    #[command(about = "Launch Psy network for development (inspired by polkadot-launch)")]
    Launch(LaunchArgs),

    #[command(about = "Get job proof for reward claiming")]
    GetJobProof(GetJobProofArgs),

    #[command(name = "qhash", about = "QHashOut utility commands")]
    QHash(qhash::QHashArgs),

    #[command(about = "Run stress test by continuously sending transactions")]
    StressTest(StressTestArgs),

    #[command(about = "Job utility commands")]
    Job(JobArgs),

    #[command(about = "Realm status test")]
    RealmStatus,
    #[command(about = "Check registered users")]
    CheckRegisteredUsers(check_registered_users::CheckRegisteredUsersArgs),

    #[command(about = "Store utility commands")]
    Store(store::StoreConfig),
}

#[derive(Parser)]
pub struct TestFullGroup1Args {
    #[arg(long, default_value = "redis://127.0.0.1:6379")]
    pub redis_url: String,

    #[arg(long, default_value = "db")]
    pub db_path: String,
}

#[derive(Parser)]
pub struct TestRegisterV2Args {
    #[arg(long, default_value = "redis://127.0.0.1:6379")]
    pub redis_url: String,

    #[arg(long, default_value = "db")]
    pub db_path: String,
}

#[derive(Parser)]
pub struct BenchmarkFullGroup1Args {
    #[arg(long, default_value = "redis://127.0.0.1:6379")]
    pub redis_url: String,

    #[arg(long, default_value = "10")]
    pub num_workers: usize,
}

#[derive(Parser)]
pub struct BenchmarkFullGroup2Args {
    #[arg(long, default_value = "redis://127.0.0.1:6379")]
    pub redis_url: String,

    #[arg(long, default_value = "10")]
    pub num_workers: usize,
}

#[derive(Parser)]
pub struct BenchmarkFullGroup3Args {
    #[arg(long, default_value = "redis://127.0.0.1:6379")]
    pub redis_url: String,

    #[arg(long, default_value = "10")]
    pub num_workers: usize,
}

#[derive(Parser)]
pub struct BenchmarkRegisterV2Args {
    #[arg(long, default_value = "redis://127.0.0.1:6379")]
    pub redis_url: String,

    #[arg(long, default_value = "1")]
    pub num_users: usize,
}

#[derive(Parser)]
pub struct GenerateTokenArgs {
    #[arg(long, env = "PRIVATE_JWT_KEY", default_value = "ykGz8xBecyAs")]
    pub private_key: String,
    #[arg(long, default_value = "0")]
    pub realm_id: u64,
}

#[derive(Parser)]
pub struct ProduceBlockArgs {
    #[clap(env, long, default_value = "config.json", env)]
    pub rpc_config: String,
}

#[derive(Parser)]
pub struct RegisterUserArgs {
    #[clap(env, long, default_value = "config.json", env)]
    pub rpc_config: String,
    #[clap(long, short, default_value = "f93ee5497d94c7d216bb5daaf77a60a4903cb7c69b752c3e1a24753691505998")]
    pub private_key: String,
}

#[derive(Parser)]
pub struct RandomArgs {
    #[clap(env, long, default_value = "config.json", env)]
    pub rpc_config: String,
    #[clap(long, default_value = "128", env)]
    pub user_per_block: u64,
    #[clap(long, default_value = "4096", env)]
    pub total_user: u64,
    #[clap(long, default_value = "3", env)]
    pub interval: u64,
}

#[derive(Parser)]
pub struct GenerateArgs {
    #[clap(env, long, default_value = "deploy.json", env)]
    pub config: String,
    #[command(subcommand)]
    pub command: GenerateCommands,
}

#[derive(Subcommand)]
pub enum GenerateCommands {
    #[command(about = "Generate docker-compose.yml from config.json")]
    DockerCompose(GenerateDockerComposeArgs),

    #[command(about = "Generate AWS CloudFormation templates from config.json")]
    Aws(GenerateAwsArgs),
}

#[derive(Parser)]
pub struct RunArgs {
    #[clap(env, long, default_value = "config.json", env)]
    pub config: String,

    #[arg(long, help = "Backend type (lmdbx or scylla)")]
    pub backend: Option<String>,

    #[arg(long, help = "Run in detached mode")]
    pub detach: bool,

    #[arg(long, help = "Stop all running services")]
    pub stop: bool,
}

#[derive(Parser)]
pub struct GenerateDockerComposeArgs {
    #[arg(long, default_value = "docker-compose.yml", help = "Output file path")]
    pub output: String,

    #[arg(long, help = "Backend type (lmdbx or scylla)")]
    pub backend: Option<String>,
}

#[derive(Parser)]
pub struct GenerateAwsArgs {
    #[arg(long, default_value = "./aws", help = "Output directory for AWS deployment files")]
    pub output_dir: String,

    #[arg(long, help = "Force overwrite existing files")]
    pub force: bool,

    #[arg(
        long,
        default_value = "balanced",
        help = "Instance optimization strategy: cost-optimized, performance-optimized, or balanced"
    )]
    pub optimization_strategy: String,

    #[arg(long, help = "Automatically set EC2 instance types based on recommendations")]
    pub auto_instance_types: bool,
}

#[derive(Parser)]
pub struct LaunchArgs {
    #[arg(long, short = 'c', help = "Path to config.json file (default: config.json)")]
    pub config: Option<String>,

    #[arg(long, short = 'v', help = "Verbose output")]
    pub verbose: bool,
}

#[derive(Parser)]
pub struct GetJobProofArgs {
    #[arg(long, help = "Checkpoint ID")]
    pub checkpoint_id: u64,

    #[arg(long, help = "Private key (hex string)")]
    pub private_key: String,

    #[arg(long, help = "RPC config file path", default_value = "config.json")]
    pub rpc_config: String,

    #[arg(long, help = "Job ID in hex format (optional, if not provided will get all jobs for checkpoint)")]
    pub job_id: Option<String>,

    #[arg(long, help = "Sign type", default_value = "zk")]
    pub sign_type: psy_common::args::SignType,

    #[arg(long, help = "Enable verbose output showing all sibling details")]
    pub verbose: bool,
}

#[derive(Parser)]
pub struct StressTestArgs {
    #[arg(long, default_value = "config.json", help = "Path to config.json file")]
    pub config: String,

    #[arg(long, default_value = "transfer", help = "Task type (transfer, ...future task types)")]
    pub task_type: String,

    #[arg(long, default_value = "4", help = "Number of concurrent tasks")]
    pub concurrent_tasks: usize,

    #[arg(long, help = "Number of transaction tasks to execute (omit for unlimited)")]
    pub max_task: Option<u64>,

    #[arg(long, default_value = "1", help = "Number of times to run the stress test")]
    pub repeat: u64,

    #[arg(long, default_value = "false", help = "Only run user registration")]
    pub only_user: bool,

    #[arg(long, default_value = "false", help = "Only run flow")]
    pub only_flow: bool,

    #[arg(long, default_value = "false", help = "Only run multi transfer")]
    pub only_multi_transfer: bool,

    #[arg(long, default_value = "false", help = "Only run multi user transfer")]
    pub only_multi_user_transfer: bool,

    #[arg(long, default_value = "false", help = "Only run user mint")]
    pub only_mint: bool,

    #[arg(long, default_value = "false", help = "Only deploy contract")]
    pub only_deploy_contract: bool,

    #[arg(long, default_value = "", help = "Path to contract file")]
    pub contract_path: String,
}
