mod config;
mod types;

pub use config::*;
pub use types::*;

use crate::{C, D, F};
use fred::prelude::{Builder, Config, ReconnectPolicy};
use kvq::memory::arc_imm::KVQArcImmutableStoreWrapper;
use kvq_store_lmdbx::KVQlibmdbxStore;
use plonky2::field::goldilocks_field::GoldilocksField;
use qed_core::config::network_constants::QED_CHECKPOINT_SYNC_INFO_COMPACT_DRAIN_QUEUE_CHANNEL;
use qed_core::job::history_queue::{
    CheckpointHistoryQueueConsumerAsyncImm, CheckpointHistoryQueueEmitterAsyncImm,
};
use qed_core::job::id::QProvingJobDataID;
use qed_crypto::common::generic_circuit_verifier::GenericCircuitVerifier;
use qed_crypto::common::simple_circuit_library::SimpleCircuitLibrary;
use qed_data::qsync::coordinator::QEDCheckpointSyncInfoCompact;
use qed_node::nimpl::proof_store_fred::ProofStoreFred;
use qed_node::realm::state::processor::{RealmConfig, RealmProcessorContext};
use qed_node::worker::simple_async_realm::SimpleAsyncRealmWorker;
use qed_node_common::verifier::get_cached_generic_verifier;
use qed_rollup_circuit::coordinator::coordinator_helper::QEDCoordinatorCircuitManager;
use qed_store::node::coordinator::store_traits::QEDCoordinatorStoreReaderAsync;
use reth_libmdbx::{Environment, EnvironmentFlags, Mode, SyncMode, RW};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::task::JoinHandle;

type KVQArcImmutableStore = KVQArcImmutableStoreWrapper<KVQlibmdbxStore<RW>>;
type ConcreteRealmProcessorContext = RealmProcessorContext<
    KVQArcImmutableStore,
    ProofStoreFred,
    ProofStoreFred,
    ProofStoreFred,
    ProofStoreFred,
>;

#[derive(Debug)]
pub struct RealmProcessor {
    pub realm_config: RealmConfig,
    pub realm_qps: ProofStoreFred,
    pub store_reader: KVQArcImmutableStore,
    pub proof_verifier: Arc<GenericCircuitVerifier<C, D>>,
    pub coordinator_worker_circuits: QEDCoordinatorCircuitManager<C, D>,
}

impl RealmProcessor {
    pub async fn new(config: RealmProcessorConfig) -> anyhow::Result<Self> {
        let pool_size = 8;
        let redis_config = Config::from_url(&config.redis_url)?;
        let pool = Builder::from_config(redis_config)
            .with_connection_config(|config| {
                config.connection_timeout = Duration::from_secs(10);
            })
            // use exponential backoff, starting at 100 ms and doubling on each failed attempt up to 30 sec
            .set_policy(ReconnectPolicy::new_exponential(0, 100, 30_000, 2))
            .build_pool(pool_size)?;
        let realm_qps = ProofStoreFred::new(
            pool,
            config.worker_queue_suffix,
            config.notifications_queue_suffix,
        );

        let realm_config = RealmConfig::get_standard(config.rpc_node_id, config.realm_id);

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
        let store_reader = KVQArcImmutableStore::new(KVQlibmdbxStore::new(txn.clone(), None)?);

        let proof_verifier = Arc::new(get_cached_generic_verifier::<C, D>());

        let coordinator_worker_circuits =
            QEDCoordinatorCircuitManager::<C, D>::new_with_library(&proof_verifier.library);

        let processor = RealmProcessor {
            realm_config,
            realm_qps,
            store_reader,
            proof_verifier,
            coordinator_worker_circuits,
        };
        Ok(processor)
    }

    pub async fn start(mut self) -> anyhow::Result<JoinHandle<()>> {
        let st = Arc::new(self.store_reader.dup());
        let realm_qps = Arc::new(self.realm_qps.clone());
        let mut realm_processor_node = RealmProcessorContext::new(
            self.realm_config,
            st.clone(),
            realm_qps.clone(),
            realm_qps.clone(),
            realm_qps.clone(),
            realm_qps.clone(),
            self.proof_verifier.clone(),
        )
        .await?;
        let handle = tokio::spawn(async move {
            loop {
                match self.wait_for_next_checkpoint().await {
                    Ok(checkpoint) => {
                        tracing::info!("Checkpoint received: {:?}", checkpoint);
                        if let Err(err) = realm_processor_node
                            .handle_checkpoint_sync(checkpoint)
                            .await
                        {
                            tracing::error!("Error handling checkpoint sync: {:?}", err);
                        }
                    }
                    Err(err) => {
                        tracing::error!("Error waiting for next checkpoint: {:?}", err);
                        continue;
                    }
                }
                let proving_data_job_id: ProvingJobDataId =
                    match self.build_block(&mut realm_processor_node).await {
                        Ok(job_id) => job_id,
                        Err(err) => {
                            tracing::error!("Error building block: {:?}", err);
                            continue;
                        }
                    };
                // Send the job id to the channel for the next step
                if let Err(err) = self.realm_qps.chq_push_imm(proving_data_job_id).await {
                    tracing::error!("Error chq_push_imm: {:?}", err);
                };
            }
        });
        Ok(handle)
    }

    pub async fn build_block(
        &mut self,
        context: &mut ConcreteRealmProcessorContext,
    ) -> anyhow::Result<ProvingJobDataId> {
        let checkpoint = self.wait_for_next_checkpoint().await?;
        context.handle_checkpoint_sync(checkpoint).await?;
        context.build_block().await?;
        let realm_worker_output_job_id = self.run_worker_until_done().await?;
        let checkpoint_id = self.checkpoint_id().await?;
        Ok(ProvingJobDataId::new(
            checkpoint_id,
            realm_worker_output_job_id,
        ))
    }

    pub async fn checkpoint_id(&self) -> anyhow::Result<u64> {
        Ok(self
            .store_reader
            .get_latest_l2_block_state()
            .await?
            .checkpoint_id)
    }

    pub async fn run_worker_until_done(&self) -> anyhow::Result<QProvingJobDataID> {
        SimpleAsyncRealmWorker::run_worker_until_done::<
            _,
            _,
            SimpleCircuitLibrary<GoldilocksField>,
            QEDCoordinatorCircuitManager<C, D>,
            C,
            D,
        >(
            &self.realm_qps.clone(),
            &self.realm_qps.clone(),
            &self.coordinator_worker_circuits,
            &self.proof_verifier.library,
        )
        .await
    }

    pub async fn wait_for_next_checkpoint(
        &self,
    ) -> anyhow::Result<QEDCheckpointSyncInfoCompact<F>> {
        let channel_id = QED_CHECKPOINT_SYNC_INFO_COMPACT_DRAIN_QUEUE_CHANNEL;
        let checkpoint_id = self.checkpoint_id().await?;
        self.realm_qps
            .wait_for_next_item_imm(channel_id, checkpoint_id)
            .await
    }
}
