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

pub async fn run(args: WalletSessionArgs) -> anyhow::Result<()> {
    let psy_config = psy_config::PsyConfigGoldilocks::from_file(&args.rpc_config)?;
    let rpc_config = psy_config.get_current_network()?.clone();
    let private_key = QHashOut::<GoldilocksField>::from_str(&args.private_key)?;

    let mut wallet_session = WalletSession::new(&rpc_config).await?;
    let fingerprint = match args.sign_type {
        SignType::ZKSign => {
            psy_prover::wallet::memory_wallet::get_zk_fingerprint()
        }
        SignType::SECP256K1Sign => {
            psy_prover::wallet::memory_wallet::get_secp256k1_fingerprint()
        }
        SignType::SoftwareDefinedPlonky2Sign => {
            let fingerprint = wallet_session.wallet.register_plonky2_software_defined_circuit(MAX_CONTRACT_STATE_TREE_HEIGHT, 0).await?;

            if let Some(mut circuit) = wallet_session.wallet.get_plonky2_software_defined_circuit_mut(&fingerprint) {
                // add custom constraints here
                // state_reader must do the same thing while generating witnesses
            }

            fingerprint
        }
        SignType::SoftwareDefinedDPNSign => {
            let user_sdc: DPNFunctionCircuitDefinition = serde_json::from_str(&std::fs::read_to_string("sdc.json")?)?;
            let fingerprint = wallet_session.wallet.register_psy_software_defined_circuit(
                user_sdc,
                args.sign_contract_id.unwrap(),
                MAX_CONTRACT_STATE_TREE_HEIGHT,
                UPS_SESSION_PROOF_TREE_HEIGHT,
                false
            ).await?;
            fingerprint
        }
    };
    let user_pk_hash = wallet_session.add_user(private_key, fingerprint).await?;
    let contract_call_data = args.to_contract_call_data()?;
    let tx_hash = wallet_session.exec_contract_call(user_pk_hash, contract_call_data).await?;

    tracing::info!("Contract call completed with tx hash: {}", tx_hash);
    Ok(())
}
