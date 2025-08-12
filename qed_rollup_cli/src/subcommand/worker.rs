use qed_node::worker::run_worker;
use qed_core::data::qhashout::QHashOut;
use qed_core::config::network_constants::DEFAULT_WORKER_PUBLIC_KEY;
use plonky2::field::goldilocks_field::GoldilocksField;
use tracing::info;
use std::str::FromStr;

pub async fn run(edge_urls: Vec<String>, public_key: Option<String>) -> anyhow::Result<()> {
    info!("Worker starting...");
    info!("Worker args: {:?}", edge_urls);

    // Parse public key if provided
    let worker_public_key = if let Some(key_str) = public_key {
        QHashOut::<GoldilocksField>::from_str(&key_str)
            .map_err(|e| anyhow::format_err!("Failed to parse public key: {}", e))?
    } else {
        DEFAULT_WORKER_PUBLIC_KEY
    };

    info!("Using worker public key: {:?}", worker_public_key);
    run_worker(edge_urls, worker_public_key).await?;
    let _ = tokio::signal::ctrl_c().await;
    info!("Worker exit.");
    Ok(())
}
