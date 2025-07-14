use plonky2::{field::goldilocks_field::GoldilocksField, plonk::config::PoseidonGoldilocksConfig};
use qed_core::job::{drain_queue::CheckpointDrainQueueEmitterAsyncImm, traits::QProofStoreAsyncImm};
use qed_data::config::store_config::QEDFelt;
use qed_store::node::coordinator::QEDCoordinatorStoreReaderAsync;

use super::state::edge::CoordinatorEdgeContext;


type F = QEDFelt;
type C = PoseidonGoldilocksConfig;
const D: usize = 2;


#[derive(Clone)]
pub struct CoordinatorDemoEdgeNode<

SR: QEDCoordinatorStoreReaderAsync<F>,
DQ: CheckpointDrainQueueEmitterAsyncImm,
PS: QProofStoreAsyncImm,
> {
    pub ctx: CoordinatorEdgeContext<SR, DQ, PS>,

}
