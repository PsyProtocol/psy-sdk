use std::str::FromStr;

use anyhow::Result;
use plonky2::{
    field::{goldilocks_field::GoldilocksField, types::Field},
    hash::poseidon::{PoseidonHash, PoseidonPermutation},
    plonk::config::PoseidonGoldilocksConfig,
};
use psy_common::{args::SignType, data::qhashout::QHashOut};
use psy_common_circuit::circuits::traits::qstandard::QStandardCircuit;
use psy_config::network_constants::{MAX_CONTRACT_STATE_TREE_HEIGHT, UPS_SESSION_PROOF_TREE_HEIGHT};
use psy_crypto::{
    hash::traits::qhashable::QFieldHashable,
    signature::{
        secp256k1::wallet::{get_secp_public_key, hash_no_pad_compressed_public_key},
        zk::{data::ZKPublicKeyInfo, wallet::SimplePsyPrivateKey},
    },
};
use psy_data::{config::store_config::PsyHasher, traits::qdatastore::qmetadata::QMetaDataStoreReaderSync};
use psy_prover::wallet::memory_wallet::{PsyMemoryWallet, SECP256K1_FINGERPRINT, ZK_FINGERPRINT};
use psy_rust_sdk::{
    provider::{QUserRpcProvider, RpcProvider},
    request::QRegisterUserRPCRequest,
};
use psy_ups_circuit::{
    circuit_manager::core::PsyUPSStepCircuitManager,
    signature::software_defined::{get_sdc_public_key_param, Plonky2SoftwareDefinedSignatureGadget},
};
use psy_vm::{dpn::vm::def::DPNFunctionCircuitDefinition, ups::circuit_manager::UPSCircuitManager};

use crate::subcommand::args::RegisterUserArgs;

pub async fn run(args: RegisterUserArgs) -> Result<()> {
    let provider = RpcProvider::new_with_config_path(&args.rpc_config)?;
    type C = PoseidonGoldilocksConfig;
    const D: usize = 2;

    let user_sdc: DPNFunctionCircuitDefinition = serde_json::from_str(&std::fs::read_to_string("sdc.json")?)?;

    let contract_state_tree_height = MAX_CONTRACT_STATE_TREE_HEIGHT;

    // Parse private key
    let private_key_base = match &args.private_key {
        Some(key) => QHashOut::<GoldilocksField>::from_str(&key).map_err(|e| anyhow::format_err!("Failed to parse private key: {}", e))?,
        None => {
            let private_key = QHashOut::rand();
            tracing::info!("random private key {:?}", private_key.to_string());
            private_key
        }
    };
    // Create wallet and circuit manager
    let main_circuits: Box<dyn UPSCircuitManager<C, D>> = Box::new(PsyUPSStepCircuitManager::<C, D>::new_with_config(
        psy_config::network_constants::PSY_NETWORK_MAGIC,
    ));
    let mut wallet = PsyMemoryWallet::new(vec![main_circuits]);

    // Get fingerprint based on sign type
    let fingerprint = match SignType::from(args.sign_type.clone()) {
        SignType::ZKSign => {
            if let Some(fingerprint) = args.fingerprint {
                assert_eq!(fingerprint, ZK_FINGERPRINT, "ZK key fingerprint mismatch");
            }
            QHashOut::<GoldilocksField>::from_str(&ZK_FINGERPRINT)?
        }
        SignType::SECP256K1Sign => {
            if let Some(fingerprint) = args.fingerprint {
                assert_eq!(fingerprint, SECP256K1_FINGERPRINT, "SECP key fingerprint mismatch");
            }
            QHashOut::<GoldilocksField>::from_str(&SECP256K1_FINGERPRINT)?
        }
        SignType::SoftwareDefinedDPNSign => QHashOut::<GoldilocksField>::from_str(
            &args
                .fingerprint
                .ok_or_else(|| anyhow::format_err!("software defined dpn sign need fingerprint"))?,
        )?,
        SignType::SoftwareDefinedPlonky2Sign => wallet.register_plonky2_software_defined_circuit(contract_state_tree_height, 0).await?,
    };

    // Use simplified interface
    let public_key_info = wallet.get_or_create_user(private_key_base, fingerprint).await?;

    // Register user
    provider.register_user(QRegisterUserRPCRequest { public_key: public_key_info }).await?;

    // Output the result
    let public_key_hash = public_key_info.qfhash::<PsyHasher>();
    println!("{{");
    if args.private_key.is_none() {
        println!("  \"private_key\": \"{}\",", private_key_base);
    }
    println!("  \"public_key_hash\": \"{}\",", public_key_hash);
    println!("  \"fingerprint\": \"{}\",", public_key_info.fingerprint);
    println!("  \"public_key_param\": \"{}\"", public_key_info.public_key_param);
    println!("}}");

    Ok(())
}
