use plonky2::{
    field::{goldilocks_field::GoldilocksField, types::Field},
    hash::poseidon::PoseidonHash,
    plonk::config::PoseidonGoldilocksConfig,
};
use qed_core::config::network_constants::UPS_SESSION_PROOF_TREE_HEIGHT;
use qed_prover::{
    dpn::{circuits::cfc::DapenContractFunctionCircuit, data::cfc_code_definition_to_dapen_fc},
    ups::{circuit_manager::core::QEDUPSStepCircuitManager, session::UserProvingSessionManager},
};
use qed_store::store::imm::{
    cmd::QSRCmdGetContractCodeDefinition, cmd_processor::QEDReadCommandProcessorSync,
};

type C = PoseidonGoldilocksConfig;
const D: usize = 2;
type F = GoldilocksField;

#[maybe_async::maybe_async]
pub async fn prove_func<R: QEDReadCommandProcessorSync<F> + Send + Sync>(
    st: &R,
    circuit_mgr: &QEDUPSStepCircuitManager<C, D>,
    mgr: &mut UserProvingSessionManager<F, PoseidonHash, R, C, D>,
    contract_id: u64,
    fn_name: &str,
    inputs: Vec<F>,
) -> anyhow::Result<()> {
    let contract_code =
        st.resolve_get_contract_code(&QSRCmdGetContractCodeDefinition { contract_id }).await?;

    for (i, func) in contract_code.functions.iter().enumerate() {
        let dapen_fc = cfc_code_definition_to_dapen_fc(&func)?;
        let dapen_fc_circuit = DapenContractFunctionCircuit::<C, D>::new(
            &dapen_fc,
            contract_code.state_tree_height as usize,
            UPS_SESSION_PROOF_TREE_HEIGHT as usize,
            false,
        );
        if dapen_fc.name == fn_name {
            return mgr.prove_contract_call(
                circuit_mgr,
                F::from_canonical_u64(contract_id),
                i as u32,
                &dapen_fc_circuit,
                &dapen_fc,
                inputs,
            ).await;
        }
    }
    anyhow::bail!("unable to find function {}", fn_name);
}
