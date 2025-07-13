mod subcommand;
use qed_dev_cli::test_helpers;

use clap::Parser;
use subcommand::{Cli, Commands};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();

    let cli = Cli::parse();
    qed_rollup_utils::setup_logging(cli.log_level.clone())?;
    
    match cli.command {
        Commands::TestFullGroup1(args) => {
            subcommand::test_full_group_1::run(args).await?;
        }
        Commands::TestRegisterV2(args) => {
            subcommand::test_register_v2::run(args).await?;
        }
        Commands::BenchmarkFullGroup1(args) => {
            subcommand::benchmark_full_group_1::run(args).await?;
        }
        Commands::BenchmarkFullGroup2(args) => {
            subcommand::benchmark_full_group_2::run(args).await?;
        }
        Commands::BenchmarkFullGroup3(args) => {
            subcommand::benchmark_full_group_3::run(args).await?;
        }
        Commands::BenchmarkRegisterV2(args) => {
            subcommand::benchmark_register_v2::run(args).await?;
        }
        Commands::GenerateToken(args) => {
            subcommand::generate_token::run(args).await?;
        }
        Commands::ProduceBlock(args) => {
            subcommand::produce_block::run(args)?;
        }
        Commands::GetBlockState(args) => {
            subcommand::block_state::get_l2_block_state(args)?;
        }
        Commands::GetLatestBlockState(args) => {
            subcommand::block_state::get_latest_block_state(args)?;
        }
        Commands::RegisterUser(args) => {
            subcommand::register_user::run(args)?;
        }
        Commands::RandomRegisterUserBatch(args) => {
            subcommand::register_user::run_random(args)?;
        }
        Commands::GetUserIdFromRegistrationId(args) => {
            subcommand::get_user_id_from_registration_id::run(args).await?;
        }
    }
    
    Ok(())
}