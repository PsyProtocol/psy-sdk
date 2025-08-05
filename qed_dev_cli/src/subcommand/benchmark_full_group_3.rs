use fred::prelude::*;
use kvq::{
    memory::simple::KVQSimpleMemoryBackingStore,
    traits::KVQBinaryStore,
};
use std::sync::Arc;
use super::super::test_helpers::{
    contract::{gen_test_contract, SimpleTestContract},
    ups::ExampleDemoUserInfoStore,
};
use qed_common_circuit::circuits::{
    traits::qstandard::QStandardCircuit, zk_signature3::manager::SimpleQEDZKSignatureManager,
};
use qed_core::{
    config::network_constants::{QED_NETWORK_MAGIC_REGTEST, UPS_SESSION_PROOF_TREE_HEIGHT},
    job::{
        drain_queue::{CheckpointDrainQueueConsumerAsyncImm, CheckpointDrainQueueEmitterAsyncImm},
        history_queue::{
            CheckpointHistoryQueueConsumerAsyncImm, CheckpointHistoryQueueEmitterAsyncImm,
        },
        id::ProvingJobCircuitType,
        traits::{QProofStoreAsyncImm, QProofStoreReaderAsync, QProofStoreWriterAsyncImm},
        worker_queue::WorkerEventTransmitterAsyncImm,
    },
    ups::circuits::{LocalCircuitId, LocalCircuitType},
    utils::debug_timer::DebugTimer,
};
use qed_crypto::{
    common::{
        generic_circuit_verifier::GenericCircuitVerifier,
        simple_circuit_library::SimpleCircuitLibrary,
    },
    hash::traits::{
        hasher::{FieldQHasher, MerkleZeroHasher},
        qhashable::QFieldHashable,
    },
    signature::zk::{data::ZKPublicKeyInfo, wallet::SimpleQEDPrivateKey},
};
use qed_data::guta::api::{
    GUTARealmCheckpointResult, SubmitGUTARealmResultAPINoProofInput, SubmitUserEndCapProofAPIInput,
};
use qed_node::{
    coordinator::{
        state::{
            edge::CoordinatorEdgeContext,
            processor::{CoordinatorConfig, CoordinatorProcessorContext},
        },
    },
    realm::state::{
        edge::RealmEdgeContext,
        processor::{RealmConfig, RealmProcessorContext},
    },
    worker::{
        simple_async_coord::SimpleAsyncCoordinatorWorker,
        simple_async_realm::SimpleAsyncRealmWorker,
    },
};
use qed_node::common::verifier::get_cached_generic_verifier;
use qed_prover::{local::provider::ProveProxyRpcTrait, ups::{
    circuit_manager::core::{QCircuitManager, QEDUPSStepCircuitManager}, session::UserProvingSessionManager,
}};
use qed_rollup_circuit::coordinator::coordinator_helper::QEDCoordinatorCircuitManager;
use qed_data::{
    config::store_config::{QEDFelt, QEDHasher},
    traits::qdatastore::qtreedata::QEDComboDataStoreReaderWriterSync,
};
use qed_store::{
    controllers::local::{
        proving_session::QEDLocalProvingSessionStore, session_info::SessionCircuitInfoStore,
    },
    node::{
        coordinator::{
            QEDCoordinatorStoreReaderAsync, QEDCoordinatorStoreWriterAsyncImm,
        },
        realm::QEDRealmStoreReaderAsync,
    },
    queue::ProofStoreFred,
};

use std::time::Duration;

use plonky2::{
    field::{extension::Extendable, goldilocks_field::GoldilocksField, types::Field},
    hash::hash_types::{HashOut, RichField},
    plonk::config::{AlgebraicHasher, GenericConfig, PoseidonGoldilocksConfig},
};
use qed_core::data::qhashout::QHashOut;
struct TestGrouping<
    CSR: QEDCoordinatorStoreReaderAsync<F> + Send + Sync + KVQBinaryStore,
    CDQ: CheckpointDrainQueueEmitterAsyncImm + Send + Sync,
    CPS: QProofStoreAsyncImm + Send + Sync,
    CPSR: QEDCoordinatorStoreWriterAsyncImm<F>
        + QEDCoordinatorStoreReaderAsync<F>
        + Send
        + Sync
        + KVQBinaryStore,
    CPDQ: CheckpointDrainQueueConsumerAsyncImm + Send + Sync,
    CPHQ: CheckpointHistoryQueueEmitterAsyncImm,
    CPPS: QProofStoreAsyncImm + QProofStoreWriterAsyncImm + QProofStoreReaderAsync,
    CPWQ: WorkerEventTransmitterAsyncImm,
    RSR: QEDRealmStoreReaderAsync<F> + Send + Sync + KVQBinaryStore,
    RDQ: CheckpointDrainQueueEmitterAsyncImm,
    RPS: QProofStoreAsyncImm,
    RPSR: QEDCoordinatorStoreWriterAsyncImm<F>
        + QEDCoordinatorStoreReaderAsync<F>
        + Send
        + Sync
        + KVQBinaryStore,
    RPDQ: CheckpointDrainQueueConsumerAsyncImm,
    RPHQ: CheckpointHistoryQueueEmitterAsyncImm + CheckpointHistoryQueueConsumerAsyncImm,
    RPWQ: WorkerEventTransmitterAsyncImm,
    RPPS: QProofStoreAsyncImm + QProofStoreWriterAsyncImm + QProofStoreReaderAsync,
