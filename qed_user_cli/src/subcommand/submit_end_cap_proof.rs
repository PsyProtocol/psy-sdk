use std::str::FromStr;

use super::args::{ContractCallArgs, SubmitEndCapArgs};
use crate::{rpc::provider::RpcConfig, subcommand::session::WalletSession};
use plonky2::field::goldilocks_field::GoldilocksField;
use qed_core::data::qhashout::QHashOut;

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

    let rpc_config: RpcConfig = serde_json::from_str(&std::fs::read_to_string(args.rpc_config)?)?;
    let private_key = QHashOut::<GoldilocksField>::from_str(&args.private_key)
        .map_err(|e| anyhow::format_err!("{}", e.to_string()))?;

    let mut wallet_session = WalletSession::new(&rpc_config)?;
    let user_pk_hash = wallet_session.add_user(private_key)?;
    wallet_session.switch_user(user_pk_hash)?;
    wallet_session.start_session()?;
    wallet_session.prove_contract_calls(contract_call_args)?;
    wallet_session.sign_and_submit()?;

    tracing::info!("local proving end");

    Ok(())
}
