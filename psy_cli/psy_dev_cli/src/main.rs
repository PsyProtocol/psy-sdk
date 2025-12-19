mod aws;
mod subcommand;
use clap::Parser;
use subcommand::{Cli, Commands};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();

    let cli = Cli::parse();
    psy_common::setup_logging()?;

    match cli.command {
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
        Commands::GetRegistrationIdFromUserId(args) => {
            subcommand::get_registration_id_from_user_id::run(args).await?;
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
        Commands::RealmStatus => {
            subcommand::realm_status::run().await?;
        }
        Commands::CheckRegisteredUsers(args) => {
            subcommand::check_registered_users::run(args).await?;
        }
        Commands::Store(args) => {
            subcommand::store::run(args).await?;
        }
    }

    Ok(())
}
