use std::{
    path::Path,
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        mpsc, Arc,
    },
    thread,
    time::{Duration, Instant},
};

use anyhow::Result;
use plonky2::field::goldilocks_field::GoldilocksField;
use qed_core::data::qhashout::QHashOut;
use qed_crypto::hash::traits::qhashable::QFieldHashable as _;
use qed_data::{
    config::store_config::QEDHasher, traits::qdatastore::qmetadata::QMetaDataStoreReaderSync,
};
use serde_json;
use tracing::{error, info, warn};

use crate::subcommand::StressTestArgs;
use qed_prover::{
    local::{
        args::ContractCallArgs,
        provider::{QUserRpcProvider, RpcConfig},
    },
    session::session::WalletSession,
};

type F = GoldilocksField;

pub async fn run(args: StressTestArgs) -> Result<()> {
    info!(
        "🚀 Starting stress test with {} concurrent tasks",
        args.concurrent_tasks
    );

    match args.task_type.as_str() {
        "transfer" => run_transfer_stress_test(args).await,
        _ => {
            error!("❌ Unsupported task type: {}", args.task_type);
            anyhow::bail!("Unsupported task type: {}", args.task_type);
        }
    }
}

#[derive(Clone)]
struct TransferTaskConfig {
    pub rpc_config: RpcConfig,
}

#[derive(Default)]
struct StressTestStats {
    transactions_sent: AtomicU64,
    transactions_successful: AtomicU64,
    transactions_failed: AtomicU64,
    total_duration_ms: AtomicU64,
    min_duration_ms: AtomicU64,
    max_duration_ms: AtomicU64,
}

impl StressTestStats {
    fn new() -> Self {
        Self {
            transactions_sent: AtomicU64::new(0),
            transactions_successful: AtomicU64::new(0),
            transactions_failed: AtomicU64::new(0),
            total_duration_ms: AtomicU64::new(0),
            min_duration_ms: AtomicU64::new(u64::MAX),
            max_duration_ms: AtomicU64::new(0),
        }
    }

    fn record_transaction(&self, success: bool, duration_ms: u64) {
        self.transactions_sent.fetch_add(1, Ordering::Relaxed);

        if success {
            self.transactions_successful.fetch_add(1, Ordering::Relaxed);
        } else {
            self.transactions_failed.fetch_add(1, Ordering::Relaxed);
        }

        self.total_duration_ms
            .fetch_add(duration_ms, Ordering::Relaxed);

        // Update min duration
        let mut current_min = self.min_duration_ms.load(Ordering::Relaxed);
        while duration_ms < current_min {
            match self.min_duration_ms.compare_exchange_weak(
                current_min,
                duration_ms,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(x) => current_min = x,
            }
        }

        // Update max duration
        let mut current_max = self.max_duration_ms.load(Ordering::Relaxed);
        while duration_ms > current_max {
            match self.max_duration_ms.compare_exchange_weak(
                current_max,
                duration_ms,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(x) => current_max = x,
            }
        }
    }

    fn get_report(&self, total_elapsed_ms: u64) -> StressTestReport {
        let sent = self.transactions_sent.load(Ordering::Relaxed);
        let successful = self.transactions_successful.load(Ordering::Relaxed);
        let failed = self.transactions_failed.load(Ordering::Relaxed);
        let total_duration = self.total_duration_ms.load(Ordering::Relaxed);
        let min_duration = self.min_duration_ms.load(Ordering::Relaxed);
        let max_duration = self.max_duration_ms.load(Ordering::Relaxed);

        StressTestReport {
            total_transactions: sent,
            successful_transactions: successful,
            failed_transactions: failed,
            success_rate: if sent > 0 {
                successful as f64 / sent as f64
            } else {
                0.0
            },
            tps: if total_elapsed_ms > 0 {
                sent as f64 / (total_elapsed_ms as f64 / 1000.0)
            } else {
                0.0
            },
            avg_duration_ms: if sent > 0 { total_duration / sent } else { 0 },
            min_duration_ms: if min_duration == u64::MAX {
                0
            } else {
                min_duration
            },
            max_duration_ms: max_duration,
            total_elapsed_ms,
        }
    }
}

