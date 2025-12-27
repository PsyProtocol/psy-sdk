use std::{
    collections::{HashMap, HashSet},
    fs::{read_dir, read_to_string, File},
    io::{BufReader, BufWriter},
    path::Path,
};

use clap::Parser;
use plonky2::field::{goldilocks_field::GoldilocksField, types::PrimeField64};
use psy_common::{
    args::{ContractCallArgs, SignType},
    data::qhashout::QHashOut,
};
use psy_config::PsyConfigGoldilocks;
use psy_prover::{
    session::WalletSession,
    wallet::memory_wallet::{get_secp256k1_fingerprint, get_zk_fingerprint},
};
use psy_rust_sdk::{
    provider::{QUserRpcProvider, RpcProvider},
    request::QSubmitEndCapRPCRequest,
};
use rand::seq::SliceRandom;
use rand::thread_rng;
use serde::{Deserialize, Serialize};
use tracing::info;

type F = GoldilocksField;

#[derive(Parser, Clone, Debug)]
pub struct BenchmarkEndCapArgs {
    #[arg(long, default_value = "config.json", help = "Path to config.json file")]
    pub rpc_config: String,

    #[clap(long, help = "Realm RPC URL", default_value = "http://127.0.0.1:1338")]
    pub realm_rpc_url: String,

    #[clap(long, help = "concurrency number to send end cap", default_value = "100")]
    pub concurrency_number: u64,

    #[clap(long, help = "Start user ID", default_value = "0")]
    pub start_user_id: u64,

    #[clap(long, help = "End user ID", default_value = "99")]
    pub end_user_id: u64,

    #[clap(long, help = "Start realm ID", default_value = "0")]
    pub start_realm_id: u64,

    #[clap(long, help = "End realm ID", default_value = "99")]
    pub end_realm_id: u64,

    #[clap(long, help = "Send mode: random, seq", default_value = "random")]
    pub send_mode: String,

    #[clap(long, help = "total send count", default_value = "1")]
    pub send_count: u64,

    #[clap(long, help = "Private key path", default_value = "private_key.json")]
    pub private_key_path: String,

    #[clap(long, default_value = "zk")]
    pub sign_type: SignType,

    #[clap(long, help = "Contract call args path", default_value = "contract_call.json")]
    pub contract_call_args_path: String,

    #[clap(long, help = "Output end cap path", default_value = "end_caps")]
    pub output_path: String,

    #[clap(long, help = "Record file path to store success/failed endcaps", default_value = "endcap_records.json")]
    pub record_file: String,

    #[clap(long, help = "Use generated end caps")]
    pub is_use_generated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct EndcapRecord {
    success: Vec<u64>,
    failed: Vec<u64>,
}

impl EndcapRecord {
    fn new() -> Self {
        Self {
            success: Vec::new(),
            failed: Vec::new(),
        }
    }

    fn load_from_file(file_path: &str) -> anyhow::Result<Self> {
        let path = Path::new(file_path);
        if path.exists() {
            let file = File::open(path)?;
            let reader = BufReader::new(file);
            let record: EndcapRecord = serde_json::from_reader(reader)?;
            Ok(record)
        } else {
            Ok(Self::new())
        }
    }

    fn save_to_file(&self, file_path: &str) -> anyhow::Result<()> {
        let file = File::create(file_path)?;
        let writer = BufWriter::new(file);
        serde_json::to_writer_pretty(writer, self)?;
        Ok(())
    }

    fn all_recorded_user_ids(&self) -> HashSet<u64> {
        let mut set = HashSet::new();
        set.extend(self.success.iter());
        set.extend(self.failed.iter());
        set
    }

    fn add_success(&mut self, user_id: u64) {
        if !self.success.contains(&user_id) {
            self.success.push(user_id);
        }
    }

