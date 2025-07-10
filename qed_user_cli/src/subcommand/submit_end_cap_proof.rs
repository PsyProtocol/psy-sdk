use std::str::FromStr;

use super::args::{ContractCallArgs, SubmitEndCapArgs};
use crate::{rpc::provider::RpcConfig};
use plonky2::field::goldilocks_field::GoldilocksField;
use qed_core::data::qhashout::QHashOut;
use crate::session::WalletSession;

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

    let mut wallet_session = WalletSession::new_fast_setup(&rpc_config)?;
    let user_pk_hash = wallet_session.add_user(private_key)?;

    wallet_session.exec_contract_call(user_pk_hash, contract_call_args)?;

    tracing::info!("local proving end");

    Ok(())
}
