use fred::prelude::*;
use kvq::memory::simple::KVQSimpleMemoryBackingStore;
use std::sync::Arc;
use qed_common_circuit::circuits::{traits::qstandard::QStandardCircuit, zk_signature3::manager::SimpleQEDZKSignatureManager};
use psy_core::{config::network_constants::{QED_NETWORK_MAGIC_REGTEST, UPS_SESSION_PROOF_TREE_HEIGHT}, job::traits::{QProofStoreAsyncImm, QProofStoreReaderAsync}, ups::circuits::{LocalCircuitId, LocalCircuitType}, utils::debug_timer::DebugTimer}
;
use psy_crypto::{
    common::simple_circuit_library::SimpleCircuitLibrary, hash::traits::qhashable::QFieldHashable, signature::zk::{data::ZKPublicKeyInfo, wallet::SimpleQEDPrivateKey}
};
use psy_data::guta::api::{GUTARealmCheckpointResult, SubmitGUTARealmResultAPINoProofInput, SubmitUserEndCapProofAPIInput};
use qed_node::{
    coordinator::state::{
       edge::CoordinatorEdgeContext,
       processor::{CoordinatorConfig, CoordinatorProcessorContext},
    }, realm::state::{edge::RealmEdgeContext, processor::{RealmConfig, RealmProcessorContext}}, worker::{simple_async_coord::SimpleAsyncCoordinatorWorker, simple_async_realm::SimpleAsyncRealmWorker}
};
use qed_node::common::verifier::get_cached_generic_verifier;
use qed_prover::{local::provider::UPSCircuitManagerTrait, ups::{circuit_manager::core::{QCircuitManager, QEDUPSStepCircuitManager}, session::UserProvingSessionManager}};
use qed_rollup_circuit::coordinator::coordinator_helper::QEDCoordinatorCircuitManager;
use psy_data::{config::store_config::{QEDFelt, QEDHasher}, traits::qdatastore::qtreedata::QEDComboDataStoreReaderWriterSync};
use psy_store::{controllers::local::{proving_session::QEDLocalProvingSessionStore, session_info::SessionCircuitInfoStore}, node::coordinator::QEDCoordinatorStoreReaderAsync, queue::ProofStoreFred, queue::task_queue::{QProvingTaskStore, QProvingTaskStoreImpl}};
use super::super::test_helpers::{contract::gen_test_contract, ups::ExampleDemoUserInfoStore};
use psy_store::node::coordinator::QEDCoordinatorStoreWriterAsyncImm;
use std::time::Duration;


use plonky2::{
    field::{goldilocks_field::GoldilocksField, types::Field},
    plonk::
        config::PoseidonGoldilocksConfig
    ,
};
use psy_core::
    data::qhashout::QHashOut
