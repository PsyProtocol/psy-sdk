use std::{sync::Arc, time::Duration};

use fred::prelude::*;
use kvq::memory::simple::KVQSimpleMemoryBackingStore;
use plonky2::{field::goldilocks_field::GoldilocksField, plonk::config::PoseidonGoldilocksConfig};
use psy_core::{data::qhashout::QHashOut, utils::debug_timer::DebugTimer};
use psy_crypto::{common::simple_circuit_library::SimpleCircuitLibrary, signature::zk::data::ZKPublicKeyInfo};
use psy_data::traits::qdatastore::qtreedata::PsyComboDataStoreReaderWriterSync;
use psy_network_circuit::coordinator::coordinator_helper::PsyCoordinatorCircuitManager;
use psy_node::{
    common::verifier::get_cached_generic_verifier,
    coordinator::state::{
        edge::CoordinatorEdgeContext,
        processor::{CoordinatorConfig, CoordinatorProcessorContext},
    },
    worker::simple_async_coord::SimpleAsyncCoordinatorWorker,
};
use psy_store::{
    node::coordinator::PsyCoordinatorStoreWriterAsyncImm,
    queue::{
        new_redis_async_pool,
        task_queue::{QProvingTaskStore, QProvingTaskStoreImpl},
        ProofStoreRedis,
    },
    store::journal::JournalStore,
};

async fn run_test3() -> anyhow::Result<()> {
    type C = PoseidonGoldilocksConfig;
    const D: usize = 2;
    let mut timer = DebugTimer::new("dq_rust_2v2");
    timer.lap("start");

    let q = ProofStoreRedis::new("redis://127.0.0.1:6379", "wq1".to_string()).await?;
    timer.lap("connected to redis");

    let store_reader = Arc::new(KVQSimpleMemoryBackingStore::new());

    store_reader.initialize_store(None).await?;
    //let worker_count = 16usize;
    //let items_per_worker = 2000usize;

    let coord_config = CoordinatorConfig::get_standard();

    let qps = Arc::new(q.clone());

    let st = Arc::new(store_reader.clone());

    timer.lap("initialized store");

    let task_store = Arc::new(
        QProvingTaskStoreImpl::new("redis://127.0.0.1/", 10, "biz_key")
            .await
            .expect("Failed to create JobTaskStore"),
    );

    let proof_verifier = Arc::new(get_cached_generic_verifier::<C, D>());
    timer.lap("created proof verifier");

    use psy_core::config::network_constants::get_default_worker_public_key;
    let coordinator_worker_circuits =
        PsyCoordinatorCircuitManager::<C, D>::new_with_library(&proof_verifier.library, get_default_worker_public_key::<GoldilocksField>());
    timer.lap("built coordinator worker circuits");

    let coordinator_edge_node =
        CoordinatorEdgeContext::new(coord_config, Arc::clone(&st), qps.clone(), qps.clone(), Arc::clone(&proof_verifier)).await?;

    let mut coordinator_processor_node = CoordinatorProcessorContext::new(
        coord_config,
        Arc::new(JournalStore::new(store_reader.clone())),
        qps.clone(),
        qps.clone(),
        qps.clone(),
        qps.clone(),
        task_store.clone(),
        Arc::clone(&proof_verifier),
        None,
        None,
    )
    .await?;
    timer.lap("created coordinator nodes");

    let user_a_info = ZKPublicKeyInfo {
        public_key_param: QHashOut::rand(),
        fingerprint: QHashOut::rand(),
    };

    coordinator_edge_node.handle_process_regsiter_user(user_a_info).await?;

    let user_b_info = ZKPublicKeyInfo {
        public_key_param: QHashOut::rand(),
        fingerprint: QHashOut::rand(),
    };

    coordinator_edge_node.handle_process_regsiter_user(user_b_info).await?;
    timer.lap("sent requests");

    coordinator_processor_node.build_block(0).await?;
    timer.lap("built block");

    timer.lap("started up");

    SimpleAsyncCoordinatorWorker::run_worker::<_, _, SimpleCircuitLibrary<GoldilocksField>, PsyCoordinatorCircuitManager<C, D>, C, D>(
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
    run_test3().await.unwrap();
}
