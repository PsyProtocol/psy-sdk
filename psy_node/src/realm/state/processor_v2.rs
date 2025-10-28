use std::sync::Arc;
use plonky2::plonk::config::PoseidonGoldilocksConfig;
use psy_crypto::common::generic_circuit_verifier::GenericCircuitVerifier;
use psy_store::queue::task_queue::QProvingTaskStoreImpl;
use psy_store::store::QEDStore;
use crate::realm::state::processor::RealmConfig;

type C = PoseidonGoldilocksConfig;
const D: usize = 2;

#[derive(Clone)]
pub struct RealmProcessorContextV2<PS> {
    pub realm_config: RealmConfig,
    pub proof_store: Arc<PS>,
    pub store: QEDStore,
    pub proof_verifier: Arc<GenericCircuitVerifier<C, D>>,
    pub task_store: Arc<QProvingTaskStoreImpl>,
    pub config_path: String,
}