;
use psy_store::store::journal::JournalStore;

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

    store_reader.initialize_store(None).await?;
    //let worker_count = 16usize;
    //let items_per_worker = 2000usize;

    let coord_config = CoordinatorConfig::get_standard();

    let qps = Arc::new(q.clone());

    let realm_qps = Arc::new(realm_q.clone());

    let st = store_reader.clone();

    timer.lap("initialized store");

    let task_store = Arc::new(
        QProvingTaskStoreImpl::new("redis://127.0.0.1/", 10, "biz_key")
            .await
            .expect("Failed to create JobTaskStore")
    );

    let proof_verifier = Arc::new(get_cached_generic_verifier::<C, D>());
    timer.lap("created proof verifier");

    use psy_core::config::network_constants::get_default_worker_public_key;
    let coordinator_worker_circuits =
        QEDCoordinatorCircuitManager::<C, D>::new_with_library(&proof_verifier.library, get_default_worker_public_key::<GoldilocksField>());

    timer.lap("built coordinator worker circuits");

    let coordinator_edge_node =
        CoordinatorEdgeContext::new(
            coord_config,
            st.clone(),
            qps.clone(),
            qps.clone(),
            Arc::clone(&proof_verifier),
        )
        .await?;

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

    let mut helper = ExampleDemoUserInfoStore::new();


    timer.lap("finished building wallet/zksig/helper circuits");
    let (contract_helper, contract_deploy_cmd) =gen_test_contract::<C,D>(QHashOut::rand())?;
    coordinator_edge_node.handle_deploy_contract(contract_deploy_cmd).await?;

    let mut current_g_reg_id = 0;
    let priv_keys_batch_1 = (0..100).map(|x| QHashOut::rand()).collect::<Vec<_>>();
    let mut g_users = helper.register_users(&coordinator_edge_node, current_g_reg_id, &priv_keys_batch_1).await?;

    timer.lap("deployed contract and registered users");

    coordinator_processor_node.build_block(0).await?;

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

    let realm_config = RealmConfig::get_standard(0);

    let realm_edge_node = RealmEdgeContext::new(
        realm_config,
        st.clone(),
        realm_qps.clone(),
        realm_qps.clone(),
        Arc::clone(&proof_verifier),
    ).await?;
    let mut realm_processor_node = RealmProcessorContext::new(
        realm_config,
        None,
        JournalStore::new(st.clone()),
        realm_qps.clone(),
        realm_qps.clone(),
        realm_qps.clone(),
        realm_qps.clone(),
        task_store.clone(),
        Arc::clone(&proof_verifier),
    ).await?;
    //realm_edge_node.handle_recv_checkpoint_sync(coordinator_processor_node.store.get_checkpoint_sync_info_compact(1).await?).await?;

    let sync1 = coordinator_processor_node.store.get_checkpoint_sync_info_compact(1).await?;
    realm_processor_node.handle_checkpoint_sync(sync1).await?;
    realm_processor_node.build_block(0).await?;
    let realm_worker_output_job_id = SimpleAsyncRealmWorker::run_worker_until_done::<
        _,
        _,
        SimpleCircuitLibrary<GoldilocksField>,
        QEDCoordinatorCircuitManager<C, D>,
        C,
        D,
    >(
        &realm_q.clone(),
        &realm_q.clone(),
        &coordinator_worker_circuits,
        &proof_verifier.library,
    ).await?;


    let realm_result: GUTARealmCheckpointResult<QEDFelt>  = { let bytes = realm_qps.get_bytes_by_id(realm_worker_output_job_id).await?; bincode::deserialize(&bytes) }.map_err(|e| anyhow::anyhow!("{:?}",e))?;
    let realm_proof = realm_qps.get_proof_by_id(realm_result.proof_id).await?;

    coordinator_edge_node.handle_recv_guta_from_realm(SubmitGUTARealmResultAPINoProofInput{
        realm_id: 0,
        checkpoint_id: realm_result.checkpoint_id,
        guta_stats: realm_result.guta_stats,
        top_line_proof: realm_result.top_line_proof,
        checkpoint_tree_root: realm_result.checkpoint_tree_root,
        proof_id: realm_result.proof_id,
    }, &realm_proof).await?;

    coordinator_processor_node.build_block(0).await?;
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

    let latest=store_reader.get_latest_l2_block_state().await?;
    let new_sync = store_reader.get_checkpoint_sync_info_compact(latest.checkpoint_id).await?;

    realm_processor_node.handle_checkpoint_sync(
        new_sync
    ).await?;









    let latest_l2_block_state = st.get_latest_l2_block_state().await?;

    //let stroots = st.get_checkpoint_global_state_roots(latest_l2_block_state.checkpoint_id).await?;
    //println!("[mainfnc] current_state_roots: {}",serde_json::to_string_pretty(&stroots).unwrap());


    timer.lap("start: init QEDUPSStepCircuitManager");

    let main_circuits = QCircuitManager::Local(QEDUPSStepCircuitManager::<C, D>::new_with_config(QED_NETWORK_MAGIC_REGTEST));
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
        UPS_SESSION_PROOF_TREE_HEIGHT as usize
    );

    let mut circuit_info = SessionCircuitInfoStore::new();

    circuit_info.register_circuit(
        LocalCircuitType::SimpleZKSignature.into(),
        helper.wallet.circuit.get_fingerprint(),
        helper.wallet.circuit.get_verifier_config_ref().into(),
    );

    main_circuits.register_info(&mut circuit_info);
    contract_helper.register_funcs(0, &mut circuit_info);

    let mut mgr = UserProvingSessionManager::<GoldilocksField,QEDHasher,_,C,D>::new(
        lps,
        circuit_info,
        main_circuits.ups_circuit_whitelist_root().await?,
    ).await?;

    timer.lap("setup mgr");

    timer.lap("started up");


    timer.lap("START USER PROVING SESSION");
    type F = GoldilocksField;
    let mut mgr = helper.run_txs_for_users_prep(mgr, &contract_helper, &main_circuits, 0, vec![
        (
            g_users[0],
            vec![
                ("simple_mint_debug", vec![F::from_canonical_u64(1000)]),
                ("simple_transfer", vec![F::from_canonical_u64(g_users[1]), F::from_canonical_u64(100)]),
            ],

        ),
    ]).await?;

    timer.lap("proved group");

    helper.send_txs_to_edge(&realm_edge_node).await?;
    timer.lap("sent all to edge");


    realm_processor_node.build_block(0).await?;
    timer.lap("built block");
    let realm_worker_output_job_id = SimpleAsyncRealmWorker::run_worker_until_done::<
        _,
        _,
        SimpleCircuitLibrary<GoldilocksField>,
        QEDCoordinatorCircuitManager<C, D>,
        C,
        D,
    >(
        &realm_q.clone(),
        &realm_q.clone(),
        &coordinator_worker_circuits,
        &proof_verifier.library,
    ).await?;


    let realm_result: GUTARealmCheckpointResult<QEDFelt>  = { let bytes = realm_qps.get_bytes_by_id(realm_worker_output_job_id).await?; bincode::deserialize(&bytes) }.map_err(|e| anyhow::anyhow!("{:?}",e))?;
    println!("rr: {:?}",realm_result);
    let realm_proof = realm_qps.get_proof_by_id(realm_result.proof_id.get_output_id()).await?;

    coordinator_edge_node.handle_recv_guta_from_realm(SubmitGUTARealmResultAPINoProofInput{
        realm_id: 0,
        checkpoint_id: realm_result.checkpoint_id,
        guta_stats: realm_result.guta_stats,
        top_line_proof: realm_result.top_line_proof,
        checkpoint_tree_root: realm_result.checkpoint_tree_root,
        proof_id: realm_result.proof_id,
    }, &realm_proof).await?;
    coordinator_processor_node.build_block(0).await?;
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


    let latest=store_reader.get_latest_l2_block_state().await?;
    let new_sync = store_reader.get_checkpoint_sync_info_compact(latest.checkpoint_id).await?;

    realm_processor_node.handle_checkpoint_sync(
        new_sync
    ).await?;

    timer.lap("finished jobs");




    realm_processor_node.build_block(0).await?;
    timer.lap("built block");
    let realm_worker_output_job_id = SimpleAsyncRealmWorker::run_worker_until_done::<
        _,
        _,
        SimpleCircuitLibrary<GoldilocksField>,
        QEDCoordinatorCircuitManager<C, D>,
        C,
        D,
    >(
        &realm_q.clone(),
        &realm_q.clone(),
        &coordinator_worker_circuits,
        &proof_verifier.library,
    ).await?;


    let realm_result: GUTARealmCheckpointResult<QEDFelt>  = { let bytes = realm_qps.get_bytes_by_id(realm_worker_output_job_id).await?; bincode::deserialize(&bytes) }.map_err(|e| anyhow::anyhow!("{:?}",e))?;
    println!("rr: {:?}",realm_result);
    let realm_proof = realm_qps.get_proof_by_id(realm_result.proof_id.get_output_id()).await?;

    coordinator_edge_node.handle_recv_guta_from_realm(SubmitGUTARealmResultAPINoProofInput{
        realm_id: 0,
        checkpoint_id: realm_result.checkpoint_id,
        guta_stats: realm_result.guta_stats,
        top_line_proof: realm_result.top_line_proof,
        checkpoint_tree_root: realm_result.checkpoint_tree_root,
        proof_id: realm_result.proof_id,
    }, &realm_proof).await?;
    coordinator_processor_node.build_block(0).await?;
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


    let latest=store_reader.get_latest_l2_block_state().await?;
    let new_sync = store_reader.get_checkpoint_sync_info_compact(latest.checkpoint_id).await?;

    realm_processor_node.handle_checkpoint_sync(
        new_sync
    ).await?;

    timer.lap("finished jobs");

    Ok(())
}
pub async fn run(args: super::BenchmarkFullGroup2Args) -> anyhow::Result<()> {
    run_fred_test3().await
}
