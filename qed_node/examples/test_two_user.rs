use fred::prelude::*;
use kvq::memory::simple::KVQSimpleMemoryBackingStore;
use qed_store::node::coordinator::QEDCoordinatorStoreWriterAsyncImm;
use qed_core::
    utils::debug_timer::DebugTimer
;
use qed_crypto::{
    common::simple_circuit_library::SimpleCircuitLibrary, signature::zk::data::ZKPublicKeyInfo,
};
use qed_node::{
    coordinator::state::{
        edge::CoordinatorEdgeContext,
        processor::{CoordinatorConfig, CoordinatorProcessorContext},
    },
    worker::simple_async_coord::SimpleAsyncCoordinatorWorker,
};
use qed_store::queue::ProofStoreFred;
use qed_node::common::verifier::get_cached_generic_verifier;
use qed_rollup_circuit::coordinator::coordinator_helper::QEDCoordinatorCircuitManager;
use qed_data::traits::qdatastore::qtreedata::QEDComboDataStoreReaderWriterSync;
use std::{sync::Arc, time::Duration};


use plonky2::{
    field::goldilocks_field::GoldilocksField,
    plonk::
        config::PoseidonGoldilocksConfig
    ,
};
use qed_core::
    data::qhashout::QHashOut
;
use qed_store::queue::new_fred_pool;

async fn run_fred_test3() -> anyhow::Result<()> {
    type C = PoseidonGoldilocksConfig;
    const D: usize = 2;
    let mut timer = DebugTimer::new("dq_rust_2v2");
    timer.lap("start");

    let pool = new_fred_pool("redis://127.0.0.1:6379",8).await?;
    timer.lap("connected to redis");

    let q = ProofStoreFred::new(pool, "wq1".to_string());

    let store_reader = Arc::new(KVQSimpleMemoryBackingStore::new());

    store_reader.initialize_store().await?;
    //let worker_count = 16usize;
    //let items_per_worker = 2000usize;

    let coord_config = CoordinatorConfig::get_standard(0);

    let qps = Arc::new(q.clone());

    let st = Arc::new(store_reader);

    timer.lap("initialized store");
    let proof_verifier = Arc::new(get_cached_generic_verifier::<C, D>());
    timer.lap("created proof verifier");

    let coordinator_worker_circuits =
        QEDCoordinatorCircuitManager::<C, D>::new_with_library(&proof_verifier.library);
    timer.lap("built coordinator worker circuits");

    let coordinator_edge_node =
        CoordinatorEdgeContext::new(
            coord_config,
            Arc::clone(&st),
            qps.clone(),
            qps.clone(),
            Arc::clone(&proof_verifier),
        )
        .await?;

    let mut coordinator_processor_node = CoordinatorProcessorContext::new(
        coord_config,
        Arc::clone(&st),
        qps.clone(),
        qps.clone(),
        qps.clone(),
        qps.clone(),
        Arc::clone(&proof_verifier),
    )
    .await?;
    timer.lap("created coordinator nodes");

    let user_a_info = ZKPublicKeyInfo {
        public_key_param: QHashOut::rand(),
        fingerprint: QHashOut::rand(),
    };

    coordinator_edge_node
        .handle_process_regsiter_user(user_a_info)
        .await?;

    let user_b_info = ZKPublicKeyInfo {
        public_key_param: QHashOut::rand(),
        fingerprint: QHashOut::rand(),
    };

    coordinator_edge_node
        .handle_process_regsiter_user(user_b_info)
        .await?;
    timer.lap("sent requests");

    coordinator_processor_node.build_block(0).await?;
    timer.lap("built block");

    timer.lap("started up");

    SimpleAsyncCoordinatorWorker::run_worker::<
        _,
        _,
        SimpleCircuitLibrary<GoldilocksField>,
        QEDCoordinatorCircuitManager<C, D>,
        C,
        D,
    >(
        &q,
        &q,
        &coordinator_worker_circuits,
        &proof_verifier.library,
    )
    .await?;
    timer.lap("finished jobs");

    Ok(())
}
#[tokio::main]
async fn main() {
    run_fred_test3().await.unwrap();
}
