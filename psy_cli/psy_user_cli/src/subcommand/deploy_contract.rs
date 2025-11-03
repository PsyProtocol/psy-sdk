use std::{fs, path::Path, str::FromStr};

use anyhow::Ok;
use plonky2::{
    field::goldilocks_field::GoldilocksField,
    hash::{hashing::hash_n_to_hash_no_pad, poseidon::PoseidonPermutation},
    plonk::config::PoseidonGoldilocksConfig,
};
use psy_common::data::qhashout::QHashOut;
use psy_common_circuit::circuits::{traits::qstandard::QStandardCircuit, zk_signature3::manager::SimplePsyZKSignatureManager};
use psy_config::network_constants::{GLOBAL_USER_TREE_HEIGHT, MAX_CONTRACT_STATE_TREE_HEIGHT, UPS_SESSION_PROOF_TREE_HEIGHT};
use psy_crypto::{hash::traits::qhashable::QFieldHashable, signature::zk::wallet::SimplePsyPrivateKey};
use psy_data::{
    config::store_config::{PsyHasher, C, D, F},
    qblock::cmds::deploy_contract::QBCDeployContract,
    qdata::{contract::ContractCodeDefinition, contract_uuid::ContractUUID},
};
use psy_dpn_circuit::circuits::cfc::DapenContractFunctionCircuit;
use psy_prover::{
    local::args::SignType,
    session::{gen_contract_deploy_and_circuits_for_functions, WalletSession},
};
use psy_rust_sdk::{
    provider::{QUserRpcProvider, RpcProvider},
    request::QDeployContractRPCRequest,
};
use psy_vm::dpn::vm::def::DPNFunctionCircuitDefinition;

use super::args::DeployContractArgs;

// #[cfg(feature = "is_sync")]
pub async fn run(args: DeployContractArgs) -> anyhow::Result<()> {
    tracing::info!("deploying contract");

    let psy_config = psy_config::PsyConfigGoldilocks::from_file(&args.rpc_config)?;
    let rpc_config = psy_config.get_current_network()?.clone();

    let mut wallet_session = WalletSession::new(&rpc_config).await?;

    let private_key = QHashOut::<F>::from_str(&args.private_key)?;
    let fingerprint = args
        .fingerprint
        .as_ref()
        .map(|f| -> anyhow::Result<_> { QHashOut::<F>::from_str(f).map_err(|e| anyhow::anyhow!("parse fingerprint error: {}", e)) })
        .transpose()?;

    let deployer = wallet_session
        .add_user_with_type(private_key, SignType::from(args.sign_type.clone()), fingerprint)
        .await?;

    let defs_array: Vec<DPNFunctionCircuitDefinition> = serde_json::from_str(&fs::read_to_string(args.contract_path)?)?;

    tracing::info!("getting contract state tree height");
    let contract_state_tree_height = MAX_CONTRACT_STATE_TREE_HEIGHT as usize;

    tracing::info!("generating circuits");
    let (_result_circuits, deploy_cmd) =
        gen_contract_deploy_and_circuits_for_functions::<C, D>(deployer, contract_state_tree_height as u8, &defs_array)?;
    let sign_data = deployer
        .0
        .elements
        .iter()
        .chain(deploy_cmd.function_whitelist.iter().flat_map(|f| f.0.elements.iter()))
        .cloned()
        .collect::<Vec<_>>();
    let sig_hash = QHashOut::<F>::from(hash_n_to_hash_no_pad::<GoldilocksField, PoseidonPermutation<GoldilocksField>>(&sign_data));

    let sign_proof = match args.sign_type.clone() {
        SignType::ZKSign => wallet_session.wallet.zk_sign_for_public_key(deployer, sig_hash).await?,
        SignType::SECP256K1Sign => wallet_session.wallet.zk_sign_secp256k1(deployer, sig_hash).await?,
        SignType::SoftwareDefinedSign => unimplemented!("software defined sign not supported"),
    };

    match args.output_path {
        Some(output_path) => {
            tracing::debug!("deploy cmd save to {}", output_path);
            let deploy_cmd_path = Path::new(&output_path).join("deploy.json");
            fs::write(deploy_cmd_path, serde_json::to_string_pretty(&deploy_cmd)?)?;
            let sign_proof_path = Path::new(&output_path).join("sign_proof.json");
            fs::write(sign_proof_path, serde_json::to_string_pretty(&sign_proof)?)?;
        }
        None => {
            tracing::debug!("deploy cmd: {}", serde_json::to_string_pretty(&deploy_cmd)?);
        }
    }

    if args.is_deploy {
        tracing::info!("user cli deploying contract");

        let contract_uuid = wallet_session
            .st_provider
            .deploy_contract(QDeployContractRPCRequest { deploy_contract: deploy_cmd })
            .await?;
        tracing::info!("contract deployed: {}", contract_uuid);
        tracing::info!("contract deployed: {:?}", ContractUUID::from_str(&contract_uuid)?);
    }

    Ok(())
}
