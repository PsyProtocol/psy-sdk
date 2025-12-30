use std::path::Path;

use clap::Parser;
use plonky2::field::goldilocks_field::GoldilocksField;
use psy_config::PsyConfigGoldilocks;
use psy_rust_sdk::{
    provider::{QUserRpcProvider, RpcProvider},
    request::QSubmitEndCapRPCRequest,
};
use serde::{Deserialize, Serialize};

type F = GoldilocksField;

const USERS_PER_REALM: u64 = 1 << 20;
const END_CAPS_PER_REALM: u64 = 4096;
const TOTAL_REALMS: u64 = 128;
const EDGES_PER_REALM: u64 = 16;
const END_CAPS_PER_EDGE: u64 = END_CAPS_PER_REALM / EDGES_PER_REALM;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SimpleEndcapRecord {
    success: Vec<u64>,
    failed: Vec<u64>,
    total_end_caps: u64,
}

impl SimpleEndcapRecord {
    pub fn new() -> Self {
        Self {
            success: Vec::new(),
            failed: Vec::new(),
            total_end_caps: 0,
        }
    }

    pub fn load_from_file(file_path: &str) -> anyhow::Result<Self> {
        std::fs::read_to_string(file_path)
            .map_err(anyhow::Error::from)
            .and_then(|content| serde_json::from_str(&content).map_err(anyhow::Error::from))
            .or_else(|e| {
                tracing::warn!("failed to load end cap record file {}: {:?}", file_path, e);
                Ok(Self::new())
            })
    }

    pub fn save_to_file(&self, file_path: &str) -> anyhow::Result<()> {
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(file_path, content)?;
        Ok(())
    }
}

#[derive(Parser)]
pub struct EndCapSubmissionArgs {
    #[arg(long, default_value = "config.json", help = "Path to config.json file")]
    pub rpc_config: String,

    #[clap(long, help = "Realm ID", default_value = "0")]
    pub realm_id: u64,

    #[clap(long, help = "Realm Edge ID", default_value = "0")]
    pub edge_id: u64,

    #[clap(long, help = "Output end cap path", default_value = "end_caps")]
    pub output_path: String,

    #[clap(long, help = "Record file path to store success/failed endcaps", default_value = "endcap_records.json")]
    pub record_file: String,
}

fn get_all_user_ids_for_realm_edge(realm_id: u64, edge_id: u64) -> Vec<u64> {
    (0..END_CAPS_PER_EDGE)
        .map(|i| realm_id * END_CAPS_PER_REALM * 256 + edge_id * 256 * 256 + i * 256)
        .collect()
}

pub async fn run(args: EndCapSubmissionArgs) -> anyhow::Result<()> {
    let psy_config = PsyConfigGoldilocks::from_file(&args.rpc_config)?;
    let rpc_config = psy_config.get_current_network()?.clone();

    tracing::info!("coordinator config: {}", serde_json::to_string_pretty(&rpc_config.coordinator_configs)?);
    tracing::info!("realm config: {}", serde_json::to_string_pretty(&rpc_config.realm_configs)?);

    let mut record = SimpleEndcapRecord::load_from_file(&args.record_file)?;

    let rpc_provider = RpcProvider::new_with_config(&rpc_config)?;

    let path = Path::new(&args.output_path);
    if !path.exists() {
        anyhow::bail!("output path {} does not exist", args.output_path);
    }

    let mut endcaps = Vec::new();

    let user_ids = get_all_user_ids_for_realm_edge(args.realm_id, args.edge_id);
    let first_user_id = user_ids[0];

    for user_id in user_ids.iter() {
        if record.success.contains(user_id) {
            tracing::debug!("user {} has been submitted", user_id);
            continue;
        }
        let user_end_cap_file = Path::new(&args.output_path).join(format!("user{}.bin", user_id));

        let req = bincode::deserialize::<QSubmitEndCapRPCRequest<F>>(&std::fs::read(user_end_cap_file)?)?;
        endcaps.push(req);
    }

    tracing::info!("total endcap avaliable: {}", endcaps.len());

    let (success_ids, failed_ids) = rpc_provider
        .with_user_id_owned(first_user_id)
        .submit_end_cap_proofs_by_edge_id(endcaps, args.edge_id)
        .await?;
    record.success.extend(success_ids.iter().map(|id| *id));
    record.failed.extend(failed_ids.iter().map(|id| *id));
    record.total_end_caps += success_ids.len() as u64;

    record.save_to_file(&args.record_file)?;

    Ok(())
}

mod tests {
    use std::collections::HashMap;

    use psy_config::USERS_PER_REALM;
    use psy_crypto::common::user_id::get_user_id_from_registration_id;

    use crate::subcommand::end_cap_submission::{get_all_user_ids_for_realm_edge, END_CAPS_PER_REALM};

    #[test]
    fn test_user_submission_consistency() -> anyhow::Result<()> {
        let mut realm_user_id_maps = HashMap::<u64, Vec<u64>>::new();
        for registration_id in 0..1 << 19 {
            let user_id = get_user_id_from_registration_id(registration_id);
            let realm_id = user_id / USERS_PER_REALM;
            realm_user_id_maps.entry(realm_id).or_insert(Vec::new()).push(user_id);
        }

        for realm_id in 0..128 {
            let mut expected_user_ids = realm_user_id_maps
                .get(&realm_id)
                .expect(&format!("get realm id {} failed", realm_id))
                .clone();
            expected_user_ids.sort();
            let realm_user_ids = (0..END_CAPS_PER_REALM)
                .map(|i| realm_id * END_CAPS_PER_REALM * 256 + i * 256)
                .collect::<Vec<u64>>();

            assert_eq!(realm_user_ids, expected_user_ids);

            let realm_edge_all_user_ids = (0..16)
                .flat_map(|edge_id| get_all_user_ids_for_realm_edge(realm_id, edge_id))
                .collect::<Vec<u64>>();

            assert_eq!(realm_edge_all_user_ids, expected_user_ids);
        }

        Ok(())
    }
}
