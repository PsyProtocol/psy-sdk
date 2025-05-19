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
use qed_crypto::{
    hash::traits::qhashable::QFieldHashable, signature::zk::wallet::SimpleQEDPrivateKey,
};
use qed_data::{
    qblock::cmds::deploy_contract::QBCDeployContract, qdata::contract::ContractCodeDefinition,
};
use qed_prover::dpn::{
    circuits::cfc::DapenContractFunctionCircuit, data::dapen_fc_to_cfc_code_definition,
};
use qed_store::config::store_config::QEDHasher;
use qedlang_core::dpn::vm::def::DPNFunctionCircuitDefinition;

use crate::rpc::{
    provider::{QUserRpcProvider, RpcProvider},
    request::QDeployContractRPCRequest,
};

use super::args::DeployContractArgs;

const D: usize = 2;
type C = PoseidonGoldilocksConfig;

fn gen_contract_deploy_and_circuits_for_functions(
    deployer: QHashOut<GoldilocksField>,
    contract_state_tree_height: u8,
    defs: &[DPNFunctionCircuitDefinition],
) -> anyhow::Result<(
    Vec<DapenContractFunctionCircuit<C, D>>,
    QBCDeployContract<GoldilocksField>,
)> {
    let code_defs = defs
        .iter()
        .map(|x| dapen_fc_to_cfc_code_definition(x))
        .collect::<Vec<_>>();
    let mut fingerprints = Vec::with_capacity(defs.len() * 2);
    let circuits = defs
        .iter()
        .map(|x| {
            let c = DapenContractFunctionCircuit::<C, D>::new(
                x,
                contract_state_tree_height as usize,
                UPS_SESSION_PROOF_TREE_HEIGHT as usize,
                false,
            );
            fingerprints.push(c.get_fingerprint());

            // sibling is [method_id, (num_outputs<<32)|num_inputs, 0, 0]
            let inputs_outputs_combo =
                ((x.circuit_outputs.len() as u64) << 32u64) | (x.circuit_inputs.len() as u64);
            fingerprints.push(QHashOut::from_values(
                x.method_id as u64,
                inputs_outputs_combo,
                0,
                0,
            ));
            c
        })
        .collect::<Vec<_>>();

    let deploy = QBCDeployContract {
        deployer,
        code_definition: ContractCodeDefinition {
            state_tree_height: contract_state_tree_height as u16,
            functions: code_defs,
        },
        function_whitelist: fingerprints,
    };

    Ok((circuits, deploy))
}

pub fn run(args: DeployContractArgs) -> anyhow::Result<()> {
    let private_key = QHashOut::<GoldilocksField>::from_str(&args.private_key)?;
    let mut wallet = SimpleQEDZKSignatureManager::<C, D>::new();

    let pk = wallet.add_private_key_get_info(SimpleQEDPrivateKey { private_key });
    let deployer = pk.qfhash::<QEDHasher>();

    let defs_array: Vec<DPNFunctionCircuitDefinition> =
        serde_json::from_str(&fs::read_to_string(args.contract_path)?)?;

    let contract_state_tree_height = MAX_CONTRACT_STATE_TREE_HEIGHT as usize;

    let (_result_circuits, deploy_cmd) = gen_contract_deploy_and_circuits_for_functions(
        deployer,
        contract_state_tree_height as u8,
        &defs_array,
    )?;

    let provider = RpcProvider::new_with_config_path(&args.rpc_config)?;
    provider.deploy_contract(QDeployContractRPCRequest {
        deploy_contract: deploy_cmd,
    })?;

    Ok(())
}
