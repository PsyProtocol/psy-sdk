use std::str::FromStr;

use plonky2::field::goldilocks_field::GoldilocksField;
use psy_core::{
    config::network_constants::{MAX_CONTRACT_STATE_TREE_HEIGHT, UPS_SESSION_PROOF_TREE_HEIGHT},
    data::qhashout::QHashOut,
};
use psy_data::traits::qdatastore::qmetadata::QMetaDataStoreReaderSync;
use psy_prover::{
    local::{
        args::{ContractCallArgs, SignData, SignType, WalletSessionArgs},
        provider::RpcConfig,
    },
    session::WalletSession,
    wallet::{
        simple_sign::SoftwareDefinedSignGadget,
        software_defined_circuit::{PSoftwareDefinedSignatureInput, QSoftwareDefinedSignatureInput, SoftwareDefinedSignatureInput},
    },
};
use psy_vm::dpn::vm::def::DPNFunctionCircuitDefinition;
use serde::{Deserialize, Serialize};

use super::args::SubmitEndCapArgs;

#[derive(Clone, Serialize, Deserialize)]
pub struct ExecContractCallArgs {
    pub rpc_config: RpcConfig,
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

    let config_str = std::fs::read_to_string(&args.rpc_config)?;
    let json_value: serde_json::Value = serde_json::from_str(&config_str)?;
    let rpc_config: RpcConfig = serde_json::from_value(json_value["network"].clone())?;
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

    let config_str = std::fs::read_to_string(&args.rpc_config)?;
    let json_value: serde_json::Value = serde_json::from_str(&config_str)?;
    let rpc_config: RpcConfig = serde_json::from_value(json_value["network"].clone())?;
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
    let fingerprint = if args.sign_type == SignType::SoftwareDefinedSign {
        let user_sdc: DPNFunctionCircuitDefinition = serde_json::from_str(&std::fs::read_to_string("sdc.json")?)?;

        let contract_state_tree_height = wallet_session
            .st_provider
            .get_contract_code_definition(args.contract_id)
            .await
            .map(|cfc| cfc.state_tree_height as u8)
            .unwrap_or(MAX_CONTRACT_STATE_TREE_HEIGHT);

        // let sdc_input =
        // SoftwareDefinedSignatureInput::QED(QSoftwareDefinedSignatureInput {
        //     fn_def: user_sdc,
        //     contract_id: args.contract_id,
        //     contract_state_tree_height: contract_state_tree_height,
        //     session_proof_tree_height: UPS_SESSION_PROOF_TREE_HEIGHT,
        //     force_four_align: false,
        // });

        let sign_circuit = Box::new(SoftwareDefinedSignGadget {});
        let sdc_input = SoftwareDefinedSignatureInput::PLONKY2(PSoftwareDefinedSignatureInput {
            contract_state_tree_height,
            input_len: 0,
            sign_circuit,
        });

        Some(wallet_session.wallet.register_software_defined_circuit(sdc_input).await?)
    } else {
        None
    };
    let user_pk_hash = wallet_session
        .add_user_with_type(args.private_key, args.sign_type.clone(), fingerprint)
        .await?;

    let sign_data = fingerprint.map(|fp| SignData {
        fingerprint: fp,
        sign_contract_id: args.contract_id,
        sign_inputs: args.sign_inputs,
    });
    let tx_hash = wallet_session
        .exec_contract_call_with_sign_data(user_pk_hash, args.contract_call_args, sign_data)
        .await?;

    tracing::info!("local proving end with tx hash: {}", tx_hash);

    Ok(())
}
