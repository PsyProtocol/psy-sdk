use crate::{C, D};
use kvq::memory::arc_imm::KVQArcImmutableStoreWrapper;
use kvq_store_lmdbx::KVQlibmdbxStore;
use plonky2::field::goldilocks_field::GoldilocksField;
use qed_core::job::id::QProvingJobDataID;
use qed_crypto::common::generic_circuit_verifier::GenericCircuitVerifier;
use qed_crypto::common::simple_circuit_library::SimpleCircuitLibrary;
use qed_node::nimpl::proof_store_fred::ProofStoreFred;
use qed_node::realm::state::processor::{RealmConfig, RealmProcessorContext};
use qed_node::worker::simple_async_realm::SimpleAsyncRealmWorker;
use qed_rollup_circuit::coordinator::coordinator_helper::QEDCoordinatorCircuitManager;
use qed_store::node::coordinator::store_traits::QEDCoordinatorStoreReaderAsync;
use reth_libmdbx::RW;
use std::sync::Arc;

pub struct RealmProcessor {
    realm_config: RealmConfig,
    realm_qps: ProofStoreFred,
    store_reader: KVQArcImmutableStoreWrapper<KVQlibmdbxStore<RW>>,
    proof_verifier: Arc<GenericCircuitVerifier<C, D>>,
    coordinator_worker_circuits: QEDCoordinatorCircuitManager<C, D>,
    checkpoint_id: u64,
}

impl RealmProcessor {
    pub async fn build_block(&mut self) -> anyhow::Result<QProvingJobDataID> {
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

        let sync1 = st
            .get_checkpoint_sync_info_compact(self.checkpoint_id)
            .await?;
        realm_processor_node.handle_checkpoint_sync(sync1).await?;
        realm_processor_node.build_block().await?;
        let realm_worker_output_job_id = self.run_worker_until_done().await?;
        self.checkpoint_id += 1;
        Ok(realm_worker_output_job_id)
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
}
