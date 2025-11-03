use std::{
    fs,
    path::{Path, PathBuf},
    str::FromStr,
    sync::Arc,
    time::Duration,
};

use kvq::traits::KVQSerializable;
use plonky2::{
    field::{goldilocks_field::GoldilocksField, types::Field},
    hash::hash_types::HashOut,
    plonk::config::GenericHashOut,
};
use psy_config::{get_default_worker_public_key, PSY_NETWORK_MAGIC};
use psy_common::data::{base_types::hash256::Hash256, qhashout::QHashOut};
use psy_crypto::{common::simple_circuit_library::SimpleCircuitLibrary, hash::traits::qhashable::QFieldHashable};
use psy_data::config::store_config::{PsyFelt, PsyHash, PsyHasher};
use psy_network_circuit::coordinator::coordinator_helper::PsyCoordinatorCircuitManager;
use psy_node::{
    common::{
        retry::{retry_with_backoff, RetryConfig},
        verifier::get_cached_generic_verifier,
    },
    worker::{
        client::WorkerCoordinatorClient,
        job_tracker::{JobLocation, WorkerJobTracker},
        run_worker,
    },
};
use psy_rust_sdk::wallet::secp_wallet::Wallet;
use psy_ups_circuit::circuit_manager::core::PsyUPSStepCircuitManager;
use tokio::{sync::Mutex, time::sleep};
use tracing::{error, info, log::warn};

type C = plonky2::plonk::config::PoseidonGoldilocksConfig;
const D: usize = 2;
type F = GoldilocksField;

fn print_banner() {
    println!(
        r#"
 ____              __  __ _
|  _ \ ___ _   _  |  \/  (_)_ __   ___ _ __
| |_) / __| | | | | |\/| | | '_ \ / _ \ '__|
|  __/\__ \ |_| | | |  | | | | | |  __/ |
|_|   |___/\__, | |_|  |_|_|_| |_|\___|_|
           |___/
    "#
    );
}

pub async fn run(
    config: String,
    private_key: Option<String>,
    keystore_path: Option<String>,
    wallet_password: Option<String>,
    recipient: Option<u64>,
) -> anyhow::Result<()> {
    print_banner();
    info!("Worker starting...");
    info!("Loading config from: {}", config);

    let config = psy_rust_sdk::provider::Config::from_file(&config)?;

    let wallet = Wallet::load(
        private_key.as_deref(),
        keystore_path.as_ref().map(|p| Path::new(p)),
        wallet_password.as_deref(),
    )?;

    info!("Worker ETH Address: {}", wallet.address());

    let wallet = Arc::new(wallet);

    let main_circuits = psy_ups_circuit::circuit_manager::core::QCircuitManager::Local(PsyUPSStepCircuitManager::<C, D>::new_with_config(
        psy_config::network_constants::PSY_NETWORK_MAGIC,
    ));

    let mut memory_wallet = psy_prover::wallet::memory_wallet::PsyMemoryWallet::new(vec![Box::new(main_circuits)]);

    let private_key = QHashOut::from(Hash256::from_bytes(&wallet.private_key())?);
    let public_key_info = memory_wallet.add_secp_private_key(private_key).await?;
    let worker_public_key = public_key_info.qfhash::<PsyHasher>();

    let proof_verifier = Arc::new(get_cached_generic_verifier::<C, D>());

    let network = config.get_current_network()?;
    let worker_coordinator_client = WorkerCoordinatorClient::new(&network.coordinator_configs[0].rpc_url[0]).await?;

    // Use retry_with_backoff from retry.rs
    let retry_config = RetryConfig {
        max_retries: 60,
        base_delay_ms: 10000,       // 10 seconds
        exponential_backoff: false, // Keep constant delay like original
    };

    let user_id = retry_with_backoff(&retry_config, &format!("get user ID for {}", worker_public_key), || async {
        worker_coordinator_client.get_user_id(&worker_public_key).await
    })
    .await
    .map_err(|e| {
        error!("Failed to get user ID after all retries");
        anyhow::anyhow!("Failed to retrieve user ID: {}", e)
    })?;

    info!("Successfully retrieved user ID: {}", user_id);

    let recipient_user_id = recipient.unwrap_or(user_id);

    let prover = Arc::new(PsyCoordinatorCircuitManager::<C, D>::new_with_library(
        &proof_verifier.library,
        user_id_hash(recipient_user_id),
    ));

    let job_tracker = Arc::new(Mutex::new(WorkerJobTracker::load_from_file(worker_public_key)));

    let mut handles = Vec::new();

    for coordinator_config in &network.coordinator_configs {
        for rpc_url in &coordinator_config.rpc_url {
            let handle = tokio::spawn(run_worker(
                rpc_url.clone(),
                JobLocation::Coordinator,
                job_tracker.clone(),
                prover.clone(),
                proof_verifier.clone(),
                wallet.clone(),
                worker_public_key.clone(),
                user_id,
            ));
            handles.push(handle);
        }
    }

    for realm_config in &network.realm_configs {
        for rpc_url in &realm_config.rpc_url {
            let handle = tokio::spawn(run_worker(
                rpc_url.clone(),
                JobLocation::Realm(realm_config.id),
                job_tracker.clone(),
                prover.clone(),
                proof_verifier.clone(),
                wallet.clone(),
                worker_public_key.clone(),
                user_id,
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

pub fn user_id_hash(user_id: u64) -> QHashOut<F> {
    use plonky2::{hash::poseidon::PoseidonHash, plonk::config::Hasher};

    let hash_out = PoseidonHash::hash_no_pad(&[F::from_canonical_u64(user_id)]);
    QHashOut(HashOut { elements: hash_out.elements })
}
