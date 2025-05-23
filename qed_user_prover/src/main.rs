use clap::Parser;
use qed_user_prover::{args::ProverArgs, run_server};

#[tokio::main]
pub async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();

    let args = ProverArgs::parse();
    qed_rollup_utils::setup_logging("info".to_owned())?;
    tracing::info!("start user prover server");

    let ctrl_c = tokio::signal::ctrl_c();
    tokio::select! {
        result = run_server(args) => {
            match result {
                Ok(()) => tracing::warn!("user prover server exit."),
                Err(e) => tracing::error!("user prover server exit error: {:?}", e),
            }
        }
        _ = ctrl_c => {
            tracing::warn!("Ctrl-C signal received, cleaning up...");
        }
    }
    Ok(())
}
