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
use psy_prover::wallet::memory_wallet::{get_secp256k1_fingerprint, get_zk_fingerprint, PsyMemoryWallet};
use psy_rust_sdk::{
    provider::{QUserRpcProvider, RpcProvider},
    request::QRegisterUserRPCRequest,
};
use psy_ups_circuit::{
    circuit_manager::core::PsyUPSStepCircuitManager,
    signature::software_defined::{get_sdc_public_key_param, Plonky2SoftwareDefinedSignatureGadget},
};
use psy_vm::{dpn::vm::def::DPNFunctionCircuitDefinition, ups::circuit_manager::UPSCircuitManager};

use crate::subcommand::{args::RegisterUserArgs, key_utils::load_wallet_key_info};

pub async fn run(args: RegisterUserArgs) -> Result<()> {
    let provider = RpcProvider::new_with_config_path(&args.rpc_config)?;
    type C = PoseidonGoldilocksConfig;
    const D: usize = 2;

    let mut info = load_wallet_key_info(&args.wallet, false)?;
    let mut fingerprint = info.fingerprint.clone();
    let mut private_key_base = info.private_key;
    let mut generated_private_key = info.generated;
    let main_circuits: Box<dyn UPSCircuitManager<C, D>> = Box::new(PsyUPSStepCircuitManager::<C, D>::new_with_config(
        psy_config::network_constants::PSY_NETWORK_MAGIC,
    ));
    let mut wallet = PsyMemoryWallet::new(vec![main_circuits]);

    let public_key_info = wallet.get_or_create_user(private_key_base, fingerprint).await?;

    provider.register_user(QRegisterUserRPCRequest { public_key: public_key_info }).await?;

    let public_key_hash = public_key_info.qfhash::<PsyHasher>();
    println!("{{");
    if generated_private_key {
        println!("  \"private_key\": \"{}\",", private_key_base);
    }
    println!("  \"public_key_hash\": \"{}\",", public_key_hash);
    println!("  \"fingerprint\": \"{}\",", public_key_info.fingerprint);
    println!("  \"public_key_param\": \"{}\"", public_key_info.public_key_param);
    println!("}}");

    Ok(())
}
