mod transfer;
mod transfer_multi;

use std::{
    path::Path,
    thread,
    time::{Duration, Instant},
};

use anyhow::Result;
use plonky2::field::goldilocks_field::GoldilocksField;
use qed_data::traits::qdatastore::qmetadata::QMetaDataStoreReaderSync;
use qed_prover::{
    local::provider::{QUserRpcProvider, RpcConfig},
    session::WalletSession,
};
use tracing::{error, info};

use crate::subcommand::StressTestArgs;

type F = GoldilocksField;

pub async fn run(args: StressTestArgs) -> Result<()> {
    info!(
        "🚀 Starting stress test with {} concurrent tasks",
        args.concurrent_tasks
    );

    match args.task_type.as_str() {
        "transfer" => transfer::run(args).await,
        "transfer_multi" => transfer_multi::run(args).await,
        _ => {
            error!("❌ Unsupported task type: {}", args.task_type);
            anyhow::bail!("Unsupported task type: {}", args.task_type);
        }
    }
}

pub(crate) fn load_rpc_config(config_path: &str) -> Result<RpcConfig> {
    // Try to find config file in current directory or relative to executable location
    let config_file_path = if Path::new(config_path).exists() {
        Path::new(config_path).to_path_buf()
    } else {
        // If not in current directory, try parent directory
        let parent_config = Path::new("../").join(config_path);
        if parent_config.exists() {
            parent_config
        } else {
            return Err(anyhow::format_err!(
                "Config file not found: {}. Please ensure config.json exists in current directory or parent directory.",
                config_path
            ));
        }
    };

    // Load network configuration
    let config_str = std::fs::read_to_string(&config_file_path)?;
    let json_value: serde_json::Value = serde_json::from_str(&config_str)?;
    let rpc_config: RpcConfig = serde_json::from_value(json_value["network"].clone())?;

    Ok(rpc_config)
}

pub(crate) fn wait_for_new_block(wallet_session: &mut WalletSession, offset: u64) -> Result<bool> {
    let starting_checkpoint = wallet_session
        .st_provider
        .get_latest_l2_block_state()?
        .checkpoint_id;
    info!("current checkpoint: {}", starting_checkpoint);
    let timeout_duration = Duration::from_secs(300);
    let interval = Duration::from_secs(3);
    let start_time = Instant::now();
    let mut last_checkpoint = starting_checkpoint;
    wallet_session.st_provider.produce_block::<F>()?;
    loop {
        let checkpoint = wallet_session
            .st_provider
            .get_latest_l2_block_state()?
            .checkpoint_id;
        info!("get latest checkpoint: {}", checkpoint);
        let duration = start_time.elapsed();
        if checkpoint >= starting_checkpoint + offset {
            info!(
                "🔄 Wait {} seconds for finalizing block",
                duration.as_secs()
            );
            return Ok(true);
        }
        if checkpoint != last_checkpoint {
            info!("new checkpoint: {}", checkpoint);
            last_checkpoint = checkpoint;
            wallet_session.st_provider.produce_block::<F>()?;
        }
        if duration > timeout_duration {
            return Ok(false);
        }
        thread::sleep(interval);
    }
}
