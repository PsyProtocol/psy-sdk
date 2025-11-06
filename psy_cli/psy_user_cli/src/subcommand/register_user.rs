use std::str::FromStr;

use anyhow::Result;
use plonky2::{
    field::goldilocks_field::GoldilocksField,
    hash::poseidon::{PoseidonHash, PoseidonPermutation},
    plonk::config::PoseidonGoldilocksConfig,
};
use psy_common::data::qhashout::QHashOut;
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
use psy_prover::{
    local::args::SignType,
    wallet::{
        memory_wallet::{SECP256K1_FINGERPRINT, ZK_FINGERPRINT},
        simple_sign::SoftwareDefinedSignGadget,
        software_defined_circuit::{
            get_sdc_public_key_param, PSoftwareDefinedSignatureInput, SoftwareDefinedSignatureCircuit, SoftwareDefinedSignatureGadget,
            SoftwareDefinedSignatureInput,
        },
    },
};
use psy_rust_sdk::{
    provider::{QUserRpcProvider, RpcProvider},
    request::{QRegisterUserRPCRequest, QSoftwareDefinedSignatureInput},
};
use psy_vm::dpn::vm::def::DPNFunctionCircuitDefinition;

use crate::subcommand::args::RegisterUserArgs;

pub async fn run(args: RegisterUserArgs) -> Result<()> {
    let provider = RpcProvider::new_with_config_path(&args.rpc_config)?;
    type C = PoseidonGoldilocksConfig;
    const D: usize = 2;

    let user_sdc: DPNFunctionCircuitDefinition = serde_json::from_str(&std::fs::read_to_string("sdc.json")?)?;

    let contract_state_tree_height = provider
        .get_contract_code_definition(0)
        .await
        .map(|cfc| cfc.state_tree_height as u8)
        .unwrap_or(MAX_CONTRACT_STATE_TREE_HEIGHT);

    // Parse private key
    let private_key_base = match &args.private_key {
        Some(key) => QHashOut::<GoldilocksField>::from_str(&key).map_err(|e| anyhow::format_err!("Failed to parse private key: {}", e))?,
        None => {
            let private_key = QHashOut::rand();
            tracing::info!("random private key {:?}", private_key.to_string());
            private_key
        }
    };
    // Get public key info
    let public_key_param = match SignType::from(args.sign_type.clone()) {
        SignType::ZKSign => SimplePsyPrivateKey {
            private_key: private_key_base,
        }
        .get_public_key_param::<PsyHasher>(),
        SignType::SECP256K1Sign => {
            let compressed_public_key = get_secp_public_key(private_key_base)?;
            tracing::info!("compressed public key {:?}", compressed_public_key);
            hash_no_pad_compressed_public_key::<GoldilocksField, PoseidonPermutation<GoldilocksField>>(compressed_public_key)
        }
        SignType::SoftwareDefinedSign => get_sdc_public_key_param(&private_key_base),
    };

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
        SignType::SoftwareDefinedSign => {
            // let sdc_input =
            // SoftwareDefinedSignatureInput::Psy(QSoftwareDefinedSignatureInput {
            //     fn_def: user_sdc,
            //     contract_id: 0,
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

            let sdc = SoftwareDefinedSignatureCircuit::<C, D, SoftwareDefinedSignatureGadget>::new(&sdc_input).await;
            sdc.get_fingerprint()
        }
    };

    let public_key_info = ZKPublicKeyInfo {
        fingerprint,
        public_key_param,
    };

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
