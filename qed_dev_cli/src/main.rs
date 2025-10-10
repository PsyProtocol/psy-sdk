mod subcommand;
mod aws;
use qed_dev_cli::test_helpers;

use clap::Parser;
use subcommand::{Cli, Commands};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();

    let cli = Cli::parse();
    qed_common::setup_logging()?;
    
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
            subcommand::produce_block::run(args).await?;
        }
        Commands::RegisterUser(args) => {
            subcommand::register_user::run(args).await?;
        }
        Commands::RandomRegisterUserBatch(args) => {
            subcommand::register_user::run_random(args).await?;
        }
        Commands::GetUserIdFromRegistrationId(args) => {
            subcommand::get_user_id_from_registration_id::run(args).await?;
        }
        Commands::Generate(args) => {
            subcommand::generate::run(args).await?;
        }
        Commands::Run(args) => {
            subcommand::generate::run_deployment(args).await?;
        }
        Commands::Launch(args) => {
            subcommand::launch::run(args).await?;
        }
        Commands::GetJobProof(args) => {
            subcommand::get_job_proof::run(args).await?;
        }
        Commands::QHash(args) => {
            subcommand::qhash::run(args)?;
        }
        Commands::StressTest(args) => {
            subcommand::stress_test::run(args).await?;
        }
        Commands::Job(args) => {
            subcommand::job::run(args).await?;
        }
        Commands::CheckRegisteredUsers(args) => {
            subcommand::check_registered_users::run(args).await?;
        }
    }
    
    Ok(())
}