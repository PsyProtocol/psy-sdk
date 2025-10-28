use std::{fs, str::FromStr};

use anyhow::Ok;
use plonky2::{field::goldilocks_field::GoldilocksField, plonk::config::PoseidonGoldilocksConfig};
use qed_common_circuit::circuits::{
    traits::qstandard::QStandardCircuit, zk_signature3::manager::SimpleQEDZKSignatureManager,
};
use qed_core::{
    config::network_constants::{GLOBAL_USER_TREE_HEIGHT, MAX_CONTRACT_STATE_TREE_HEIGHT, UPS_SESSION_PROOF_TREE_HEIGHT},
    data::qhashout::QHashOut,
};
use psy_crypto::{
    hash::traits::qhashable::QFieldHashable, signature::zk::wallet::SimpleQEDPrivateKey,
};
use qed_data::{
    qblock::cmds::deploy_contract::QBCDeployContract, qdata::contract::ContractCodeDefinition,
};
use qed_prover::dpn::circuits::cfc::DapenContractFunctionCircuit;
use qed_data::config::store_config::QEDHasher;
use qedlang_core::dpn::vm::def::DPNFunctionCircuitDefinition;

use qed_prover::local::{
    provider::{QUserRpcProvider, RpcProvider},
    request::QDeployContractRPCRequest,
};

use super::args::DeployContractArgs;

// #[cfg(feature = "is_sync")]
pub async fn run(args: DeployContractArgs) -> anyhow::Result<()> {
    tracing::info!("user cli deploying contract");
    use qed_data::config::store_config::{C, D};
    use qed_prover::session::gen_contract_deploy_and_circuits_for_functions;

    let private_key = QHashOut::<GoldilocksField>::from_str(&args.private_key)?;
    let mut wallet = SimpleQEDZKSignatureManager::<C, D>::new();

    tracing::info!("adding private key to wallet");
    let pk = wallet.add_private_key_get_info(SimpleQEDPrivateKey { private_key });
    let deployer = pk.qfhash::<QEDHasher>();

    let defs_array: Vec<DPNFunctionCircuitDefinition> =
        serde_json::from_str(&fs::read_to_string(args.contract_path)?)?;

    tracing::info!("getting contract state tree height");
    let contract_state_tree_height = MAX_CONTRACT_STATE_TREE_HEIGHT as usize;

    tracing::info!("generating circuits");
    let (_result_circuits, deploy_cmd) = gen_contract_deploy_and_circuits_for_functions::<C, D>(
        deployer,
        contract_state_tree_height as u8,
        &defs_array,
    )?;

    tracing::info!("deploying contract");
    let provider = RpcProvider::new_with_config_path(&args.rpc_config)?;
    tracing::info!("deploying contract");
    provider.deploy_contract(QDeployContractRPCRequest {
        deploy_contract: deploy_cmd,
    }).await?;
    tracing::info!("contract deployed");

    Ok(())
}
