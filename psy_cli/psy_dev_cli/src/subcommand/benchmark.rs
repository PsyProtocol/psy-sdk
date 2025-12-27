use std::{
    collections::{HashMap, HashSet},
    fs::{read_dir, read_to_string, File},
    io::{BufReader, BufWriter},
    path::Path,
};

use clap::Parser;
use futures::future;
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
use rand::{seq::SliceRandom, thread_rng};
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

impl Default for EndcapRecord {
    fn default() -> Self {
        Self::new()
    }
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
            serde_json::from_reader(reader).map_err(Into::into)
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
        self.success.iter().chain(self.failed.iter()).copied().collect()
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

fn load_endcap_from_file(dir: &Path, filename: &str) -> anyhow::Result<Option<QSubmitEndCapRPCRequest<F>>> {
    let user_id = match extract_user_id_from_filename(filename) {
        Some(id) => id,
        None => return Ok(None),
    };

    let file_path = dir.join(filename);
    let content = read_to_string(&file_path)?;
    let req: QSubmitEndCapRPCRequest<F> = serde_json::from_str(&content)?;

    // Verify user_id matches
    let req_user_id = req.user_ec_input.core.new_user_leaf.user_id.to_noncanonical_u64();
    if req_user_id != user_id {
        return Ok(None);
    }

    Ok(Some(req))
}

fn should_include_endcap(
    user_id: u64,
    rpc_provider: &RpcProvider,
    start_user_id: u64,
    end_user_id: u64,
    start_realm_id: u64,
    end_realm_id: u64,
    recorded_user_ids: &HashSet<u64>,
) -> bool {
    // Skip if already recorded
    if recorded_user_ids.contains(&user_id) {
        return false;
    }

    // Filter by user_id range
    if user_id < start_user_id || user_id > end_user_id {
        return false;
    }

    // Filter by realm_id range
    let realm_id = rpc_provider.get_realm_id(user_id);
    if realm_id < start_realm_id || realm_id > end_realm_id {
        return false;
    }

    // Check if realm URL exists
    rpc_provider.get_realm_url(user_id).is_ok()
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
            if !should_include_endcap(user_id, rpc_provider, start_user_id, end_user_id, start_realm_id, end_realm_id, recorded_user_ids) {
                continue;
            }

            if let Some(req) = load_endcap_from_file(dir, file)? {
                endcaps.push(req);
            }
        }
    }

    Ok(endcaps)
}

