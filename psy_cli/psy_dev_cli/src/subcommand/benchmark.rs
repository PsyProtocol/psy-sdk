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

#[derive(Parser, Clone, Debug)]
pub struct BenchmarkEndCapArgs {
    #[arg(long, default_value = "config.json", help = "Path to config.json file")]
    pub rpc_config: String,

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

    #[clap(long, help = "Total end caps to process (0 means no limit)", default_value = "0")]
    pub total_end_caps: u64,

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

        // Filter by user_id range
        if user_id < self.args.start_user_id || user_id > self.args.end_user_id {
            return false;
        }

        // Filter by realm_id range
        let realm_id = self.rpc_provider.get_realm_id(user_id);
        if realm_id < self.args.start_realm_id || realm_id > self.args.end_realm_id {
            return false;
        }

        // Check if realm URL exists
        self.rpc_provider.get_realm_url(user_id).is_ok()
    }

    async fn load_endcaps_from_dir(&self) -> anyhow::Result<Vec<QSubmitEndCapRPCRequest<F>>> {
        let files = self.get_files_only().await?;
    let mut endcaps = Vec::new();

        for file in files.iter() {
            if let Some(user_id) = self.extract_user_id_from_filename(file) {
                if !self.should_include_endcap(user_id) {
                    continue;
                }

                if let Some(req) = self.load_endcap_from_file(file).await? {
                    endcaps.push(req);
                }
            }
        }

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
            let file_path = self.output_path.join(format!("user{}.json", user_id));
            let content = serde_json::to_string_pretty(&req)?;
            fs::write(&file_path, content).await?;

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
        let batch_size = self.args.concurrency_number as usize;
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
                }
                for user_id in failed_ids {
                    self.record.add_failed(user_id);
                    self.recorded_user_ids.insert(user_id);
                }
                (success_count, failed_count)
            }
            Err(_) => {
                // If batch fails, mark all as failed
                let count = batch.len() as u64;
                for endcap in batch {
                    let user_id = endcap.user_ec_input.core.new_user_leaf.user_id.to_noncanonical_u64();
                    self.record.add_failed(user_id);
                    self.recorded_user_ids.insert(user_id);
                }
                (0, count)
            }
        }
    }

    fn prepare_endcaps_for_sending(&self, mut endcaps: Vec<QSubmitEndCapRPCRequest<F>>) -> Vec<QSubmitEndCapRPCRequest<F>> {
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
        if self.args.total_end_caps > 0 && filtered.len() > self.args.total_end_caps as usize {
            filtered.truncate(self.args.total_end_caps as usize);
            info!("Limited to {} endcaps", self.args.total_end_caps);
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
        // Create parallel futures for all batches
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

        // Execute all batches in parallel
        let results = future::join_all(futures).await;

        // Process results and update records
        self.process_batch_results(results)
    }

    async fn submit_endcaps_batch_static(rpc_provider: &RpcProvider, endcaps: Vec<QSubmitEndCapRPCRequest<F>>) -> anyhow::Result<(Vec<u64>, Vec<u64>)> {
        if endcaps.is_empty() {
            return Ok((Vec::new(), Vec::new()));
        }

        let first_user_id = endcaps[0].user_ec_input.core.new_user_leaf.user_id.to_noncanonical_u64();
        let provider = rpc_provider.clone().with_user_id_owned(first_user_id);
        provider.submit_end_cap_proofs::<F>(endcaps).await
    }

    async fn execute_rounds(&mut self, batches: Vec<(u64, Vec<QSubmitEndCapRPCRequest<F>>)>) -> anyhow::Result<()> {
        for round in 1..=self.args.send_count {
            info!("\n=== Round {}/{} ===", round, self.args.send_count);

            let (round_success, round_failed) = self.execute_single_round(&batches).await;

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

        Ok(())
    }
}

pub async fn run(args: BenchmarkEndCapArgs) -> anyhow::Result<()> {
    let mut benchmark = Benchmark::new(args).await?;
    benchmark.run().await
}