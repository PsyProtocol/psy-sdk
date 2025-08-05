use std::str::FromStr;

use plonky2::field::goldilocks_field::GoldilocksField;
use qed_core::config::network_constants::{
    MAX_CONTRACT_STATE_TREE_HEIGHT, UPS_SESSION_PROOF_TREE_HEIGHT,
};
use qed_core::data::qhashout::QHashOut;
use qed_data::traits::qdatastore::qmetadata::QMetaDataStoreReaderSync;
use qed_prover::session::WalletSession;
use qed_prover::wallet::simple_sign::SoftwareDefinedSignGadget;
use qed_prover::wallet::software_defined_circuit::{
    PSoftwareDefinedSignatureInput, QSoftwareDefinedSignatureInput,
};
use qed_prover::{
    local::{
        args::{ContractCallArgs, SignType},
        provider::RpcConfig,
    },
    wallet::software_defined_circuit::SoftwareDefinedSignatureInput,
};
use qedlang_core::dpn::vm::def::DPNFunctionCircuitDefinition;

use super::args::SubmitEndCapArgs;

pub fn run(args: SubmitEndCapArgs) -> anyhow::Result<()> {
    tracing::info!(
        "local proving start with {}",
        serde_json::to_string_pretty(&args)?
    );
    let contract_call_args: Vec<ContractCallArgs> = vec![ContractCallArgs {
        contract_id: args.contract_id,
        method_name: args.method_name,
        inputs: args.inputs,
    }];

    let config_str = std::fs::read_to_string(&args.rpc_config)?;
    let json_value: serde_json::Value = serde_json::from_str(&config_str)?;
    let rpc_config: RpcConfig = serde_json::from_value(json_value["network"].clone())?;
    let private_key = QHashOut::<GoldilocksField>::from_str(&args.private_key)
        .map_err(|e| anyhow::format_err!("{}", e.to_string()))?;

    let mut wallet_session = WalletSession::new(&rpc_config)?;
    let fingerprint = if args.sign_type == SignType::SoftwareDefinedSign {
        let user_sdc: DPNFunctionCircuitDefinition =
            serde_json::from_str(&std::fs::read_to_string("sdc.json")?)?;

        let contract_state_tree_height = wallet_session
            .st_provider
            .get_contract_code_definition(args.contract_id)
            .map(|cfc| cfc.state_tree_height as u8)
            .unwrap_or(MAX_CONTRACT_STATE_TREE_HEIGHT);

        // let sdc_input = SoftwareDefinedSignatureInput::QED(QSoftwareDefinedSignatureInput {
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

        Some(
            wallet_session
                .wallet
                .register_software_defined_circuit(sdc_input)?,
        )
    } else {
        None
    };
    let user_pk_hash =
        wallet_session.add_user_with_type(private_key, args.sign_type.clone(), fingerprint)?;

    wallet_session.exec_contract_call_with_sign_type(
        user_pk_hash,
        contract_call_args,
        args.sign_type.clone(),
        fingerprint,
        Some(args.contract_id),
        vec![],
    )?;

    tracing::info!("local proving end");

    Ok(())
}
