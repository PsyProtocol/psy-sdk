use std::{fs::read_to_string, path::Path};

use clap::Parser;
use plonky2::field::{goldilocks_field::GoldilocksField, types::PrimeField64};
use psy_common::{
    args::{ContractCallArgs, ContractCallData, SignType},
    data::qhashout::QHashOut,
};
use psy_config::PsyConfigGoldilocks;
use psy_prover::{
    session::WalletSession,
    wallet::memory_wallet::{get_secp256k1_fingerprint, get_zk_fingerprint},
};
use psy_rust_sdk::{provider::QUserRpcProvider, request::QSubmitEndCapRPCRequest};

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

pub async fn run(args: MultiEndCapArgs) -> anyhow::Result<()> {
    let psy_config = PsyConfigGoldilocks::from_file(&args.rpc_config)?;
    let rpc_config = psy_config.get_current_network()?.clone();

    let path = Path::new(&args.output_path);
    if !path.exists() {
        std::fs::create_dir_all(path)?;
    }

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

    let mut endcaps = Vec::with_capacity(private_keys_len);
    for public_key in public_keys.into_iter() {
        let req = if args.is_use_generated {
            let user_id = wallet_session.st_provider.get_user_ids_for_public_key(public_key).await?[0];
            serde_json::from_str(&std::fs::read_to_string(format!("{}/user{}.json", args.output_path, user_id))?)?
        } else {
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
            req
        };
        endcaps.push(req);
    }

    if args.is_submit_at_end {
        let rpc_provider = wallet_session.st_provider.clone();
        let mut errors = Vec::with_capacity(private_keys_len);
        for end_cap in endcaps.into_iter() {
            let user_id = end_cap.user_ec_input.core.new_user_leaf.user_id.to_noncanonical_u64();
            match rpc_provider.with_user_id_owned(user_id).submit_end_cap_proof(end_cap).await {
                Ok(_) => {}
                Err(e) => {
                    errors.push((user_id, e.to_string()));
                }
            }
        }
        if !errors.is_empty() {
            println!("errors: {:?}", errors);
        } else {
            println!("All end caps generate successfully!");
        }
    }

    Ok(())
}
