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

pub async fn run(args: MultiEndCapArgs) -> anyhow::Result<()> {
    let psy_config = PsyConfigGoldilocks::from_file(&args.rpc_config)?;
    let rpc_config = psy_config.get_current_network()?.clone();

    println!("coordinator config: {}", serde_json::to_string_pretty(&rpc_config.coordinator_configs)?);
    println!("realm config: {}", serde_json::to_string_pretty(&rpc_config.realm_configs)?);

    let rpc_provider = RpcProvider::new_with_config(&rpc_config)?;

    let path = Path::new(&args.output_path);
    if !path.exists() {
        std::fs::create_dir_all(path)?;
    }

    let mut endcaps = Vec::new();
    if args.is_use_generated {
        let files = get_files_only(&args.output_path)?;
        println!("files: {:?}", files);

        for file in files.iter() {
            let (user_id, req): (u64, Option<QSubmitEndCapRPCRequest<F>>) = match extract_user_id_from_filename(file) {
                Some(user_id) => (user_id, None),
                None => {
                    let req = serde_json::from_str::<QSubmitEndCapRPCRequest<F>>(&std::fs::read_to_string(path.join(file))?)?;
                    let user_id = req.user_ec_input.core.new_user_leaf.user_id.to_noncanonical_u64();
                    (user_id, Some(req))
                }
            };

            if rpc_provider.get_realm_url(user_id).is_ok() {
                let endcap_req = if let Some(req) = req {
                    req
                } else {
                    serde_json::from_str::<QSubmitEndCapRPCRequest<F>>(&std::fs::read_to_string(path.join(file))?)?
                };
                println!("push user {} end cap of realm {}", user_id, rpc_provider.get_realm_id(user_id));
                endcaps.push(endcap_req);
            } else {
                println!("user {} not supported", user_id);
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
            wallet_session.start_session(public_key).await?;
            wallet_session.prove_contract_call(public_key, contract_call_args.clone()).await?;
            let (user_ec_input, end_cap_proof) = wallet_session.sign(public_key, None).await?;
            let req = QSubmitEndCapRPCRequest {
                user_ec_input,
                proof: bincode::serialize(&end_cap_proof)?,
            };

            std::fs::write(
                format!(
                    "{}/user{}.json",
                    args.output_path,
                    req.user_ec_input.core.new_user_leaf.user_id.to_noncanonical_u64()
                ),
                serde_json::to_string_pretty(&req)?,
            )?;

            endcaps.push(req);
        }
    }

    println!("total endcap avaliable: {}", endcaps.len());

    if args.is_submit_at_end {
        let mut errors = submit_end_cap_proofs(endcaps, rpc_provider).await;

        if !errors.is_empty() {
            println!("total errors: {}", errors.len());
            println!("errors: {}", serde_json::to_string_pretty(&errors)?);
        } else {
            println!("All end caps generate successfully!");
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
