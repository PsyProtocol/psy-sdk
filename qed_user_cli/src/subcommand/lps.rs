use std::str::FromStr;

use anyhow::Ok;
use plonky2::{
    field::{goldilocks_field::GoldilocksField, types::Field},
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

use super::{args::ContractCallArgs, utils::prove_func};

use kvq_store_lmdbx::KVQlibmdbxStore;
use std::sync::Arc;

use plonky2::plonk::proof::ProofWithPublicInputs;

type C = PoseidonGoldilocksConfig;
const D: usize = 2;
type F = GoldilocksField;

pub fn run_local(
    st_provider: Arc<KVQlibmdbxStore>,
    contract_call_path: &str,
    private_key: &str,
) -> anyhow::Result<(
    SubmitUserEndCapNonProofInput<F>,
    ProofWithPublicInputs<GoldilocksField, C, D>,
)> {
    let contract_call_args: Vec<ContractCallArgs> =
        serde_json::from_str(&std::fs::read_to_string(contract_call_path)?)?;

    let latest_l2_block_state = st_provider.resolve_get_latest_l2_block_state()?;

    let main_circuits =
        QEDUPSStepCircuitManager::<C, D>::new_with_config(QED_NETWORK_MAGIC_REGTEST);

    let priv_key = QHashOut::<GoldilocksField>::from_str(&private_key)
        .map_err(|e| anyhow::format_err!("{}", e.to_string()))?;

    let wallet = SimpleQEDZKSignatureManager::<C, D>::new();

    let user_id = 0;

    let lps = QEDLocalProvingSessionStore::new_at(
        st_provider.clone(),
        GoldilocksField::from_noncanonical_u64(latest_l2_block_state.checkpoint_id),
        F::from_canonical_u64(user_id),
        GoldilocksField::ONE,
        UPS_SESSION_PROOF_TREE_HEIGHT as usize,
    );

    let mut circuit_info = SessionCircuitInfoStore::new();

    circuit_info.register_circuit(
        LocalCircuitType::SimpleZKSignature.into(),
        wallet.circuit.get_fingerprint(),
        wallet.circuit.get_verifier_config_ref().into(),
    );

    main_circuits.register_info(&mut circuit_info);

    let mut mgr = UserProvingSessionManager::<F, QEDHasher, _, C, D>::new(
        lps,
        circuit_info,
        main_circuits.ups_circuit_whitelist_root,
    )?;

    mgr.prove_ups_start(&main_circuits)?;

    for contract_call_arg in contract_call_args {
        prove_func(
            &st_provider.clone(),
            &main_circuits,
            &mut mgr,
            contract_call_arg.contract_id,
            &contract_call_arg.method_name,
            contract_call_arg
                .inputs
                .iter()
                .map(|x| GoldilocksField::from_noncanonical_u64(*x))
                .collect(),
        )
        .unwrap();
    }

    let new_nonce = GoldilocksField::from_noncanonical_u64(1);

    let sighash = mgr.get_sighash(QED_NETWORK_MAGIC_REGTEST, new_nonce);
    let signature_proof = wallet.zk_sign_for_private_key_value(priv_key, sighash)?;

    mgr.proof_tree_state
        .finalize_tree(&main_circuits.proof_tree_agg_circuits)?;

    let public_key_param = SimpleQEDPrivateKey::new(priv_key).get_public_key_param::<QEDHasher>();

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
    Ok((user_ec_input, end_cap_proof))
}