fn group_endcaps_by_realm(
    endcaps: Vec<QSubmitEndCapRPCRequest<F>>,
    rpc_provider: &RpcProvider,
) -> HashMap<u64, Vec<QSubmitEndCapRPCRequest<F>>> {
    endcaps.into_iter().fold(HashMap::new(), |mut acc, endcap| {
        let user_id = endcap.user_ec_input.core.new_user_leaf.user_id.to_noncanonical_u64();
        let realm_id = rpc_provider.get_realm_id(user_id);
        acc.entry(realm_id).or_insert_with(Vec::new).push(endcap);
        acc
    })
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

fn create_batches_by_realm(
    grouped: &HashMap<u64, Vec<QSubmitEndCapRPCRequest<F>>>,
    batch_size: usize,
) -> Vec<(u64, Vec<QSubmitEndCapRPCRequest<F>>)> {
    grouped
        .iter()
        .flat_map(|(realm_id, endcaps)| {
            endcaps.chunks(batch_size).map(|batch| (*realm_id, batch.to_vec()))
        })
        .collect()
}

fn update_record_from_batch_result(
    batch: Vec<QSubmitEndCapRPCRequest<F>>,
    result: anyhow::Result<(Vec<u64>, Vec<u64>)>,
    record: &mut EndcapRecord,
    recorded_user_ids: &mut HashSet<u64>,
) -> (u64, u64) {
    match result {
        Ok((success_ids, failed_ids)) => {
            let success_count = success_ids.len() as u64;
            let failed_count = failed_ids.len() as u64;
            for user_id in success_ids {
                record.add_success(user_id);
                recorded_user_ids.insert(user_id);
            }
            for user_id in failed_ids {
                record.add_failed(user_id);
                recorded_user_ids.insert(user_id);
            }
            (success_count, failed_count)
        }
        Err(_) => {
            // If batch fails, mark all as failed
            let count = batch.len() as u64;
            for endcap in batch {
                let user_id = endcap.user_ec_input.core.new_user_leaf.user_id.to_noncanonical_u64();
                record.add_failed(user_id);
                recorded_user_ids.insert(user_id);
            }
            (0, count)
        }
    }
}

async fn generate_endcaps(
    args: &BenchmarkEndCapArgs,
    rpc_config: &psy_config::NetworkConfig<F>,
    output_path: &Path,
) -> anyhow::Result<Vec<QSubmitEndCapRPCRequest<F>>> {
    let contract_call_args = serde_json::from_str::<Vec<ContractCallArgs>>(&read_to_string(&args.contract_call_args_path)?)?;
    let private_keys = serde_json::from_str::<Vec<QHashOut<F>>>(&read_to_string(&args.private_key_path)?)?;

    let mut wallet_session = WalletSession::new(rpc_config).await?;
    let fingerprint = match args.sign_type {
        SignType::ZKSign => get_zk_fingerprint(),
        SignType::SECP256K1Sign => get_secp256k1_fingerprint(),
        SignType::SoftwareDefinedDPNSign => unimplemented!("SoftwareDefinedDPNSign is not supported"),
        SignType::SoftwareDefinedPlonky2Sign => unimplemented!("SoftwareDefinedPlonky2Sign is not supported"),
    };

    let mut public_keys = Vec::with_capacity(private_keys.len());
    for private_key in private_keys.iter() {
        public_keys.push(wallet_session.add_user(*private_key, fingerprint).await?);
    }

    let mut endcaps = Vec::new();
    for public_key in public_keys {
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

    Ok(endcaps)
}

fn prepare_endcaps_for_sending(
    mut endcaps: Vec<QSubmitEndCapRPCRequest<F>>,
    send_mode: &str,
    recorded_user_ids: &HashSet<u64>,
) -> anyhow::Result<Vec<QSubmitEndCapRPCRequest<F>>> {
    // Apply send mode (random or sequential)
    if send_mode == "random" {
        endcaps.shuffle(&mut thread_rng());
        info!("Shuffled endcaps for random mode");
    } else {
        info!("Using sequential mode");
    }

    // Filter out already recorded endcaps
    let endcaps_to_send: Vec<QSubmitEndCapRPCRequest<F>> = endcaps
        .into_iter()
        .filter(|endcap| {
            let user_id = endcap.user_ec_input.core.new_user_leaf.user_id.to_noncanonical_u64();
            !recorded_user_ids.contains(&user_id)
        })
        .collect();

    info!("Total endcaps to send: {}", endcaps_to_send.len());
    Ok(endcaps_to_send)
}

fn prepare_batches(
    endcaps: Vec<QSubmitEndCapRPCRequest<F>>,
    rpc_provider: &RpcProvider,
    batch_size: usize,
) -> Vec<(u64, Vec<QSubmitEndCapRPCRequest<F>>)> {
    let grouped = group_endcaps_by_realm(endcaps, rpc_provider);
    info!("Grouped into {} realms", grouped.len());
    let batches = create_batches_by_realm(&grouped, batch_size);
    info!("Created {} batches for parallel sending", batches.len());
    batches
}

async fn execute_single_round(
    batches: &[(u64, Vec<QSubmitEndCapRPCRequest<F>>)],
    rpc_provider: &RpcProvider,
    record: &mut EndcapRecord,
    recorded_user_ids: &mut HashSet<u64>,
) -> (u64, u64) {
    // Create parallel futures for all batches
    let futures: Vec<_> = batches
        .iter()
        .map(|(realm_id, batch)| {
            let batch = batch.clone();
            let rpc_provider = rpc_provider.clone();
            let realm_id = *realm_id;
            async move { (realm_id, batch.clone(), submit_endcaps_batch(batch, &rpc_provider).await) }
        })
        .collect();

    // Execute all batches in parallel
    let results = future::join_all(futures).await;

    // Process results and update records
    process_batch_results(results, record, recorded_user_ids)
}

async fn execute_rounds(
    batches: Vec<(u64, Vec<QSubmitEndCapRPCRequest<F>>)>,
    send_count: u64,
    rpc_provider: &RpcProvider,
    record: &mut EndcapRecord,
    recorded_user_ids: &mut HashSet<u64>,
    record_file: &str,
) -> anyhow::Result<()> {
    for round in 1..=send_count {
        info!("\n=== Round {}/{} ===", round, send_count);

        let (round_success, round_failed) = execute_single_round(
            &batches,
            rpc_provider,
            record,
            recorded_user_ids,
        ).await;

        record.save_to_file(record_file)?;
        info!("Round {} completed: {} success, {} failed", round, round_success, round_failed);
    }
    Ok(())
}

fn process_batch_results(
    results: Vec<(u64, Vec<QSubmitEndCapRPCRequest<F>>, anyhow::Result<(Vec<u64>, Vec<u64>)>)>,
    record: &mut EndcapRecord,
    recorded_user_ids: &mut HashSet<u64>,
) -> (u64, u64) {
    results
        .into_iter()
        .map(|(realm_id, batch, result)| {
            let (success, failed) = update_record_from_batch_result(batch, result, record, recorded_user_ids);
            info!("Realm {} batch: {} success, {} failed", realm_id, success, failed);
            (success, failed)
        })
        .fold((0, 0), |(acc_s, acc_f), (s, f)| (acc_s + s, acc_f + f))
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

    let endcaps = if args.is_use_generated {
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
        generate_endcaps(&args, &rpc_config, output_path).await?
    };

    info!("Total endcaps available: {}", endcaps.len());

    let endcaps_to_send = prepare_endcaps_for_sending(endcaps, &args.send_mode, &recorded_user_ids)?;
    if endcaps_to_send.is_empty() {
        info!("No endcaps to send (all already recorded)");
        return Ok(());
    }

    let batches = prepare_batches(endcaps_to_send, &rpc_provider, args.concurrency_number as usize);
    execute_rounds(batches, args.send_count, &rpc_provider, &mut record, &mut recorded_user_ids, &args.record_file).await?;

    info!("\n=== Final Statistics ===");
    info!("Total success: {}", record.success.len());
    info!("Total failed: {}", record.failed.len());
    info!("Record saved to: {}", args.record_file);

    Ok(())
}