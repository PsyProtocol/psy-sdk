use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
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
use tokio::fs;
use tracing::info;

type F = GoldilocksField;

#[derive(Clone, Debug, PartialEq, Eq)]
enum FileFormat {
    Json,
    Bin,
}

impl std::str::FromStr for FileFormat {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "json" => Ok(FileFormat::Json),
            "bin" => Ok(FileFormat::Bin),
            _ => Err(format!("Invalid file format: {}. Must be 'json' or 'bin'", s)),
        }
    }
}

impl std::fmt::Display for FileFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FileFormat::Json => write!(f, "json"),
            FileFormat::Bin => write!(f, "bin"),
        }
    }
}

#[derive(Parser, Clone, Debug)]
pub struct BenchmarkEndCapArgs {
    #[arg(long, default_value = "config.json", help = "Path to config.json file")]
    pub rpc_config: String,

    #[clap(long, help = "parallel number to send end cap", default_value = "100")]
    pub parallel: u64,

    #[clap(long, help = "Start user ID")]
    pub start_user_id: Option<u64>,

    #[clap(long, help = "End user ID")]
    pub end_user_id: Option<u64>,

    #[clap(long, help = "Start realm ID")]
    pub start_realm_id: Option<u64>,

    #[clap(long, help = "End realm ID")]
    pub end_realm_id: Option<u64>,

    #[clap(long, help = "Send mode: random, seq", default_value = "random")]
    pub send_mode: String,

    #[clap(long, help = "try send max round", default_value = "1")]
    pub max_round: u64,

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

    #[clap(long, help = "Total end caps to process (0 means no limit)", default_value = "1000")]
    pub total_end_caps: u64,

    #[clap(long, help = "File format for saving/loading endcaps: json or bin", default_value = "bin", value_parser = clap::value_parser!(String))]
    pub file_format: String,

    #[clap(long, help = "Use generated end caps")]
    pub is_use_generated: bool,

    #[clap(long, help = "Submit end caps at the end")]
    pub is_submit_at_end: bool,

    #[clap(long, help = "Use batch submission (submit_end_cap_proofs) instead of individual submission (submit_end_cap_proof)", default_value = "false")]
    pub is_batch: bool,
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