> {
    coord_circuits: QEDCoordinatorCircuitManager<C, D>,
    coord_edge: CoordinatorEdgeContext<CSR, CDQ, CPS>,
    coord_proc: CoordinatorProcessorContext<CPSR, CPDQ, CPHQ, CPWQ, CPPS>,

    realm_edge: RealmEdgeContext<RSR, RDQ, RPS>,
    realm_proc: RealmProcessorContext<RPSR, RPDQ, RPHQ, RPWQ, RPPS>,

    coord_w_queue_store: ProofStoreFred,
    realm_w_queue_store: ProofStoreFred,

    proof_verifier: Arc<GenericCircuitVerifier<C, D>>,
}

type C = PoseidonGoldilocksConfig;
type F = GoldilocksField;
const D: usize = 2;
impl<
        CSR: QEDCoordinatorStoreReaderAsync<F> + Send + Sync + KVQBinaryStore,
        CDQ: CheckpointDrainQueueEmitterAsyncImm + Send + Sync,
        CPS: QProofStoreAsyncImm + Send + Sync,
        CPSR: QEDCoordinatorStoreWriterAsyncImm<F>
            + QEDCoordinatorStoreReaderAsync<F>
            + Send
            + Sync
            + KVQBinaryStore,
        CPDQ: CheckpointDrainQueueConsumerAsyncImm + Send + Sync,
        CPHQ: CheckpointHistoryQueueEmitterAsyncImm,
        CPPS: QProofStoreAsyncImm + QProofStoreWriterAsyncImm + QProofStoreReaderAsync,
        CPWQ: WorkerEventTransmitterAsyncImm,
        RSR: QEDRealmStoreReaderAsync<F> + Send + Sync + KVQBinaryStore,
        RDQ: CheckpointDrainQueueEmitterAsyncImm,
        RPS: QProofStoreAsyncImm,
        RPSR: QEDCoordinatorStoreWriterAsyncImm<F>
            + QEDCoordinatorStoreReaderAsync<F>
            + Send
            + Sync
            + KVQBinaryStore,
        RPDQ: CheckpointDrainQueueConsumerAsyncImm,
        RPHQ: CheckpointHistoryQueueEmitterAsyncImm + CheckpointHistoryQueueConsumerAsyncImm,
        RPWQ: WorkerEventTransmitterAsyncImm,
        RPPS: QProofStoreAsyncImm + QProofStoreWriterAsyncImm + QProofStoreReaderAsync,
    >
    TestGrouping<
        CSR,
        CDQ,
        CPS,
        CPSR,
        CPDQ,
        CPHQ,
        CPPS,
        CPWQ,
        RSR,
        RDQ,
        RPS,
        RPSR,
        RPDQ,
        RPHQ,
        RPWQ,
        RPPS,
    >
{
    pub async fn deploy_contract(&mut self) -> anyhow::Result<SimpleTestContract<C, D>> {
        let (contract_helper, contract_deploy_cmd) = gen_test_contract::<C, D>(QHashOut::rand())?;
        self.coord_edge
            .handle_deploy_contract(contract_deploy_cmd)
            .await?;

        Ok(contract_helper)
    }

    pub async fn produce_block(&mut self) -> anyhow::Result<()> {
        self.realm_proc.build_block().await?;
        let realm_worker_output_job_id = SimpleAsyncRealmWorker::run_worker_until_done::<
            _,
            _,
            SimpleCircuitLibrary<GoldilocksField>,
            QEDCoordinatorCircuitManager<C, D>,
            C,
            D,
        >(
            &self.realm_w_queue_store,
            &self.realm_w_queue_store,
            &self.coord_circuits,
            &self.proof_verifier.library,
        )
        .await?;

        let realm_result: GUTARealmCheckpointResult<QEDFelt> = bincode::deserialize(
            &self
                .realm_w_queue_store
                .get_bytes_by_id(realm_worker_output_job_id)
                .await?,
        )
        .map_err(|e| anyhow::anyhow!("{:?}", e))?;
        println!(
            "get realm_proof: {:?}",
            realm_result.proof_id.get_output_id()
        );

        if realm_result.proof_id.circuit_type != ProvingJobCircuitType::GUTANoChange {
            let realm_proof = self
                .realm_w_queue_store
                .get_proof_by_id(realm_result.proof_id.get_output_id())
                .await?;

            println!("got realm proof");

            self.coord_edge
                .handle_recv_guta_from_realm(
                    SubmitGUTARealmResultAPINoProofInput {
                        realm_id: 0,
                        checkpoint_id: realm_result.checkpoint_id,
                        guta_stats: realm_result.guta_stats,
                        top_line_proof: realm_result.top_line_proof,
                        checkpoint_tree_root: realm_result.checkpoint_tree_root,
                        circuit_type: realm_result.proof_id.circuit_type,
                    },
                    &realm_proof,
                )
                .await?;
        }

        self.coord_proc.build_block().await?;
        SimpleAsyncCoordinatorWorker::run_worker_until_done::<
            _,
            _,
            SimpleCircuitLibrary<GoldilocksField>,
            QEDCoordinatorCircuitManager<C, D>,
            C,
            D,
        >(
            &self.coord_w_queue_store,
            &self.coord_w_queue_store,
            &self.coord_circuits,
            &self.proof_verifier.library,
        )
        .await?;

        let latest = QEDCoordinatorStoreReaderAsync::get_latest_l2_block_state(&self.coord_proc.store).await?;
        let new_sync = self
            .coord_proc
            .store
            .get_checkpoint_sync_info_compact(latest.checkpoint_id)
            .await?;

        self.realm_proc.handle_checkpoint_sync(new_sync).await?;
        Ok(())
    }
}

