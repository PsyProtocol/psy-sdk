use std::{
    fs::{read_dir, read_to_string},
    path::Path,
    sync::Arc,
};

use clap::Parser;
use futures::future::join_all;
use plonky2::field::{goldilocks_field::GoldilocksField, types::PrimeField64};
use psy_common::{
    args::{ContractCallArgs, SignType},
    data::qhashout::QHashOut,
};
use psy_config::PsyConfigGoldilocks;
use psy_data::qstore::controllers::proving_session::PsyReadLocalProvingSessionStore;
use psy_prover::{
    session::WalletSession,
    wallet::memory_wallet::{get_secp256k1_fingerprint, get_zk_fingerprint},
};
use psy_rust_sdk::{
    provider::{QUserRpcProvider, RpcProvider},
    request::QSubmitEndCapRPCRequest,
};
use tokio::sync::Semaphore;

const MAX_CONCURRENT_REQUESTS: usize = 32;

type F = GoldilocksField;

#[derive(Parser)]
pub struct MultiEndCapArgs {
    #[arg(long, default_value = "config.json", help = "Path to config.json file")]
    pub rpc_config: String,

    #[clap(long, help = "Private key path", default_value = "private_key.json")]
    pub private_key_path: String,

    #[clap(long, default_value = "zk")]
    pub sign_type: SignType,

    #[clap(long, help = "User ID start from", default_value = "0")]
    pub start_user_id: u64,

    #[clap(long, help = "User ID end at", default_value = "4294967296")]
    pub end_user_id: u64,

    #[clap(long, help = "Realm ID start from", default_value = "0")]
    pub start_realm_id: u64,

    #[clap(long, help = "Realm ID end at", default_value = "128")]
    pub end_realm_id: u64,

    #[clap(long, help = "Total end caps", default_value = "4096")]
    pub total_end_caps: u64,

    #[clap(long, help = "Contract call args path", default_value = "contract_call.json")]
    pub contract_call_args_path: String,

    #[clap(long, help = "Output end cap path", default_value = "end_caps")]
    pub output_path: String,

    #[clap(long, help = "Use generated end caps")]
    pub is_use_generated: bool,

    #[clap(long, help = "Submit end caps at the end")]
    pub is_submit_at_end: bool,
}

fn extract_user_id_from_filename(filename: &str) -> Option<u64> {
    filename.strip_prefix("user")?.strip_suffix(".bin")?.parse::<u64>().ok()
}

