use clap::{command, Parser, Subcommand};

pub mod test_full_group_1;
pub mod test_register_v2;
pub mod benchmark_full_group_1;
pub mod benchmark_full_group_2;
pub mod benchmark_full_group_3;
pub mod benchmark_register_v2;
pub mod generate_token;
pub mod produce_block;
pub mod block_state;
pub mod register_user;

#[derive(Parser)]
pub struct Cli {
    #[arg(
        long = "log-level",
        default_value = "info",
        help = "Set the log level (error, warn, info, debug, trace)"
    )]
    pub log_level: String,
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    #[command(about = "Run test full group 1 (from qed_test_sandbox)")]
    TestFullGroup1(TestFullGroup1Args),
    
    #[command(about = "Run test register v2 (from qed_test_sandbox)")]
    TestRegisterV2(TestRegisterV2Args),
    
    #[command(about = "Run benchmark full group 1")]
    BenchmarkFullGroup1(BenchmarkFullGroup1Args),
    
    #[command(about = "Run benchmark full group 2")]
    BenchmarkFullGroup2(BenchmarkFullGroup2Args),
    
    #[command(about = "Run benchmark full group 3")]
    BenchmarkFullGroup3(BenchmarkFullGroup3Args),
    
    #[command(about = "Run benchmark register v2")]
    BenchmarkRegisterV2(BenchmarkRegisterV2Args),
    
    #[command(about = "Generate JWT access token")]
    GenerateToken(GenerateTokenArgs),
    
    #[command(about = "Produce a new block")]
    ProduceBlock(ProduceBlockArgs),
    
    #[command(about = "Get block state information")]
    GetBlockState(BlockStateArgs),
    
    #[command(about = "Get latest block state")]
    GetLatestBlockState(LatestBlockStateArgs),
    
    #[command(about = "Register a new user")]
    RegisterUser(RegisterUserArgs),
    
    #[command(about = "Register random users in batch")]
    RandomRegisterUserBatch(RandomArgs),
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
    #[clap(env, long, default_value = "rpc.config", env)]
    pub rpc_config: String,
}

#[derive(Parser)]
pub struct BlockStateArgs {
    #[clap(env, long, default_value = "rpc.config", env)]
    pub rpc_config: String,
    #[arg(long, default_value = "0", env)]
    pub checkpoint_id: u64,
}

#[derive(Parser)]
pub struct LatestBlockStateArgs {
    #[clap(env, long, default_value = "rpc.config", env)]
    pub rpc_config: String,
}

#[derive(Parser)]
pub struct RegisterUserArgs {
    #[clap(env, long, default_value = "rpc.config", env)]
    pub rpc_config: String,
    #[clap(
        long,
        short,
        default_value = "f93ee5497d94c7d216bb5daaf77a60a4903cb7c69b752c3e1a24753691505998"
    )]
    pub private_key: String,
}

#[derive(Parser)]
pub struct RandomArgs {
    #[clap(env, long, default_value = "rpc.config", env)]
    pub rpc_config: String,
    #[clap(long, default_value = "128", env)]
    pub user_per_block: u64,
    #[clap(long, default_value = "4096", env)]
    pub total_user: u64,
    #[clap(long, default_value = "3", env)]
    pub interval: u64,
}