struct StressTestReport {
    total_transactions: u64,
    successful_transactions: u64,
    failed_transactions: u64,
    success_rate: f64,
    tps: f64,
    avg_duration_ms: u64,
    min_duration_ms: u64,
    max_duration_ms: u64,
    total_elapsed_ms: u64,
}

impl StressTestReport {
    fn print(&self) {
        println!("\n📊 ===== QED STRESS TEST REPORT =====");
        println!("🎯 Test Scenario: Full User Lifecycle (Register → Mint → Transfer → Claim)");
        println!(
            "⏱️  Total Duration: {:.2} seconds",
            self.total_elapsed_ms as f64 / 1000.0
        );
        println!("📤 Total Scenarios Completed: {}", self.total_transactions);
        println!("✅ Successful Scenarios: {}", self.successful_transactions);
        println!("❌ Failed Scenarios: {}", self.failed_transactions);
        println!("📈 Success Rate: {:.2}%", self.success_rate * 100.0);
        println!("🔄 SPS (Scenarios Per Second): {:.2}", self.tps);
        println!("⏱️  Average Scenario Duration: {} ms", self.avg_duration_ms);
        println!("⚡ Min Scenario Duration: {} ms", self.min_duration_ms);
        println!("🐌 Max Scenario Duration: {} ms", self.max_duration_ms);
        println!("🎭 User Operations Performed:");
        println!("   🔑 New Users Generated: {}", self.total_transactions * 2);
        println!("   🪙 Mint Operations: {}", self.successful_transactions);
        println!(
            "   💸 Transfer Operations: {}",
            self.successful_transactions
        );
        println!("   🎁 Claim Operations: {}", self.successful_transactions);
        println!(
            "   🔄 Blocks Produced: {}",
            self.successful_transactions * 4
        );
        println!("========================================\n");
    }
}