    fn add_failed(&mut self, user_id: u64) {
        if !self.failed.contains(&user_id) {
            self.failed.push(user_id);
        }
    }
}

fn extract_user_id_from_filename(filename: &str) -> Option<u64> {
    filename.strip_prefix("user")?.strip_suffix(".json")?.parse::<u64>().ok()
}

fn get_files_only(dir: &str) -> anyhow::Result<Vec<String>> {
    let mut files = Vec::new();
    for entry in read_dir(dir)? {
        let entry = entry?;
        if entry.metadata()?.is_file() {
            let file_name = entry.file_name();
            files.push(file_name.to_string_lossy().to_string());
        }
    }
    Ok(files)
}

fn load_endcaps_from_dir(
    dir: &Path,
    rpc_provider: &RpcProvider,
    start_user_id: u64,
    end_user_id: u64,
    start_realm_id: u64,
    end_realm_id: u64,
    recorded_user_ids: &HashSet<u64>,
) -> anyhow::Result<Vec<QSubmitEndCapRPCRequest<F>>> {
    let files = get_files_only(dir.to_str().unwrap())?;
    let mut endcaps = Vec::new();

    for file in files.iter() {
        if let Some(user_id) = extract_user_id_from_filename(file) {
            // Skip if already recorded
            if recorded_user_ids.contains(&user_id) {
                continue;
            }

            // Filter by user_id range
            if user_id < start_user_id || user_id > end_user_id {
                continue;
            }

            // Filter by realm_id range
            let realm_id = rpc_provider.get_realm_id(user_id);
            if realm_id < start_realm_id || realm_id > end_realm_id {
                continue;
            }

            // Check if realm URL exists
            if rpc_provider.get_realm_url(user_id).is_err() {
                continue;
            }

            // Load endcap file
            let file_path = dir.join(file);
            let content = read_to_string(&file_path)?;
            let req: QSubmitEndCapRPCRequest<F> = serde_json::from_str(&content)?;

            // Verify user_id matches
            let req_user_id = req.user_ec_input.core.new_user_leaf.user_id.to_noncanonical_u64();
            if req_user_id != user_id {
                continue;
            }

            endcaps.push(req);
        }
    }

    Ok(endcaps)
}



fn group_endcaps_by_realm(
    endcaps: Vec<QSubmitEndCapRPCRequest<F>>,
    rpc_provider: &RpcProvider,
) -> HashMap<u64, Vec<QSubmitEndCapRPCRequest<F>>> {
    let mut grouped: HashMap<u64, Vec<QSubmitEndCapRPCRequest<F>>> = HashMap::new();

    for endcap in endcaps {
        let user_id = endcap.user_ec_input.core.new_user_leaf.user_id.to_noncanonical_u64();
        let realm_id = rpc_provider.get_realm_id(user_id);
        grouped.entry(realm_id).or_insert_with(Vec::new).push(endcap);
    }

    grouped
}

async fn submit_endcaps_batch(
    endcaps: Vec<QSubmitEndCapRPCRequest<F>>,
    rpc_provider: &RpcProvider,
) -> anyhow::Result<(Vec<u64>, Vec<u64>)> {
    if endcaps.is_empty() {
        return Ok((Vec::new(), Vec::new()));
    }

    let first_user_id = endcaps[0].user_ec_input.core.new_user_leaf.user_id.to_noncanonical_u64();
    let provider = rpc_provider.clone().with_user_id_owned(first_user_id);

    provider.submit_end_cap_proofs::<F>(endcaps).await
}

pub async fn run(args: BenchmarkEndCapArgs) -> anyhow::Result<()> {
    // Load network configuration
    let psy_config = PsyConfigGoldilocks::from_file(&args.rpc_config)?;
    let rpc_config = psy_config.get_current_network()?.clone();
    let rpc_provider = RpcProvider::new_with_config(&rpc_config)?;

    info!("Coordinator config: {}", serde_json::to_string_pretty(&rpc_config.coordinator_configs)?);
    info!("Realm config: {}", serde_json::to_string_pretty(&rpc_config.realm_configs)?);

    // Load record file
    let mut record = EndcapRecord::load_from_file(&args.record_file)?;
    let mut recorded_user_ids = record.all_recorded_user_ids();
    info!("Loaded {} recorded endcaps ({} success, {} failed)", recorded_user_ids.len(), record.success.len(), record.failed.len());

    // Load endcaps from directory
    let output_path = Path::new(&args.output_path);
    if !output_path.exists() {
        std::fs::create_dir_all(output_path)?;
    }

    let mut endcaps = if args.is_use_generated {
        load_endcaps_from_dir(
            output_path,
            &rpc_provider,
            args.start_user_id,
            args.end_user_id,
            args.start_realm_id,
            args.end_realm_id,
            &recorded_user_ids,
        )?
    } else {
        // Generate endcaps (existing logic)
        let contract_call_args = serde_json::from_str::<Vec<ContractCallArgs>>(&read_to_string(&args.contract_call_args_path)?)?;
        let private_keys = serde_json::from_str::<Vec<QHashOut<F>>>(&read_to_string(&args.private_key_path)?)?;
        let private_keys_len = private_keys.len();

        let mut wallet_session = WalletSession::new(&rpc_config).await?;
        let fingerprint = match args.sign_type {
            SignType::ZKSign => get_zk_fingerprint(),
            SignType::SECP256K1Sign => get_secp256k1_fingerprint(),
            SignType::SoftwareDefinedDPNSign => unimplemented!("SoftwareDefinedDPNSign is not supported"),
            SignType::SoftwareDefinedPlonky2Sign => unimplemented!("SoftwareDefinedPlonky2Sign is not supported"),
        };

        let mut public_keys = Vec::with_capacity(private_keys_len);
        for private_key in private_keys.iter() {
            public_keys.push(wallet_session.add_user(*private_key, fingerprint).await?);
        }

        let mut endcaps = Vec::new();
        for public_key in public_keys.into_iter() {
            wallet_session.start_session(public_key).await?;
            wallet_session.prove_contract_call(public_key, contract_call_args.clone()).await?;
            let (user_ec_input, end_cap_proof) = wallet_session.sign(public_key, None).await?;
            let req = QSubmitEndCapRPCRequest {
                user_ec_input,
                proof: bincode::serialize(&end_cap_proof)?,
            };

            let user_id = req.user_ec_input.core.new_user_leaf.user_id.to_noncanonical_u64();
            std::fs::write(
                output_path.join(format!("user{}.json", user_id)),
                serde_json::to_string_pretty(&req)?,
            )?;

            endcaps.push(req);
        }
        endcaps
    };

    info!("Total endcaps available: {}", endcaps.len());

    // Apply send mode (random or sequential)
    if args.send_mode == "random" {
        let mut rng = thread_rng();
        endcaps.shuffle(&mut rng);
        info!("Shuffled endcaps for random mode");
    } else {
        info!("Using sequential mode");
    }

    // Filter out already recorded endcaps before grouping
    let endcaps_to_send: Vec<QSubmitEndCapRPCRequest<F>> = endcaps
        .into_iter()
        .filter(|endcap| {
            let user_id = endcap.user_ec_input.core.new_user_leaf.user_id.to_noncanonical_u64();
            !recorded_user_ids.contains(&user_id)
        })
        .collect();

    if endcaps_to_send.is_empty() {
        info!("No endcaps to send (all already recorded)");
        return Ok(());
    }

    info!("Total endcaps to send: {}", endcaps_to_send.len());

    // Group endcaps by realm
    let grouped = group_endcaps_by_realm(endcaps_to_send, &rpc_provider);
    info!("Grouped into {} realms", grouped.len());

    // Execute multiple rounds (for performance testing, same endcaps are sent multiple times)
    for round in 1..=args.send_count {
        info!("\n=== Round {}/{} ===", round, args.send_count);

        let mut round_success = 0;
        let mut round_failed = 0;

        // Process each realm's endcaps
        for (realm_id, realm_endcaps) in &grouped {
            // Process in batches based on concurrency
            let batch_size = args.concurrency_number as usize;

            for batch in realm_endcaps.chunks(batch_size) {
                match submit_endcaps_batch(batch.to_vec(), &rpc_provider).await {
                    Ok((success_ids, failed_ids)) => {
                        let success_count = success_ids.len();
                        let failed_count = failed_ids.len();
                        for user_id in success_ids {
                            record.add_success(user_id);
                            recorded_user_ids.insert(user_id);
                            round_success += 1;
                        }
                        for user_id in failed_ids {
                            record.add_failed(user_id);
                            recorded_user_ids.insert(user_id);
                            round_failed += 1;
                        }
                        info!("Realm {} batch: {} success, {} failed", realm_id, success_count, failed_count);
                    }
                    Err(e) => {
                        // If batch fails, mark all as failed
                        for endcap in batch {
                            let user_id = endcap.user_ec_input.core.new_user_leaf.user_id.to_noncanonical_u64();
                            record.add_failed(user_id);
                            recorded_user_ids.insert(user_id);
                            round_failed += 1;
                        }
                        info!("Realm {} batch failed: {}", realm_id, e);
                    }
                }

                // Save record after each batch
                record.save_to_file(&args.record_file)?;
            }
        }

        info!("Round {} completed: {} success, {} failed", round, round_success, round_failed);
    }

    info!("\n=== Final Statistics ===");
    info!("Total success: {}", record.success.len());
    info!("Total failed: {}", record.failed.len());
    info!("Record saved to: {}", args.record_file);

    Ok(())
}