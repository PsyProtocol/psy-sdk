use anyhow::Ok;
use fred::prelude::ClientLike;
use fred::prelude::Config;
use fred::prelude::ReconnectPolicy;
use fred::types::Builder;
use kvq::memory::arc_imm::KVQArcImmutableStoreWrapper;
use kvq_store_lmdbx::KVQlibmdbxStore;
use plonky2::plonk::config::PoseidonGoldilocksConfig;
use qed_core::job::{
    drain_queue::CheckpointDrainQueueConsumerAsyncImm,
    history_queue::CheckpointHistoryQueueEmitterAsyncImm,
    traits::{QProofStoreAsyncImm, QProofStoreReaderAsync, QProofStoreWriterAsyncImm},
    worker_queue::{ProvingDispatcher, ProvingWorkerListener, WorkerEventTransmitterAsyncImm},
};
use qed_node::coordinator::state::processor::CoordinatorConfig;
use qed_node::nimpl::proof_store_fred::ProofStoreFred;
use qed_node::nimpl::worker_queue_redis::redis_queue::{CPQueueNotification, CP_NOTIFICATIONS};
use qed_node::{
    coordinator::state::processor::CoordinatorProcessorContext,
    nimpl::worker_queue_redis::redis_queue::{CEQueueNotification, RedisQueue, CE_NOTIFICATIONS},
};
use qed_node_common::verifier::get_cached_generic_verifier;
use qed_store::{
    config::store_config::QEDFelt,
    node::coordinator::store_traits::{
        QEDCoordinatorStoreReaderAsync, QEDCoordinatorStoreWriterAsyncImm,
    },
    traits::qdatastore::qtreedata::QEDComboDataStoreReaderWriterSync,
};
use reth_libmdbx::{Environment, EnvironmentFlags, Mode, SyncMode, RW};
use std::{path::PathBuf, sync::Arc, time::Duration};

use crate::subcommand::CoordinatorProcessorArgs;

type C = PoseidonGoldilocksConfig;
const D: usize = 2;
type F = QEDFelt;

#[derive(Clone)]
pub struct CoordinatorProcessNode<
    SR: QEDCoordinatorStoreWriterAsyncImm<F> + QEDCoordinatorStoreReaderAsync<F>,
    DQ: CheckpointDrainQueueConsumerAsyncImm,
    HQ: CheckpointHistoryQueueEmitterAsyncImm,
    WQ: WorkerEventTransmitterAsyncImm,
    PS: QProofStoreAsyncImm + QProofStoreWriterAsyncImm + QProofStoreReaderAsync,
> {
    pub ctx: CoordinatorProcessorContext<SR, DQ, HQ, WQ, PS>,
    pub sync_queue: RedisQueue,
}

pub struct CoordinatorProcessNodeConfig {
    pool_size: usize,
    redis_url: String,
}

impl<
        SR: QEDCoordinatorStoreWriterAsyncImm<F> + QEDCoordinatorStoreReaderAsync<F>,
        DQ: CheckpointDrainQueueConsumerAsyncImm,
        HQ: CheckpointHistoryQueueEmitterAsyncImm,
        WQ: WorkerEventTransmitterAsyncImm,
        PS: QProofStoreAsyncImm,
    > CoordinatorProcessNode<SR, DQ, HQ, WQ, PS>
{
    pub fn new(
        ctx: CoordinatorProcessorContext<SR, DQ, HQ, WQ, PS>,
        sync_queue: RedisQueue,
    ) -> Self {
        Self { ctx, sync_queue }
    }

    pub async fn wait_for_produce_block(&mut self) -> anyhow::Result<bool> {
        match self.sync_queue.pop_one(CE_NOTIFICATIONS)? {
            Some(message) => {
                let notify_message = serde_json::from_slice::<CEQueueNotification>(&message)?;
                match notify_message {
                    CEQueueNotification::StartProduceBlock => Ok(true),
                    _ => Ok(false),
                }
            }
            None => Ok(false),
        }
    }

    pub async fn notify_sync(&mut self) -> anyhow::Result<()> {
        self.sync_queue
            .dispatch(CP_NOTIFICATIONS, CPQueueNotification::StartSync)?;
        Ok(())
    }
}

impl
    CoordinatorProcessNode<
        KVQArcImmutableStoreWrapper<KVQlibmdbxStore<RW>>,
        ProofStoreFred,
        ProofStoreFred,
        ProofStoreFred,
        ProofStoreFred,
    >
{
    pub async fn new_with_config(cp_config: CoordinatorProcessNodeConfig) -> anyhow::Result<Self> {
        let config = Config::from_url(&cp_config.redis_url)?;
        let pool = Builder::from_config(config)
            .with_connection_config(|config| {
                config.connection_timeout = Duration::from_secs(10);
            })
            // use exponential backoff, starting at 100 ms and doubling on each failed attempt up to 30 sec
            .set_policy(ReconnectPolicy::new_exponential(0, 100, 30_000, 2))
            .build_pool(cp_config.pool_size)?;

        pool.init().await?;

        let q = ProofStoreFred::new(pool.clone(), "wq1".to_string(), "nq1".to_string());
        let flags = EnvironmentFlags {
            no_sub_dir: false,
            mode: Mode::ReadWrite {
                sync_mode: SyncMode::Durable,
            },
            coalesce: true,
            ..Default::default()
        };

        let env = Environment::builder()
            .set_max_dbs(10)
            .set_flags(flags)
            .open(PathBuf::new().join("db").as_path())?;

        let txn = env.begin_rw_txn()?;
        let store_reader: KVQArcImmutableStoreWrapper<KVQlibmdbxStore<RW>> =
            KVQArcImmutableStoreWrapper::<KVQlibmdbxStore<RW>>::new(KVQlibmdbxStore::new(
                txn.clone(),
                None,
            )?);

        store_reader.initialize_store()?;

        let coord_config = CoordinatorConfig::get_standard(0);

        let qps = Arc::new(q.clone());

        let st = Arc::new(store_reader.dup());

        let proof_verifier = Arc::new(get_cached_generic_verifier::<C, D>());

        let coordinator_processor_ctx = CoordinatorProcessorContext::new(
            coord_config,
            Arc::clone(&st),
            qps.clone(),
            qps.clone(),
            qps.clone(),
            qps.clone(),
            Arc::clone(&proof_verifier),
        )
        .await?;

        let sync_queue = RedisQueue::new(&cp_config.redis_url)?;

        Ok(CoordinatorProcessNode::new(
            coordinator_processor_ctx,
            sync_queue,
        ))
    }
}

pub async fn run(args: CoordinatorProcessorArgs) -> anyhow::Result<()> {
    let mut coordinator_processor =
        CoordinatorProcessNode::new_with_config(CoordinatorProcessNodeConfig {
            pool_size: args.pool_size as usize,
            redis_url: args.redis_uri,
        })
        .await?;
    loop {
        // wait for produceblock message from coordinator edge
        if coordinator_processor.wait_for_produce_block().await? {
            coordinator_processor.ctx.build_block().await?;
            // send sync message to coordinator edge
            coordinator_processor.notify_sync().await?;
        }
        tokio::time::sleep(Duration::from_millis(750)).await;
    }
}