    async fn load_from_file(file_path: &str) -> anyhow::Result<Self> {
        match fs::read_to_string(file_path).await {
            Ok(content) => serde_json::from_str(&content).map_err(Into::into),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self::new()),
            Err(e) => Err(e.into()),
        }
    }

    async fn save_to_file(&self, file_path: &str) -> anyhow::Result<()> {
        let content = serde_json::to_string_pretty(self)?;
        fs::write(file_path, content).await?;
        Ok(())
    }

    fn all_recorded_user_ids(&self) -> HashSet<u64> {
        // Only consider successful ones as recorded (failed ones can be retried)
        self.success.iter().copied().collect()
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

struct Benchmark {
    rpc_provider: RpcProvider,
    rpc_config: psy_config::NetworkConfig<F>,
    record: EndcapRecord,
    recorded_user_ids: HashSet<u64>,
    args: BenchmarkEndCapArgs,
    output_path: PathBuf,
}

impl Benchmark {
    async fn new(args: BenchmarkEndCapArgs) -> anyhow::Result<Self> {
    let psy_config = PsyConfigGoldilocks::from_file(&args.rpc_config)?;
    let rpc_config = psy_config.get_current_network()?.clone();
    let rpc_provider = RpcProvider::new_with_config(&rpc_config)?;
        let record = EndcapRecord::load_from_file(&args.record_file).await?;
        let recorded_user_ids = record.all_recorded_user_ids();
        let output_path = PathBuf::from(&args.output_path);

        info!("Coordinator config: {}", serde_json::to_string_pretty(&rpc_config.coordinator_configs)?);
        info!("Realm config: {}", serde_json::to_string_pretty(&rpc_config.realm_configs)?);
        info!("Loaded {} recorded endcaps ({} success, {} failed)", recorded_user_ids.len(), record.success.len(), record.failed.len());

        Ok(Self {
            rpc_provider,
            record,
            recorded_user_ids,
            args,
            output_path,
            rpc_config,
        })
    }

    fn extract_user_id_from_filename(&self, filename: &str) -> Option<u64> {
        let without_prefix = filename.strip_prefix("user")?;
        if let Some(id_str) = without_prefix.strip_suffix(".json") {
            id_str.parse::<u64>().ok()
        } else if let Some(id_str) = without_prefix.strip_suffix(".bin") {
            id_str.parse::<u64>().ok()
        } else {
            None
        }
    }

    async fn get_files_only(&self) -> anyhow::Result<Vec<String>> {
        let mut files = Vec::new();
        let mut entries = fs::read_dir(&self.output_path).await?;
        while let Some(entry) = entries.next_entry().await? {
            if entry.metadata().await?.is_file() {
                let file_name = entry.file_name();
                files.push(file_name.to_string_lossy().to_string());
            }
        }
        Ok(files)
    }

    async fn load_endcap_from_file(&self, filename: &str) -> anyhow::Result<Option<QSubmitEndCapRPCRequest<F>>> {
        let user_id = match self.extract_user_id_from_filename(filename) {
            Some(id) => id,
            None => return Ok(None),
        };

        let file_path = self.output_path.join(filename);
        let req: QSubmitEndCapRPCRequest<F> = if filename.ends_with(".bin") {
            // Load as bincode format
            let bytes = fs::read(&file_path).await?;
            bincode::deserialize(&bytes).map_err(|e| anyhow::anyhow!("Failed to deserialize bincode: {}", e))?
        } else if filename.ends_with(".json") {
            // Load as JSON format
            let content = fs::read_to_string(&file_path).await?;
            serde_json::from_str(&content).map_err(|e| anyhow::anyhow!("Failed to deserialize JSON: {}", e))?
        } else {
            return Ok(None);
        };

        // Verify user_id matches
        let req_user_id = req.user_ec_input.core.new_user_leaf.user_id.to_noncanonical_u64();
        if req_user_id != user_id {
            return Ok(None);
        }

        Ok(Some(req))
    }

    fn should_include_endcap(&self, user_id: u64) -> bool {
        // Skip if already recorded
        if self.recorded_user_ids.contains(&user_id) {
            return false;
        }

        // Filter by user_id range (skip if None)
        if let Some(start) = self.args.start_user_id {
            if user_id < start {
                return false;
            }
        }
        if let Some(end) = self.args.end_user_id {
            if user_id > end {
                return false;
            }
        }

        // Filter by realm_id range (skip if None)
        let realm_id = self.rpc_provider.get_realm_id(user_id);
        if let Some(start) = self.args.start_realm_id {
            if realm_id < start {
                return false;
            }
        }
        if let Some(end) = self.args.end_realm_id {
            if realm_id > end {
                return false;
            }
        }

        // Check if realm URL exists
        self.rpc_provider.get_realm_url(user_id).is_ok()
    }

    async fn load_endcaps_from_dir(&self) -> anyhow::Result<Vec<QSubmitEndCapRPCRequest<F>>> {
        let files = self.get_files_only().await?;
        let mut endcaps = Vec::new();
        let limit = if self.args.total_end_caps > 0 {
            self.args.total_end_caps as usize
        } else {
            usize::MAX
        };

        let limit_str = if limit == usize::MAX { "unlimited".to_string() } else { limit.to_string() };
        info!("Loading endcaps from {} files (limit: {})", files.len(), limit_str);

        for file in files.iter() {
            // Stop loading if we've reached the limit
            if endcaps.len() >= limit {
                info!("Reached total_end_caps limit ({}), stopping file loading", limit);
                break;
            }

            if let Some(user_id) = self.extract_user_id_from_filename(file) {
                if !self.should_include_endcap(user_id) {
                    continue;
                }

                if let Some(req) = self.load_endcap_from_file(file).await? {
                    endcaps.push(req);
                }
            }
        }

        info!("Loaded {} endcaps from directory", endcaps.len());
        Ok(endcaps)
    }

    async fn generate_endcaps(&self) -> anyhow::Result<Vec<QSubmitEndCapRPCRequest<F>>> {
        let contract_call_args_content = fs::read_to_string(&self.args.contract_call_args_path).await?;
        let contract_call_args = serde_json::from_str::<Vec<ContractCallArgs>>(&contract_call_args_content)?;
        let private_keys_content = fs::read_to_string(&self.args.private_key_path).await?;
        let private_keys = serde_json::from_str::<Vec<QHashOut<F>>>(&private_keys_content)?;

        let mut wallet_session = WalletSession::new(&self.rpc_config).await?;
        let fingerprint = match self.args.sign_type {
            SignType::ZKSign => get_zk_fingerprint(),
            SignType::SECP256K1Sign => get_secp256k1_fingerprint(),
            SignType::SoftwareDefinedDPNSign => unimplemented!("SoftwareDefinedDPNSign is not supported"),
            SignType::SoftwareDefinedPlonky2Sign => unimplemented!("SoftwareDefinedPlonky2Sign is not supported"),
        };
        let (left, right) = private_keys.split_at(self.args.total_end_caps as usize);
        if left.is_empty() {
            return Ok(vec![]);
        }
        let private_keys = left;

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
            let file_format = self.args.file_format.parse::<FileFormat>()
                .map_err(|e| anyhow::anyhow!("Invalid file format: {}", e))?;
            
            let file_path = match file_format {
                FileFormat::Json => self.output_path.join(format!("user{}.json", user_id)),
                FileFormat::Bin => self.output_path.join(format!("user{}.bin", user_id)),
            };

            match file_format {
                FileFormat::Json => {
                    let content = serde_json::to_string_pretty(&req)?;
                    fs::write(&file_path, content).await?;
                }
                FileFormat::Bin => {
                    let content = bincode::serialize(&req)?;
                    fs::write(&file_path, content).await?;
                }
            }

            endcaps.push(req);
        }

        Ok(endcaps)
    }

    fn group_endcaps_by_realm(&self, endcaps: Vec<QSubmitEndCapRPCRequest<F>>) -> HashMap<u64, Vec<QSubmitEndCapRPCRequest<F>>> {
        endcaps.into_iter().fold(HashMap::new(), |mut acc, endcap| {
            let user_id = endcap.user_ec_input.core.new_user_leaf.user_id.to_noncanonical_u64();
            let realm_id = self.rpc_provider.get_realm_id(user_id);
            acc.entry(realm_id).or_insert_with(Vec::new).push(endcap);
            acc
        })
    }

    fn create_batches_by_realm(&self, grouped: &HashMap<u64, Vec<QSubmitEndCapRPCRequest<F>>>) -> Vec<(u64, Vec<QSubmitEndCapRPCRequest<F>>)> {
        let batch_size = self.args.parallel as usize;
        grouped
            .iter()
            .flat_map(|(realm_id, endcaps)| {
                endcaps.chunks(batch_size).map(|batch| (*realm_id, batch.to_vec()))
            })
            .collect()
    }

    fn update_record_from_batch_result(&mut self, batch: Vec<QSubmitEndCapRPCRequest<F>>, result: anyhow::Result<(Vec<u64>, Vec<u64>)>) -> (u64, u64) {
        match result {
            Ok((success_ids, failed_ids)) => {
                let success_count = success_ids.len() as u64;
                let failed_count = failed_ids.len() as u64;
                for user_id in success_ids {
                    self.record.add_success(user_id);
                    self.recorded_user_ids.insert(user_id);
                    // Remove from failed list if it was previously failed (now succeeded)
                    self.record.failed.retain(|&id| id != user_id);
                }
                for user_id in failed_ids {
                    // Only add to failed list if not already successful
                    if !self.record.success.contains(&user_id) {
                        self.record.add_failed(user_id);
                    }
                    // Don't add to recorded_user_ids, so it can be retried in next round
                }
                (success_count, failed_count)
            }
            Err(e) => {
                // If batch fails, mark all as failed (but don't mark as recorded for retry)
                let count = batch.len() as u64;
                let user_ids: Vec<u64> = batch.iter()
                    .map(|endcap| endcap.user_ec_input.core.new_user_leaf.user_id.to_noncanonical_u64())
                    .collect();
                info!("Batch submission failed with error: {} (user_ids: {:?})", e, user_ids);
                for endcap in batch {
                    let user_id = endcap.user_ec_input.core.new_user_leaf.user_id.to_noncanonical_u64();
                    // Only add to failed list if not already successful
                    if !self.record.success.contains(&user_id) {
                        self.record.add_failed(user_id);
                    }
                    // Don't add to recorded_user_ids, so it can be retried in next round
                }
                (0, count)
            }
        }
    }

    fn prepare_endcaps_for_sending(&self, endcaps: Vec<QSubmitEndCapRPCRequest<F>>) -> Vec<QSubmitEndCapRPCRequest<F>> {
        // Filter out already recorded endcaps first
        let mut filtered: Vec<QSubmitEndCapRPCRequest<F>> = endcaps
            .into_iter()
            .filter(|endcap| {
                let user_id = endcap.user_ec_input.core.new_user_leaf.user_id.to_noncanonical_u64();
                !self.recorded_user_ids.contains(&user_id)
            })
            .collect();

        // Apply send mode (random or sequential) after filtering
        if self.args.send_mode == "random" {
            filtered.shuffle(&mut thread_rng());
            info!("Shuffled endcaps for random mode");
        } else {
            info!("Using sequential mode");
        }

        // Apply total_end_caps limit if specified
        // Note: The limit is already applied during file loading to avoid memory issues,
        // but we apply it here again as a safeguard in case filtering significantly changed the count
        if self.args.total_end_caps > 0 && filtered.len() > self.args.total_end_caps as usize {
            filtered.truncate(self.args.total_end_caps as usize);
            info!("Limited to {} endcaps after filtering", self.args.total_end_caps);
        }

        filtered
    }

    fn prepare_batches(&self, endcaps: Vec<QSubmitEndCapRPCRequest<F>>) -> Vec<(u64, Vec<QSubmitEndCapRPCRequest<F>>)> {
        let grouped = self.group_endcaps_by_realm(endcaps);
        info!("Grouped into {} realms", grouped.len());
        let batches = self.create_batches_by_realm(&grouped);
        info!("Created {} batches for parallel sending", batches.len());
        batches
    }

    fn process_batch_results(&mut self, results: Vec<(u64, Vec<QSubmitEndCapRPCRequest<F>>, anyhow::Result<(Vec<u64>, Vec<u64>)>)>) -> (u64, u64) {
        results
            .into_iter()
            .map(|(realm_id, batch, result)| {
                let (success, failed) = self.update_record_from_batch_result(batch, result);
                info!("Realm {} batch: {} success, {} failed", realm_id, success, failed);
                (success, failed)
            })
            .fold((0, 0), |(acc_s, acc_f), (s, f)| (acc_s + s, acc_f + f))
    }

    async fn execute_single_round(&mut self, batches: &[(u64, Vec<QSubmitEndCapRPCRequest<F>>)]) -> (u64, u64) {
        if self.args.is_batch {
            // Use batch submission method
            let futures: Vec<_> = batches
                .iter()
                .map(|(realm_id, batch)| {
                    let batch = batch.clone();
                    let rpc_provider = self.rpc_provider.clone();
                    let realm_id = *realm_id;
                    async move {
                        let result = Self::submit_endcaps_batch_static(&rpc_provider, batch.clone()).await;
                        (realm_id, batch, result)
                    }
                })
                .collect();

            let results = future::join_all(futures).await;
            self.process_batch_results(results)
        } else {
            // Use individual submission method
            // Flatten all batches into a single list of endcaps
            let all_endcaps: Vec<QSubmitEndCapRPCRequest<F>> = batches
                .iter()
                .flat_map(|(_, batch)| batch.iter().cloned())
                .collect();

            if all_endcaps.is_empty() {
                return (0, 0);
            }

            info!("Executing single round with {} endcaps using individual submission", all_endcaps.len());
            match Self::submit_endcaps_single_static(&self.rpc_provider, all_endcaps).await {
                Ok((success_ids, failed_ids)) => {
                    // Convert to batch format for processing
                    let success_count = success_ids.len() as u64;
                    let failed_count = failed_ids.len() as u64;

                    // Update records
                    for user_id in success_ids {
                        self.record.add_success(user_id);
                        self.recorded_user_ids.insert(user_id);
                        self.record.failed.retain(|&id| id != user_id);
                    }
                    for user_id in failed_ids {
                        if !self.record.success.contains(&user_id) {
                            self.record.add_failed(user_id);
                        }
                    }

                    (success_count, failed_count)
                }
                Err(e) => {
                    // If entire submission fails, mark all as failed
                    let count = batches.iter().map(|(_, batch)| batch.len()).sum::<usize>() as u64;
                    info!("Single round submission failed: {}", e);
                    (0, count)
                }
            }
        }
    }

    async fn submit_endcaps_batch_static(rpc_provider: &RpcProvider, endcaps: Vec<QSubmitEndCapRPCRequest<F>>) -> anyhow::Result<(Vec<u64>, Vec<u64>)> {
        if endcaps.is_empty() {
            return Ok((Vec::new(), Vec::new()));
        }

        let first_user_id = endcaps[0].user_ec_input.core.new_user_leaf.user_id.to_noncanonical_u64();
        let realm_id = rpc_provider.get_realm_id(first_user_id);
        info!("Submitting batch of {} endcaps for realm {} (first user_id: {})", endcaps.len(), realm_id, first_user_id);
        
        let provider = rpc_provider.clone().with_user_id_owned(first_user_id);
        match provider.submit_end_cap_proofs::<F>(endcaps.clone()).await {
            Ok((success_ids, failed_ids)) => {
                info!("Batch submission completed for realm {}: {} success, {} failed", realm_id, success_ids.len(), failed_ids.len());
                Ok((success_ids, failed_ids))
            }
            Err(e) => {
                let user_ids: Vec<u64> = endcaps.iter()
                    .map(|endcap| endcap.user_ec_input.core.new_user_leaf.user_id.to_noncanonical_u64())
                    .collect();
                info!("Batch submission error for realm {} (user_ids: {:?}): {}", realm_id, user_ids, e);
                Err(e)
            }
        }
    }

    async fn submit_endcaps_single_static(rpc_provider: &RpcProvider, endcaps: Vec<QSubmitEndCapRPCRequest<F>>) -> anyhow::Result<(Vec<u64>, Vec<u64>)> {
        if endcaps.is_empty() {
            return Ok((Vec::new(), Vec::new()));
        }

        info!("Submitting {} endcaps individually in parallel", endcaps.len());

        // Create parallel futures for all individual submissions
        let futures: Vec<_> = endcaps
            .iter()
            .map(|endcap| {
                let endcap = endcap.clone();
                let rpc_provider = rpc_provider.clone();
                let user_id = endcap.user_ec_input.core.new_user_leaf.user_id.to_noncanonical_u64();
                async move {
                    let provider = rpc_provider.with_user_id_owned(user_id);
                    match provider.submit_end_cap_proof::<F>(endcap.clone()).await {
                        Ok(_uuid) => Ok(user_id),
                        Err(e) => {
                            info!("Single submission failed for user_id {}: {}", user_id, e);
                            Err((user_id, e))
                        }
                    }
                }
            })
            .collect();

        // Execute all submissions in parallel
        let results = future::join_all(futures).await;

        // Collect success and failed user_ids
        let mut success_ids = Vec::new();
        let mut failed_ids = Vec::new();

        for result in results {
            match result {
                Ok(user_id) => success_ids.push(user_id),
                Err((user_id, _e)) => failed_ids.push(user_id),
            }
        }

        info!("Single submission completed: {} success, {} failed", success_ids.len(), failed_ids.len());
        Ok((success_ids, failed_ids))
    }

    fn filter_batches_by_recorded(&self, batches: Vec<(u64, Vec<QSubmitEndCapRPCRequest<F>>)>) -> Vec<(u64, Vec<QSubmitEndCapRPCRequest<F>>)> {
        batches
            .into_iter()
            .filter_map(|(realm_id, batch)| {
                let filtered: Vec<QSubmitEndCapRPCRequest<F>> = batch
                    .into_iter()
                    .filter(|endcap| {
                        let user_id = endcap.user_ec_input.core.new_user_leaf.user_id.to_noncanonical_u64();
                        !self.recorded_user_ids.contains(&user_id)
                    })
                    .collect();
                if filtered.is_empty() {
                    None
                } else {
                    Some((realm_id, filtered))
                }
            })
            .collect()
    }

    async fn execute_rounds(&mut self, mut batches: Vec<(u64, Vec<QSubmitEndCapRPCRequest<F>>)>) -> anyhow::Result<()> {
        for round in 1..=self.args.max_round {
            info!("\n=== Round {}/{} ===", round, self.args.max_round);

            // Filter out already successful endcaps before each round
            batches = self.filter_batches_by_recorded(batches);
            
            if batches.is_empty() {
                info!("No more endcaps to send (all succeeded)");
                break;
            }

            // Recreate batches with filtered endcaps (regroup by realm if needed)
            let all_remaining: Vec<QSubmitEndCapRPCRequest<F>> = batches
                .iter()
                .flat_map(|(_, batch)| batch.iter().cloned())
                .collect();
            batches = self.prepare_batches(all_remaining);

            let (round_success, round_failed) = self.execute_single_round(&batches).await;

            // Update recorded_user_ids to reflect newly successful endcaps for next round filtering
            self.recorded_user_ids = self.record.all_recorded_user_ids();

            self.record.save_to_file(&self.args.record_file).await?;
            info!("Round {} completed: {} success, {} failed", round, round_success, round_failed);
        }
        Ok(())
    }

    async fn run(&mut self) -> anyhow::Result<()> {
        // Ensure output directory exists
        fs::create_dir_all(&self.output_path).await?;

        // Load endcaps
        let endcaps = if self.args.is_use_generated {
            self.load_endcaps_from_dir().await?
        } else {
            self.generate_endcaps().await?
        };

        info!("Total endcaps available: {}", endcaps.len());

        // Only submit if is_submit_at_end is true
        if self.args.is_submit_at_end {
            // Prepare endcaps for sending
            let endcaps_to_send = self.prepare_endcaps_for_sending(endcaps);
            if endcaps_to_send.is_empty() {
                info!("No endcaps to send (all already recorded)");
                return Ok(());
            }

            info!("Total endcaps to send: {}", endcaps_to_send.len());

            // Prepare batches and execute rounds
            let batches = self.prepare_batches(endcaps_to_send);
            self.execute_rounds(batches).await?;

            // Print final statistics
            info!("\n=== Final Statistics ===");
            info!("Total success: {}", self.record.success.len());
            info!("Total failed: {}", self.record.failed.len());
            info!("Record saved to: {}", self.args.record_file);
        } else {
            info!("Skipping submission (is_submit_at_end is false)");
        }

        Ok(())
    }
}

pub async fn run(args: BenchmarkEndCapArgs) -> anyhow::Result<()> {
    let mut benchmark = Benchmark::new(args).await?;
    benchmark.run().await
}