async fn run_transfer_stress_test(args: StressTestArgs) -> Result<()> {
    let config = load_config(&args.config)?;

    let stats = Arc::new(StressTestStats::new());
    let should_stop = Arc::new(AtomicBool::new(false));
    let tasks_counter = Arc::new(AtomicU64::new(0));

    let start_time = Instant::now();

    let (tx, rx) = mpsc::channel();
    let mut handles = Vec::new();

    info!("🔄 Starting {} concurrent tasks", args.concurrent_tasks);
    for task_id in 0..args.concurrent_tasks {
        let config_clone = config.clone();
        let stats_clone = stats.clone();
        let should_stop_clone = should_stop.clone();
        let tx_clone = tx.clone();

        info!("🔄 Starting task {}", task_id);
        let task_counter = tasks_counter.clone();
        let handle = thread::spawn(move || {
            let result = run_transfer_task_sync(
                task_id,
                config_clone,
                stats_clone,
                should_stop_clone,
                task_counter,
            );

            // Send completion signal
            let _ = tx_clone.send((task_id, result));
        });

        handles.push(handle);
    }

    // Drop the sender
    drop(tx);

    // Handle max task limit
    let max_task = args.max_task;
    let should_stop_groups = should_stop.clone();

    if let Some(max) = max_task {
        let tasks_counter = tasks_counter.clone();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_secs(1)).await;
                if tasks_counter.load(Ordering::Relaxed) >= max {
                    info!("🎯 Max tasks ({}) reached, stopping all tasks...", max);
                    should_stop_groups.store(true, Ordering::Relaxed);
                    break;
                }
            }
        });
    }

    // Spawn stats reporter
    let stats_for_reporter = stats.clone();
    let should_stop_for_reporter = should_stop.clone();
    let reporter_handle = tokio::spawn(async move {
        let mut last_sent = 0u64;

        while !should_stop_for_reporter.load(Ordering::Relaxed) {
            tokio::time::sleep(Duration::from_secs(10)).await;

            let current_sent = stats_for_reporter.transactions_sent.load(Ordering::Relaxed);
            let current_successful = stats_for_reporter
                .transactions_successful
                .load(Ordering::Relaxed);
            let current_failed = stats_for_reporter
                .transactions_failed
                .load(Ordering::Relaxed);

            let rate = if current_sent > last_sent {
                (current_sent - last_sent) as f64 / 5.0
            } else {
                0.0
            };

            info!(
                "📊 Progress: {} sent, {} successful, {} failed, {:.1} TPS (last 5s)",
                current_sent, current_successful, current_failed, rate
            );

            last_sent = current_sent;
        }
    });

    // Wait for user input to stop (if no max task specified)
    if args.max_task.is_none() {
        info!("⏳ Stress test running indefinitely. Press Ctrl+C to stop...");
        tokio::signal::ctrl_c().await?;
        should_stop.store(true, Ordering::Relaxed);
        info!("🛑 Stopping stress test...");
    }

    // Wait for all tasks to complete
    let mut completed_tasks = 0;
    let total_tasks = args.concurrent_tasks;

    // Use non-blocking method to check task completion status
    while completed_tasks < total_tasks && !should_stop.load(Ordering::Relaxed) {
        match rx.try_recv() {
            Ok((task_id, result)) => {
                completed_tasks += 1;
                if let Err(e) = result {
                    info!("Task {} completed with error: {:?}", task_id, e);
                } else {
                    info!("Task {} completed successfully", task_id);
                }
            }
            Err(mpsc::TryRecvError::Empty) => {
                // No tasks completed, sleep briefly
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
            Err(mpsc::TryRecvError::Disconnected) => {
                // All senders closed, meaning all tasks completed
                break;
            }
        }
    }

    // Ensure all threads are completed
    for handle in handles {
        let _ = handle.join();
    }

    // Stop reporter
    reporter_handle.abort();

    let total_elapsed = start_time.elapsed().as_millis() as u64;
    let final_groups_completed = tasks_counter.load(Ordering::Relaxed);
    let report = stats.get_report(total_elapsed);

    info!(
        "🎯 Total completed transaction tasks: {}",
        final_groups_completed
    );
    report.print();

    Ok(())
}

fn run_transfer_task_sync(
    task_id: usize,
    config: TransferTaskConfig,
    stats: Arc<StressTestStats>,
    should_stop: Arc<AtomicBool>,
    task_counter: Arc<AtomicU64>,
) -> Result<()> {
    info!("🎯 Starting transfer task {}", task_id);

    // Create wallet session, handle errors
    let mut wallet_session = match WalletSession::new(&config.rpc_config) {
        Ok(ws) => ws,
        Err(e) => {
            error!(
                "Failed to create wallet session for task {}: {:?}",
                task_id, e
            );
            return Err(e);
        }
    };

    let mut transaction_count = 0u64;

    while !should_stop.load(Ordering::Relaxed) {
        let start = Instant::now();
        task_counter.fetch_add(1, Ordering::Relaxed);

        let success = match execute_transfer_transaction_sync(
            &mut wallet_session,
            task_id,
            transaction_count,
        ) {
            Ok(_) => {
                info!(
                    "✅ Task {} transaction {} completed",
                    task_id, transaction_count
                );
                true
            }
            Err(e) => {
                warn!(
                    "❌ Task {} transaction {} failed: {:?}",
                    task_id, transaction_count, e
                );
                false
            }
        };

        let duration = start.elapsed().as_millis() as u64;
        stats.record_transaction(success, duration);
        transaction_count += 1;
    }

    info!(
        "🏁 Task {} completed {} transactions",
        task_id, transaction_count
    );
    Ok(())
}

