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

type C = plonky2::plonk::config::PoseidonGoldilocksConfig;
const D: usize = 2;
type F = GoldilocksField;

pub async fn run(config_path: String, public_key: Option<String>) -> anyhow::Result<()> {
    info!("Worker starting...");
    info!("Loading config from: {}", config_path);

    let config_str = fs::read_to_string(&config_path)?;
    let config: qed_prover::local::provider::Config = serde_json::from_str(&config_str)?;

    let worker_public_key = if let Some(key_str) = public_key {
        QHashOut::<GoldilocksField>::from_str(&key_str)
            .map_err(|e| anyhow::format_err!("Failed to parse public key: {}", e))?
    } else {
        get_default_worker_public_key::<GoldilocksField>()
    };

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
                worker_public_key,
                JobLocation::Coordinator,
                job_tracker.clone(),
                prover.clone(),
                library.clone(),
            ));
            handles.push(handle);
        }
    }

    for realm_config in &config.network.realm_configs {
        for rpc_url in &realm_config.rpc_url {
            let handle = tokio::spawn(run_worker(
                rpc_url.clone(),
                worker_public_key,
                JobLocation::Realm(realm_config.id),
                job_tracker.clone(),
                prover.clone(),
                library.clone(),
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

