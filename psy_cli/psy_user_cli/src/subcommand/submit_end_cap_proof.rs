use std::str::FromStr;

use plonky2::field::{goldilocks_field::GoldilocksField, types::Field};
use psy_common::{
    args::{ContractCallArgs, ContractCallData, DPNSoftwareDefinedCallData, SignType, WalletSessionArgs},
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

use crate::subcommand::key_utils::load_wallet_key_info;

pub async fn run(args: WalletSessionArgs) -> anyhow::Result<()> {
    let psy_config = psy_config::PsyConfigGoldilocks::from_file(&args.rpc_config)?;
    let rpc_config = psy_config.get_current_network()?.clone();

    let mut wallet_session = WalletSession::new(&rpc_config).await?;
    let mut info = load_wallet_key_info(&args.wallet, false)?;

    match args.wallet.sign_type {
        SignType::SoftwareDefinedPlonky2Sign => {
            let fingerprint = wallet_session
                .wallet
                .register_plonky2_software_defined_circuit(MAX_CONTRACT_STATE_TREE_HEIGHT, 0)
                .await?;

            if let Some(mut circuit) = wallet_session.wallet.get_plonky2_software_defined_circuit_mut(&fingerprint) {
                // add custom constraints here
                // state_reader must do the same thing while generating
                // witnesses
            }

            assert_eq!(info.fingerprint, fingerprint, "software-defined-plonky2-sign key fingerprint mismatch");
        }
        SignType::SoftwareDefinedDPNSign => {
            let user_sdc: DPNFunctionCircuitDefinition = serde_json::from_str(&std::fs::read_to_string("sdc.json")?)?;
            let fingerprint = wallet_session
                .wallet
                .register_psy_software_defined_circuit(
                    user_sdc,
                    args.sign_contract_id.unwrap(),
                    MAX_CONTRACT_STATE_TREE_HEIGHT,
                    UPS_SESSION_PROOF_TREE_HEIGHT,
                    false,
                )
                .await?;
            assert_eq!(info.fingerprint, fingerprint, "software-defined-dpn-sign key fingerprint mismatch");
        }
        _ => {}
    };
    let user_pk_hash = wallet_session.add_user(info.private_key, info.fingerprint).await?;
    let contract_call_data = args.to_contract_call_data()?;
    let user_endcap_uuid = wallet_session.exec_contract_call(user_pk_hash, contract_call_data).await?;

    tracing::info!("Contract call completed with user_endcap_uuid: {}", user_endcap_uuid.to_string());
    Ok(())
}