fn execute_transfer_transaction_sync(
    wallet_session: &mut WalletSession,
    task_id: usize,
    transaction_count: u64,
) -> Result<()> {
    info!(
        "🎯 Executing complete test scenario - Task {}, Transaction {}",
        task_id, transaction_count
    );

    let private_key_from = QHashOut::<GoldilocksField>::rand();
    let private_key_to = QHashOut::<GoldilocksField>::rand();

    info!("🔑 Task {} - Registering user_from and user_to", task_id);
    let pk_hash_from = wallet_session.register_user(private_key_from)?;
    let pk_hash_to = wallet_session.register_user(private_key_to)?;
    println!("pk_hash_from: {}", pk_hash_from);
    println!("pk_hash_to: {}", pk_hash_to);

    if !wait_for_new_block(wallet_session, 2)? {
        return Err(anyhow::format_err!(
            "register user timeout waiting for checkpoint"
        ));
    }
    info!("🔑 Task {} - Registered user_from and user_to", task_id);

    wallet_session.add_user(private_key_from)?;
    wallet_session.add_user(private_key_to)?;

    // let user_id_to = wallet_session.st_provider.get_user_id(private_key_to)?;
    let pk_info_from = wallet_session.wallet.get_zk_pk_info(private_key_from)?;
    let pk_hash_from = pk_info_from.qfhash::<QEDHasher>();
    // println!("pk_hash_from: {}", pk_hash_from);
    let user_id_from = wallet_session.st_provider.get_user_id(pk_hash_from)?;
    info!("👥 Task {} - User_id_from: {}", task_id, user_id_from);
    let pk_info_to = wallet_session.wallet.get_zk_pk_info(private_key_to)?;
    let pk_hash_to = pk_info_to.qfhash::<QEDHasher>();
    let user_id_to = wallet_session.st_provider.get_user_id(pk_hash_to)?;
    info!("👥 Task {} - User_id_to: {}", task_id, user_id_to);

    let mint_amount = 1000u64;
    info!(
        "🪙 Task {} - user_from minting {} tokens",
        task_id, mint_amount
    );
    wallet_session.exec_contract_call(
        pk_hash_from,
        vec![ContractCallArgs {
            contract_id: 0,
            method_name: "simple_mint".to_string(),
            inputs: vec![mint_amount],
        }],
    )?;
    info!("🪙 Task {} - Waiting for new block after mint", task_id);
    if !wait_for_new_block(wallet_session, 1)? {
        return Err(anyhow::format_err!("mint timeout waiting for checkpoint"));
    }

    let transfer_amount = 10u64;
    info!(
        "💸 Task {} - user_from transferring {} tokens to user_to",
        task_id, transfer_amount
    );

    wallet_session.exec_contract_call(
        pk_hash_from,
        vec![ContractCallArgs {
            contract_id: 0,
            method_name: "simple_transfer".to_string(),
            inputs: vec![user_id_to, transfer_amount],
        }],
    )?;

    info!("💸 Task {} - Waiting for new block after transfer", task_id);
    if !wait_for_new_block(wallet_session, 1)? {
        return Err(anyhow::format_err!(
            "transfer timeout waiting for checkpoint"
        ));
    }

    info!(
        "🎁 Task {} - user_to claiming tokens from user_from",
        task_id
    );
    wallet_session.exec_contract_call(
        pk_hash_to,
        vec![ContractCallArgs {
            contract_id: 0,
            method_name: "simple_claim".to_string(),
            inputs: vec![user_id_from],
        }],
    )?;
    info!("🎁 Task {} - Waiting for new block after claim", task_id);
    if !wait_for_new_block(wallet_session, 1)? {
        return Err(anyhow::format_err!("claim timeout waiting for checkpoint"));
    }

    info!(
        "✅ Task {} - Scenario completed successfully: mint({}) -> transfer({}) -> claim",
        task_id, mint_amount, transfer_amount
    );

    Ok(())
}

fn wait_for_new_block(wallet_session: &mut WalletSession, offset: u64) -> Result<bool> {
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

fn load_config(config_path: &str) -> Result<TransferTaskConfig> {
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

    Ok(TransferTaskConfig { rpc_config })
}