async fn run_fred_test3() -> anyhow::Result<()> {
    type C = PoseidonGoldilocksConfig;
    const D: usize = 2;
    let mut timer = DebugTimer::new("dq_rust_2v2");
    timer.lap("start");

    let pool_size = 8;
    let config = Config::from_url("redis://127.0.0.1:6379")?;
    let pool = Builder::from_config(config)
        .with_connection_config(|config| {
            config.connection_timeout = Duration::from_secs(10);
        })
        // use exponential backoff, starting at 100 ms and doubling on each failed attempt up to 30 sec
        .set_policy(ReconnectPolicy::new_exponential(0, 100, 30_000, 2))
        .build_pool(pool_size)?;

    pool.init().await?;
    timer.lap("connected to redis");

    let q = ProofStoreFred::new(pool.clone(), "wq1".to_string());
    let realm_q = ProofStoreFred::new(pool, "rwq1".to_string());

    let store_reader: Arc<KVQSimpleMemoryBackingStore> =
        Arc::new(KVQSimpleMemoryBackingStore::new());

    let mut checkpoint_id =
        QEDCoordinatorStoreWriterAsyncImm::initialize_store(&store_reader).await?;
    //let worker_count = 16usize;
    //let items_per_worker = 2000usize;

    let coord_config = CoordinatorConfig::get_standard(0);

    let qps = Arc::new(q.clone());

    let realm_qps = Arc::new(realm_q.clone());

    let st = Arc::new(store_reader.clone());

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
    timer.lap("finished building wallet/zksig/helper circuits");
    timer.lap("deployed contract and registered users");

    coordinator_processor_node.build_block().await?;

    SimpleAsyncCoordinatorWorker::run_worker_until_done::<
        _,
        _,
        SimpleCircuitLibrary<GoldilocksField>,
        QEDCoordinatorCircuitManager<C, D>,
        C,
        D,
    >(
        &q.clone(),
        &q.clone(),
        &coordinator_worker_circuits,
        &proof_verifier.library,
    ).await?;
    /*
    coordinator_processor_node.build_block().await?;

    SimpleAsyncCoordinatorWorker::run_worker_until_done::<
        _,
        _,
        SimpleCircuitLibrary<GoldilocksField>,
        QEDCoordinatorCircuitManager<C, D>,
        C,
        D,
    >(
        &q.clone(),
        &q.clone(),
        &coordinator_worker_circuits,
        &proof_verifier.library,
    ).await?;*/

    let realm_config = RealmConfig::get_standard(0, 0);

    let realm_edge_node = RealmEdgeContext::new(
        realm_config,
        st.clone(),
        realm_qps.clone(),
        realm_qps.clone(),
        Arc::clone(&proof_verifier),
    )
    .await?;
    let mut realm_processor_node = RealmProcessorContext::new(
        realm_config,
        st.clone(),
        realm_qps.clone(),
        realm_qps.clone(),
        realm_qps.clone(),
        realm_qps.clone(),
        Arc::clone(&proof_verifier),
    )
    .await?;
    realm_edge_node.handle_recv_checkpoint_sync(coordinator_processor_node.store.get_checkpoint_sync_info_compact(1).await?).await?;

    let mut tg = TestGrouping {
        coord_circuits: coordinator_worker_circuits,
        coord_edge: coordinator_edge_node,
        coord_proc: coordinator_processor_node,
        realm_edge: realm_edge_node,
        realm_proc: realm_processor_node,
        coord_w_queue_store: q.clone(),
        realm_w_queue_store: realm_q.clone(),
        proof_verifier,
    };

    timer.lap("created coordinator nodes");

    let mut helper = ExampleDemoUserInfoStore::new();

    let contract_helper = tg.deploy_contract().await?;

    timer.lap("finished building wallet/zksig/helper circuits");

    tg.produce_block().await?;
    checkpoint_id += 1;
    timer.lap("deployed contract");

    let mut current_g_reg_id = 0;
    let priv_keys_batch_1 = (0..100).map(|x| QHashOut::rand()).collect::<Vec<_>>();
    let mut g_users = helper
        .register_users(&tg.coord_edge, current_g_reg_id, &priv_keys_batch_1)
        .await?;

    helper
        .register_users(&tg.coord_edge, current_g_reg_id, &priv_keys_batch_1)
        .await?;

    timer.lap("reg users");
    tg.produce_block().await?;
    checkpoint_id += 1;

    timer.lap("prod block");

    let latest_l2_block_state =
        QEDCoordinatorStoreReaderAsync::get_latest_l2_block_state(&store_reader).await?;

    //let stroots = st.get_checkpoint_global_state_roots(latest_l2_block_state.checkpoint_id).await?;
    //println!("[mainfnc] current_state_roots: {}",serde_json::to_string_pretty(&stroots).unwrap());

    timer.lap("start: init QEDUPSStepCircuitManager");

    let main_circuits =
        QCircuitManager::Local(QEDUPSStepCircuitManager::<C, D>::new_with_config(QED_NETWORK_MAGIC_REGTEST));
    //main_circuits.print_common_config();

    timer.lap("end: init QEDUPSStepCircuitManager");

    let lps: QEDLocalProvingSessionStore<
        GoldilocksField,
        Arc<KVQSimpleMemoryBackingStore>,
    > = QEDLocalProvingSessionStore::new_at(
        store_reader.clone(),
        GoldilocksField::from_noncanonical_u64(latest_l2_block_state.checkpoint_id),
        GoldilocksField::from_noncanonical_u64(0),
        GoldilocksField::ONE,
        UPS_SESSION_PROOF_TREE_HEIGHT as usize,
    );

    let mut circuit_info = SessionCircuitInfoStore::new();

    circuit_info.register_circuit(
        LocalCircuitType::SimpleZKSignature.into(),
        helper.wallet.circuit.get_fingerprint(),
        helper.wallet.circuit.get_verifier_config_ref().into(),
    );

    main_circuits.register_info(&mut circuit_info);
    contract_helper.register_funcs(0, &mut circuit_info);

    let mut mgr = UserProvingSessionManager::<GoldilocksField, QEDHasher, _, C, D>::new(
        lps,
        circuit_info,
        main_circuits.ups_circuit_whitelist_root()?,
    )?;

    timer.lap("setup mgr");

    timer.lap("started up");

    timer.lap("START USER PROVING SESSION");
    type F = GoldilocksField;
    let mut mgr = helper.run_txs_for_users_prep(
        mgr,
        &contract_helper,
        &main_circuits,
        0,
        vec![(
            g_users[0],
            vec![
                ("simple_mint_debug", vec![F::from_canonical_u64(1000)]),
                (
                    "simple_transfer",
                    vec![
                        F::from_canonical_u64(g_users[1]),
                        F::from_canonical_u64(100),
                    ],
                ),
            ],
        )],
    )?;

    timer.lap("proved group");

    helper.send_txs_to_edge(&tg.realm_edge).await?;

    timer.lap("sent all to edge");

    tg.produce_block().await?;
    checkpoint_id += 1;
    timer.lap("built block");
    timer.lap("finished jobs");

    Ok(())
}
pub async fn run(args: super::BenchmarkFullGroup3Args) -> anyhow::Result<()> {
    run_fred_test3().await
}
