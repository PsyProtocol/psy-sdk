use std::str::FromStr;

use plonky2::{
    field::{
        goldilocks_field::GoldilocksField,
        types::{Field, PrimeField64},
    },
    plonk::config::PoseidonGoldilocksConfig,
};
use qed_common_circuit::circuits::{
    traits::qstandard::QStandardCircuit, zk_signature3::manager::SimpleQEDZKSignatureManager,
};
use qed_core::{
    config::network_constants::{QED_NETWORK_MAGIC_REGTEST, UPS_SESSION_PROOF_TREE_HEIGHT},
    data::qhashout::QHashOut,
    ups::circuits::LocalCircuitType,
};
use qed_crypto::signature::zk::wallet::SimpleQEDPrivateKey;
use qed_data::guta::end_cap_input::SubmitUserEndCapNonProofInput;
use qed_prover::ups::{
    circuit_manager::core::QEDUPSStepCircuitManager, session::UserProvingSessionManager,
};
use qed_store::{
    config::store_config::QEDHasher,
    controllers::local::{
        proving_session::QEDLocalProvingSessionStore, session_info::SessionCircuitInfoStore,
    },
    store::imm::cmd_processor::QEDReadCommandProcessorSync,
};

use crate::rpc::{
    provider::{QUserRpcProvider, RpcProvider},
    request::QSubmitEndCapRPCRequest,
};

use super::{
    args::{ContractCallArgs, SubmitEndCapArgs},
    utils::prove_func,
};

type C = PoseidonGoldilocksConfig;
const D: usize = 2;
type F = GoldilocksField;

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

    let mut st_provider = RpcProvider::new_with_config_path(&args.rpc_config)?;

    let latest_l2_block_state = st_provider.resolve_get_latest_l2_block_state()?;
    tracing::info!("latest l2 block state: {:?}", latest_l2_block_state);

    tracing::info!(
        "start QEDUPSStepCircuitManager with network magic {:x}",
        QED_NETWORK_MAGIC_REGTEST
    );
    let main_circuits =
        QEDUPSStepCircuitManager::<C, D>::new_with_config(QED_NETWORK_MAGIC_REGTEST);

    let priv_key = QHashOut::<GoldilocksField>::from_str(&args.private_key)
        .map_err(|e| anyhow::format_err!("{}", e.to_string()))?;
    let mut wallet = SimpleQEDZKSignatureManager::<C, D>::new();

    let zkey_info = wallet.add_private_key_get_info(SimpleQEDPrivateKey {
        private_key: priv_key,
    });
    let new_nonce = GoldilocksField::from_noncanonical_u64(args.nonce);

    let user_id = st_provider.get_user_id(zkey_info.public_key_param)?;
    tracing::info!("user id: {}", user_id);
    tracing::info!("public key: {}", zkey_info.public_key_param);
    st_provider.current_user_id = user_id;

    tracing::info!(
        "create QEDLocalProvingSessionStore with checkpoint id {}, user id {}, nonce {}",
        latest_l2_block_state.checkpoint_id,
        user_id,
        new_nonce.to_canonical_u64()
    );
    let lps = QEDLocalProvingSessionStore::new_at(
        st_provider.clone(),
        GoldilocksField::from_noncanonical_u64(latest_l2_block_state.checkpoint_id),
        F::from_canonical_u64(user_id),
        new_nonce,
        UPS_SESSION_PROOF_TREE_HEIGHT as usize,
    );

    let mut circuit_info = SessionCircuitInfoStore::new();

    tracing::info!("register ZKSignature circuit info");
    circuit_info.register_circuit(
        LocalCircuitType::SimpleZKSignature.into(),
        wallet.circuit.get_fingerprint(),
        wallet.circuit.get_verifier_config_ref().into(),
    );

    main_circuits.register_info(&mut circuit_info);

    tracing::info!("create UserProvingSessionManager");
    let mut mgr = UserProvingSessionManager::<F, QEDHasher, _, C, D>::new(
        lps,
        circuit_info,
        main_circuits.ups_circuit_whitelist_root,
    )?;

    tracing::info!("local proving ups start");
    mgr.prove_ups_start(&main_circuits)?;

    for contract_call_arg in contract_call_args {
        tracing::info!(
            "prove contract call at contract {}, method {}",
            contract_call_arg.contract_id,
            contract_call_arg.method_name
        );
        prove_func(
            &st_provider,
            &main_circuits,
            &mut mgr,
            contract_call_arg.contract_id,
            &contract_call_arg.method_name,
            contract_call_arg
                .inputs
                .iter()
                .map(|x| GoldilocksField::from_noncanonical_u64(*x))
                .collect(),
        )?;
    }

    let sighash = mgr.get_sighash(QED_NETWORK_MAGIC_REGTEST, new_nonce);
    tracing::info!("zk sign for signhash: {}", sighash.to_string());
    let signature_proof = wallet.zk_sign_for_private_key_value(priv_key, sighash)?;

    mgr.proof_tree_state
        .finalize_tree(&main_circuits.proof_tree_agg_circuits)?;

    let public_key_param = SimpleQEDPrivateKey::new(priv_key).get_public_key_param::<QEDHasher>();
    tracing::info!(
        "prove end cap with network magic {:x}, nonce {}, fingerprint {}, public key param {}, signature proof {:?}",
        QED_NETWORK_MAGIC_REGTEST,
        new_nonce,
        wallet.circuit.get_fingerprint(),
        public_key_param,
        signature_proof
    );
    let end_cap_proof = mgr.prove_end_cap(
        &main_circuits,
        QED_NETWORK_MAGIC_REGTEST,
        new_nonce,
        wallet.circuit.get_fingerprint(),
        public_key_param,
        signature_proof,
        wallet.circuit.get_verifier_config_ref().to_owned(),
    )?;

    let user_ec_input: SubmitUserEndCapNonProofInput<F> = mgr.get_api_input()?;
    tracing::info!(
        "get user ec input: {}",
        serde_json::to_string_pretty(&user_ec_input)?
    );

    tracing::info!(
        "submit end cap proof: {}",
        serde_json::to_string_pretty(&end_cap_proof)?
    );
    let req = QSubmitEndCapRPCRequest {
        user_ec_input,
        proof: end_cap_proof,
    };

    st_provider.submit_end_cap_proof::<F>(req)?;
    tracing::info!("local proving end");

    Ok(())
}