fn user_id_in_range(start_realm_id: u64, end_realm_id: u64, start_user_id: u64, end_user_id: u64, users_per_realm: u64, user_id: u64) -> bool {
    let realm_start_user_id = start_realm_id * users_per_realm;
    let realm_end_user_id = end_realm_id * users_per_realm;
    (realm_start_user_id <= user_id && user_id < realm_end_user_id) && (start_user_id <= user_id && user_id < end_user_id)
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

pub async fn run(args: MultiEndCapArgs) -> anyhow::Result<()> {
    let psy_config = PsyConfigGoldilocks::from_file(&args.rpc_config)?;
    let rpc_config = psy_config.get_current_network()?.clone();

    tracing::info!("coordinator config: {}", serde_json::to_string_pretty(&rpc_config.coordinator_configs)?);
    tracing::info!("realm config: {}", serde_json::to_string_pretty(&rpc_config.realm_configs)?);

    let rpc_provider = RpcProvider::new_with_config(&rpc_config)?;

    let path = Path::new(&args.output_path);
    if !path.exists() {
        std::fs::create_dir_all(path)?;
    }

    let mut endcaps = Vec::new();
    if args.is_use_generated {
        let files = get_files_only(&args.output_path)?;
        tracing::info!("files: {:?}", files);

        for file in files.iter() {
            let (user_id, req): (u64, Option<QSubmitEndCapRPCRequest<F>>) = match extract_user_id_from_filename(file) {
                Some(user_id) => (user_id, None),
                None => {
                    let req = serde_json::from_str::<QSubmitEndCapRPCRequest<F>>(&std::fs::read_to_string(path.join(file))?)?;
                    let user_id = req.user_ec_input.core.new_user_leaf.user_id.to_noncanonical_u64();
                    (user_id, Some(req))
                }
            };

            if !user_id_in_range(
                args.start_realm_id,
                args.end_realm_id,
                args.start_user_id,
                args.end_user_id,
                rpc_config.users_per_realm,
                user_id,
            ) {
                tracing::info!("user {} of realm {} is not in range", user_id, rpc_provider.get_realm_id(user_id));
                continue;
            }

            if rpc_provider.get_realm_url(user_id).is_ok() {
                let endcap_req = if let Some(req) = req {
                    req
                } else {
                    bincode::deserialize::<QSubmitEndCapRPCRequest<F>>(&std::fs::read(path.join(file))?)?
                };
                tracing::info!("push user {} end cap of realm {}", user_id, rpc_provider.get_realm_id(user_id));
                endcaps.push(endcap_req);
            } else {
                tracing::info!("user {} not supported", user_id);
            }

            if endcaps.len() >= args.total_end_caps as usize {
                break;
            }
        }
    } else {
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

        for public_key in public_keys.into_iter() {
            let user_id = wallet_session
                .user_session_mgrs
                .get(&public_key)
                .ok_or_else(|| anyhow::format_err!("user {} not found", public_key.to_string()))?
                .lps
                .get_current_user_id_64();
            if !user_id_in_range(
                args.start_realm_id,
                args.end_realm_id,
                args.start_user_id,
                args.end_user_id,
                rpc_config.users_per_realm,
                user_id,
            ) {
                tracing::info!("user {} of realm {} is not in range", user_id, rpc_provider.get_realm_id(user_id));
                continue;
            }
            tracing::info!("process user {} end cap of realm {}", user_id, rpc_provider.get_realm_id(user_id));
            wallet_session.start_session(public_key).await?;
            wallet_session.prove_contract_call(public_key, contract_call_args.clone()).await?;
            let (user_ec_input, end_cap_proof) = wallet_session.sign(public_key, None).await?;
            let req = QSubmitEndCapRPCRequest {
                user_ec_input,
                proof: bincode::serialize(&end_cap_proof)?,
            };

            std::fs::write(
                format!(
                    "{}/user{}.bin",
                    args.output_path,
                    req.user_ec_input.core.new_user_leaf.user_id.to_noncanonical_u64()
                ),
                bincode::serialize(&req)?,
            )?;

            endcaps.push(req);

            if endcaps.len() >= args.total_end_caps as usize {
                break;
            }
        }
    }

    tracing::info!("total endcap avaliable: {}", endcaps.len());

    if args.is_submit_at_end {
        let errors = submit_end_cap_proofs(endcaps, rpc_provider).await;

        if !errors.is_empty() {
            tracing::info!("total errors: {}", errors.len());
            tracing::info!("errors: {}", serde_json::to_string_pretty(&errors)?);
        } else {
            tracing::info!("All end caps generate successfully!");
        }
    }

    Ok(())
}

async fn submit_end_cap_proofs(endcaps: Vec<QSubmitEndCapRPCRequest<F>>, rpc_provider: RpcProvider) -> Vec<(u64, String)> {
    let mut errors = Vec::new();
    let mut tasks = Vec::new();

    let semaphore = Arc::new(Semaphore::new(MAX_CONCURRENT_REQUESTS));

    for end_cap in endcaps.into_iter() {
        let permit = semaphore.clone().acquire_owned().await.unwrap();

        let rpc_provider_clone = rpc_provider.clone();

        let task = tokio::spawn(async move {
            let _permit = permit;
            let user_id = end_cap.user_ec_input.core.new_user_leaf.user_id.to_noncanonical_u64();

            match rpc_provider_clone.with_user_id_owned(user_id).submit_end_cap_proof(end_cap).await {
                Ok(_) => None,
                Err(e) => Some((user_id, e.to_string())),
            }
        });

        tasks.push(task);
    }

    for task in join_all(tasks).await {
        match task {
            Ok(Some(error)) => errors.push(error),

            Ok(None) => (),

            Err(join_err) => {
                errors.push((0, format!("Task join error: {}", join_err)));
            }
        }
    }

    errors
}
