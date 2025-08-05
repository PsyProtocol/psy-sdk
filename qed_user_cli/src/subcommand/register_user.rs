use crate::subcommand::args::RegisterUserArgs;
use anyhow::Result;
use plonky2::field::goldilocks_field::GoldilocksField;
use plonky2::hash::poseidon::{PoseidonHash, PoseidonPermutation};
use plonky2::plonk::config::PoseidonGoldilocksConfig;
use qed_common_circuit::circuits::traits::qstandard::QStandardCircuit;
use qed_core::config::network_constants::{
    MAX_CONTRACT_STATE_TREE_HEIGHT, UPS_SESSION_PROOF_TREE_HEIGHT,
};
use qed_core::data::qhashout::QHashOut;
use qed_crypto::hash::traits::qhashable::QFieldHashable;
use qed_crypto::signature::zk::{data::ZKPublicKeyInfo, wallet::SimpleQEDPrivateKey};
use qed_data::config::store_config::QEDHasher;
use qed_data::traits::qdatastore::qmetadata::QMetaDataStoreReaderSync;
use qed_prover::local::args::SignType;
use qed_prover::local::provider::{QUserRpcProvider, RpcProvider};
use qed_prover::local::request::QRegisterUserRPCRequest;
use qed_prover::wallet::simple_sign::SoftwareDefinedSignGadget;
use qed_prover::wallet::software_defined_circuit::{
    get_sdc_public_key_param, PSoftwareDefinedSignatureInput, QSoftwareDefinedSignatureInput,
    SoftwareDefinedSignatureCircuit, SoftwareDefinedSignatureGadget, SoftwareDefinedSignatureInput,
};
use qed_prover::wallet::utils::{get_secp_public_key, hash_no_pad_compressed_public_key};
use qedlang_core::dpn::vm::def::DPNFunctionCircuitDefinition;
use std::str::FromStr;

const ZK_FINGERPRINT: &str = "65ac37ce1e8ef55ca83dc342e76c1e9c0b377c98eb38bcc95c08525418f067c0";
const SECP256K1_FINGERPRINT: &str =
    "795c781a246206d4d1efc7cf566c31319928c52957efc5cb4f27362d94a4976f";

pub fn run(args: RegisterUserArgs) -> Result<()> {
    let provider = RpcProvider::new_with_config_path(&args.rpc_config)?;
    type C = PoseidonGoldilocksConfig;
    const D: usize = 2;

    let user_sdc: DPNFunctionCircuitDefinition =
        serde_json::from_str(&std::fs::read_to_string("sdc.json")?)?;

    let contract_state_tree_height = provider
        .get_contract_code_definition(0)
        .map(|cfc| cfc.state_tree_height as u8)
        .unwrap_or(MAX_CONTRACT_STATE_TREE_HEIGHT);

    // let sdc_input = SoftwareDefinedSignatureInput::QED(QSoftwareDefinedSignatureInput {
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

    let sdc =
        SoftwareDefinedSignatureCircuit::<C, D, SoftwareDefinedSignatureGadget>::new(&sdc_input);

    // Parse private key
    let private_key_base = QHashOut::<GoldilocksField>::from_str(&args.private_key)
        .map_err(|e| anyhow::format_err!("Failed to parse private key: {}", e))?;
    // Get public key info
    let public_key_param = match SignType::from(args.sign_type.clone()) {
        SignType::ZKSign => SimpleQEDPrivateKey {
            private_key: private_key_base,
        }
        .get_public_key_param::<QEDHasher>(),
        SignType::SECP256K1Sign => {
            let compressed_public_key = get_secp_public_key(private_key_base)?;
            tracing::info!("compressed public key {:?}", compressed_public_key);
            hash_no_pad_compressed_public_key::<GoldilocksField, PoseidonPermutation<GoldilocksField>>(
                compressed_public_key,
            )
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
                assert_eq!(
                    fingerprint, SECP256K1_FINGERPRINT,
                    "SECP key fingerprint mismatch"
                );
            }
            QHashOut::<GoldilocksField>::from_str(&SECP256K1_FINGERPRINT)?
        }
        SignType::SoftwareDefinedSign => sdc.get_fingerprint(),
    };

    let public_key_info = ZKPublicKeyInfo {
        fingerprint,
        public_key_param,
    };

    // Register user
    provider.register_user(QRegisterUserRPCRequest {
        public_key: public_key_info,
    })?;

    // Output the result
    let public_key_hash = public_key_info.qfhash::<QEDHasher>();
    println!("{{");
    println!("  \"public_key_hash\": \"{}\",", public_key_hash);
    println!("  \"fingerprint\": \"{}\",", public_key_info.fingerprint);
    println!(
        "  \"public_key_param\": \"{}\"",
        public_key_info.public_key_param
    );
    println!("}}");

    Ok(())
}
