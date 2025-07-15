use plonky2::plonk::config::PoseidonGoldilocksConfig;
use qed_core::job::{drain_queue::CheckpointDrainQueueEmitterAsyncImm, traits::QProofStoreAsyncImm};
use qed_store::{config::store_config::QEDFelt, node::coordinator::store_traits::QEDCoordinatorStoreReaderAsync};

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