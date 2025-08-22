use qed_node::worker::{run_worker, job_tracker::{JobLocation, WorkerJobTracker}};
use qed_core::data::qhashout::QHashOut;
use qed_core::config::network_constants::get_default_worker_public_key;
use plonky2::field::goldilocks_field::GoldilocksField;
use qed_rollup_circuit::coordinator::coordinator_helper::QEDCoordinatorCircuitManager;
use qed_crypto::common::simple_circuit_library::SimpleCircuitLibrary;
use qed_node::common::verifier::get_cached_generic_verifier;
use tokio::sync::Mutex;
use std::sync::Arc;
use tracing::{info, error};
use std::str::FromStr;
use std::fs;
use std::path::{Path, PathBuf};
// TODO: Implement wallet_interactive in secp_wallet or remove interactive mode
use qed_prover::wallet::secp_wallet::Wallet;

type C = plonky2::plonk::config::PoseidonGoldilocksConfig;
const D: usize = 2;
type F = GoldilocksField;

fn print_banner() {
    println!(r#"
 ____              __  __ _
|  _ \ ___ _   _  |  \/  (_)_ __   ___ _ __
| |_) / __| | | | | |\/| | | '_ \ / _ \ '__|
|  __/\__ \ |_| | | |  | | | | | |  __/ |
|_|   |___/\__, | |_|  |_|_|_| |_|\___|_|
           |___/
    "#);
}

pub async fn run(
    config: String,
    private_key: Option<String>,
    keystore_path: Option<String>,
    wallet_password: Option<String>,
    non_interactive: bool
) -> anyhow::Result<()> {
    print_banner();
    info!("Worker starting...");
    info!("Loading config from: {}", config);

    let config_str = fs::read_to_string(&config)?;
    let config: qed_prover::local::provider::Config = serde_json::from_str(&config_str)?;

    let wallet = Wallet::load(
        private_key.as_deref(),
        keystore_path.as_ref().map(|p| Path::new(p)),
        wallet_password.as_deref()
    )?;

    let wallet = Arc::new(wallet);
    let worker_public_key = wallet.public_key_hash();

    let proof_verifier = Arc::new(get_cached_generic_verifier::<C, D>());
    let prover = Arc::new(QEDCoordinatorCircuitManager::<C, D>::new_with_library(
        &proof_verifier.library,
        worker_public_key,
    ));
    let library = Arc::new(proof_verifier.library.clone());

    let job_tracker = Arc::new(Mutex::new(WorkerJobTracker::load_from_file(worker_public_key)));

    let mut handles = Vec::new();

    for coordinator_config in &config.network.coordinator_configs {
        for rpc_url in &coordinator_config.rpc_url {
            let handle = tokio::spawn(run_worker(
                rpc_url.clone(),
                JobLocation::Coordinator,
                job_tracker.clone(),
                prover.clone(),
                library.clone(),
                wallet.clone(),
            ));
            handles.push(handle);
        }
    }

    for realm_config in &config.network.realm_configs {
        for rpc_url in &realm_config.rpc_url {
            let handle = tokio::spawn(run_worker(
                rpc_url.clone(),
                JobLocation::Realm(realm_config.id),
                job_tracker.clone(),
                prover.clone(),
                library.clone(),
                wallet.clone(),
            ));
            handles.push(handle);
        }
    }

    info!("Started {} worker threads", handles.len());

    let ctrl_c = tokio::signal::ctrl_c();
    tokio::select! {
        _ = ctrl_c => {
            info!("Ctrl-C signal received, cleaning up...");
        }
        _ = async {
            for handle in handles {
                if let Err(e) = handle.await {
                    error!("Worker thread failed: {:?}", e);
                }
            }
        } => {
            info!("All worker threads completed");
        }
    }

    info!("Worker exit.");
    Ok(())
}

