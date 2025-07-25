use std::str::FromStr;

use plonky2::field::goldilocks_field::GoldilocksField;
use qed_core::data::qhashout::QHashOut;
use qed_prover::local::{
    args::{ContractCallArgs, SignType},
    provider::RpcConfig,
};
use qed_prover::session::WalletSession;
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
    if args.sign_type == SignType::SoftwareDefinedSignV2 {
        let user_sdc: DPNFunctionCircuitDefinition =
            serde_json::from_str(&std::fs::read_to_string("sdc.json")?)?;

        wallet_session
            .wallet
            .register_software_defined_circuit(user_sdc)?;
    }
    let user_pk_hash = wallet_session.add_user_with_type(private_key, args.sign_type.clone())?;

    wallet_session.exec_contract_call_with_sign_type(
        user_pk_hash,
        contract_call_args,
        args.sign_type.clone(),
        vec![],
    )?;

    tracing::info!("local proving end");

    Ok(())
}
