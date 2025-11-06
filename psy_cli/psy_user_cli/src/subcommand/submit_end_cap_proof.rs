use std::str::FromStr;

use plonky2::field::{goldilocks_field::GoldilocksField, types::Field};
use psy_common::{
    args::{ContractCallArgs, SignData, SignType, WalletSessionArgs},
    data::qhashout::QHashOut,
};
use psy_common_circuit::builder::comparison::CircuitBuilderComparison;
use psy_config::network_constants::{MAX_CONTRACT_STATE_TREE_HEIGHT, UPS_SESSION_PROOF_TREE_HEIGHT};
use psy_data::traits::qdatastore::qmetadata::QMetaDataStoreReaderSync;
use psy_prover::session::WalletSession;
use psy_rust_sdk::provider::NetworkConfig;
use psy_ups_circuit::signature::software_defined::Plonky2SoftwareDefinedSignatureGadget;
use psy_vm::dpn::vm::def::DPNFunctionCircuitDefinition;
use serde::{Deserialize, Serialize};

use super::args::SubmitEndCapArgs;

#[derive(Clone, Serialize, Deserialize)]
pub struct ExecContractCallArgs {
    pub rpc_config: NetworkConfig<GoldilocksField>,
    pub private_key: QHashOut<GoldilocksField>,
    pub contract_id: u64,
    pub contract_call_args: Vec<ContractCallArgs>,
    pub sign_type: SignType,
    pub sign_inputs: Vec<u64>,
}

pub async fn run(args: SubmitEndCapArgs) -> anyhow::Result<()> {
    tracing::info!("local proving start with {}", serde_json::to_string_pretty(&args)?);
    let contract_call_args: Vec<ContractCallArgs> = vec![ContractCallArgs {
        contract_id: args.contract_id,
        method_name: args.method_name,
        inputs: args.inputs,
    }];

    let psy_config = psy_config::PsyConfigGoldilocks::from_file(&args.rpc_config)?;
    let rpc_config = psy_config.get_current_network()?.clone();
    let private_key = QHashOut::<GoldilocksField>::from_str(&args.private_key).map_err(|e| anyhow::format_err!("{}", e.to_string()))?;

    let exec_contract_call_args = ExecContractCallArgs {
        rpc_config,
        private_key,
        contract_id: args.contract_id,
        contract_call_args,
        sign_type: args.sign_type,
        sign_inputs: args.sign_inputs,
    };

    run_inner(exec_contract_call_args).await?;
    Ok(())
}

pub async fn run_multi(args: WalletSessionArgs) -> anyhow::Result<()> {
    tracing::info!("local proving start with {}", serde_json::to_string_pretty(&args)?);
    let contract_call_args: Vec<ContractCallArgs> = serde_json::from_str(&std::fs::read_to_string(&args.contract_calls)?)?;

    let psy_config = psy_config::PsyConfigGoldilocks::from_file(&args.rpc_config)?;
    let rpc_config = psy_config.get_current_network()?.clone();
    let private_key = QHashOut::<GoldilocksField>::from_str(&args.private_key).map_err(|e| anyhow::format_err!("{}", e.to_string()))?;

    let exec_contract_call_args = ExecContractCallArgs {
        rpc_config,
        private_key,
        contract_id: args.contract_id,
        contract_call_args,
        sign_type: args.sign_type,
        sign_inputs: args.sign_inputs,
    };

    run_inner(exec_contract_call_args).await?;
    Ok(())
}

pub async fn run_inner(args: ExecContractCallArgs) -> anyhow::Result<()> {
    let mut wallet_session = WalletSession::new(&args.rpc_config).await?;
    let fingerprint = if args.sign_type == SignType::SoftwareDefinedPlonky2Sign {
        let user_sdc: DPNFunctionCircuitDefinition = serde_json::from_str(&std::fs::read_to_string("sdc.json")?)?;

        let contract_state_tree_height = wallet_session
            .st_provider
            .get_contract_code_definition(args.contract_id)
            .await
            .map(|cfc| cfc.state_tree_height as u8)
            .unwrap_or(MAX_CONTRACT_STATE_TREE_HEIGHT);

        let config = plonky2::plonk::circuit_data::CircuitConfig::standard_recursion_config();
        let mut builder = plonky2::plonk::circuit_builder::CircuitBuilder::<GoldilocksField, 2>::new(config);

        let mut gadget = Plonky2SoftwareDefinedSignatureGadget::add_virtual_to(&mut builder, contract_state_tree_height, 0);

        // Add custom constraints: ensure slot0 is less than 1000
        gadget.add_custom_constraints(&mut builder, |builder, state_reader, _circuit_inputs| {
            if let Ok(slot0) = state_reader.get_self_user_current_contract_state_slot_single(builder, GoldilocksField::from_canonical_u64(0)) {
                let one_thousand = builder.constant(GoldilocksField::from_canonical_u64(1000));
                builder.ensure_is_less_than(32, slot0, one_thousand);
            }
        });

        gadget.build_circuit(builder).unwrap();
        let fingerprint = gadget.get_fingerprint();

        wallet_session.wallet.software_defined_plonky2_circuits.insert(fingerprint, gadget);
        Some(fingerprint)
    } else {
        None
    };
    let fingerprint = fingerprint.unwrap_or_else(|| psy_prover::wallet::memory_wallet::get_zk_fingerprint());
    let user_pk_hash = wallet_session.add_user(args.private_key, fingerprint).await?;

    let sign_data = Some(SignData {
        fingerprint: fingerprint,
        sign_contract_id: args.contract_id,
        sign_inputs: args.sign_inputs,
    });
    let tx_hash = wallet_session
        .exec_contract_call_with_sign_data(user_pk_hash, args.contract_call_args, sign_data)
        .await?;

    tracing::info!("local proving end with tx hash: {}", tx_hash);

    Ok(())
}
