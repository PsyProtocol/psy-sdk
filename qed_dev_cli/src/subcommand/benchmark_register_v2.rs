use fred::prelude::*;
use kvq::memory::simple::KVQSimpleMemoryBackingStore;
use std::sync::Arc;
use qed_common_circuit::circuits::{traits::qstandard::QStandardCircuit, zk_signature3::manager::SimpleQEDZKSignatureManager};
use qed_core::{config::network_constants::{QED_NETWORK_MAGIC_REGTEST, UPS_SESSION_PROOF_TREE_HEIGHT}, job::traits::{QProofStoreAsyncImm, QProofStoreReaderAsync}, ups::circuits::{LocalCircuitId, LocalCircuitType}, utils::debug_timer::DebugTimer}
;
use qed_crypto::{
    common::simple_circuit_library::SimpleCircuitLibrary, hash::traits::qhashable::QFieldHashable, signature::zk::{data::ZKPublicKeyInfo, wallet::SimpleQEDPrivateKey}
};
use qed_data::guta::api::{GUTARealmCheckpointResult, SubmitGUTARealmResultAPINoProofInput, SubmitUserEndCapProofAPIInput};
use qed_node::{
    coordinator::{
        state::{
            edge::CoordinatorEdgeContext,
            processor::{CoordinatorConfig, CoordinatorProcessorContext},
        },
    }, realm::state::{edge::RealmEdgeContext, processor::{RealmConfig, RealmProcessorContext}}, worker::{simple_async_coord::SimpleAsyncCoordinatorWorker, simple_async_realm::SimpleAsyncRealmWorker}
};
use qed_node::common::verifier::get_cached_generic_verifier;
use qed_prover::{local::provider::ProveProxyRpcTrait, ups::{circuit_manager::core::{QCircuitManager, QEDUPSStepCircuitManager}, session::UserProvingSessionManager}};
use qed_rollup_circuit::coordinator::coordinator_helper::QEDCoordinatorCircuitManager;
use qed_data::{config::store_config::{QEDFelt, QEDHasher}, traits::qdatastore::qtreedata::QEDComboDataStoreReaderWriterSync};
use qed_store::{controllers::local::{proving_session::QEDLocalProvingSessionStore, session_info::SessionCircuitInfoStore}, node::coordinator::{QEDCoordinatorStoreReaderAsync, QEDCoordinatorStoreWriterAsyncImm}, queue::ProofStoreFred, queue::task_queue::{JobTaskStore, JobTaskStoreImpl}};
use super::super::test_helpers::contract::gen_test_contract;
use std::time::Duration;


use plonky2::{
    field::{goldilocks_field::GoldilocksField, types::Field},
    plonk::
        config::PoseidonGoldilocksConfig
    ,
};
use qed_core::
    data::qhashout::QHashOut
;


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

    store_reader.initialize_store().await?;
    //let worker_count = 16usize;
    //let items_per_worker = 2000usize;

    let coord_config = CoordinatorConfig::get_standard(0);

    let qps = Arc::new(q.clone());

    let realm_qps = Arc::new(realm_q.clone());

    let st = Arc::new(store_reader.clone());

    timer.lap("initialized store");

    let job_task_store = Arc::new(
        JobTaskStoreImpl::new("redis://127.0.0.1/", 10)
            .await
            .expect("Failed to create JobTaskStore")
    );

    let proof_verifier = Arc::new(get_cached_generic_verifier::<C, D>());
    timer.lap("created proof verifier");

    use qed_core::config::network_constants::DEFAULT_WORKER_PUBLIC_KEY;
    let coordinator_worker_circuits =
        QEDCoordinatorCircuitManager::<C, D>::new_with_library(&proof_verifier.library, DEFAULT_WORKER_PUBLIC_KEY);

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
        job_task_store.clone(),
        Arc::clone(&proof_verifier),
    )
    .await?;
    timer.lap("created coordinator nodes");

    let priv_key_0 = QHashOut::rand();
    let priv_key_1 = QHashOut::rand();
    let mut wallet = SimpleQEDZKSignatureManager::<C,D>::new();
    let pub_key_0 = wallet.add_private_key_get_info(SimpleQEDPrivateKey::new(priv_key_0));
    let pub_alt_0 =SimpleQEDPrivateKey::new(priv_key_0).get_public_key_for_fingerprint::<QEDHasher>(wallet.circuit.get_fingerprint());

    println!("pub_key_0 {:?}, ({:?})",pub_key_0,pub_key_0.to_hash::<QEDHasher>());
    println!("pub_alt_0 {:?}",pub_alt_0);


    let pub_key_1 = wallet.add_private_key_get_info(SimpleQEDPrivateKey::new(priv_key_1));
    timer.lap("finished building wallet/zksig circuits");
    let (contract_helper, contract_deploy_cmd) =gen_test_contract::<C,D>(pub_key_1.qfhash::<QEDHasher>())?;
    coordinator_edge_node.handle_deploy_contract(contract_deploy_cmd).await?;

    coordinator_edge_node
        .handle_process_regsiter_user(pub_key_0)
        .await?;
    coordinator_edge_node
        .handle_process_regsiter_user(pub_key_1)
        .await?;
    timer.lap("sent requests");

    coordinator_processor_node.build_block().await?;

    let realm_config = RealmConfig::get_standard(0, 0);

    let realm_edge_node = RealmEdgeContext::new(
        realm_config,
        st.clone(),
        realm_qps.clone(),
        realm_qps.clone(),
        Arc::clone(&proof_verifier),
    ).await?;
    let mut realm_processor_node = RealmProcessorContext::new(
        realm_config,
        st.clone(),
        realm_qps.clone(),
        realm_qps.clone(),
        realm_qps.clone(),
        realm_qps.clone(),
        job_task_store.clone(),
        Arc::clone(&proof_verifier),
    ).await?;
    //realm_edge_node.handle_recv_checkpoint_sync(coordinator_processor_node.store.get_checkpoint_sync_info_compact(1).await?).await?;

    let sync1 = coordinator_processor_node.store.get_checkpoint_sync_info_compact(1).await?;
    realm_processor_node.handle_checkpoint_sync(sync1).await?;
    realm_processor_node.build_block().await?;
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
        circuit_type:realm_result.proof_id.circuit_type,
    }, &realm_proof).await?;

    coordinator_processor_node.build_block().await?;

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

    let user_0_pub_key = st.get_user_registration_tree_leaf_hash(latest_l2_block_state.checkpoint_id,0).await?;
    let priv_key_user_0 = if pub_key_0.qfhash::<QEDHasher>() == user_0_pub_key {
        priv_key_0
    }else if pub_key_1.qfhash::<QEDHasher>() == user_0_pub_key {
        priv_key_1
    }else{
        anyhow::bail!("missing private key!");
    };


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
        wallet.circuit.get_fingerprint(),
        wallet.circuit.get_verifier_config_ref().into(),
    );

    main_circuits.register_info(&mut circuit_info);
    contract_helper.register_funcs(0, &mut circuit_info);

    let mut mgr = UserProvingSessionManager::<GoldilocksField,QEDHasher,_,C,D>::new(
        lps,
        circuit_info,
        main_circuits.ups_circuit_whitelist_root()?,
    )?;

    timer.lap("setup mgr");

    timer.lap("started up");


    timer.lap("START USER PROVING SESSION");

    mgr.prove_ups_start(&main_circuits)?;
    timer.lap("proved ups_start");

    contract_helper.prove_func(
        &main_circuits,
        &mut mgr,
        0,
        "simple_mint_debug",
        vec![
            GoldilocksField::from_noncanonical_u64(1000),
        ]
    )?;
    timer.lap("proved token.simple_mint_debug(amount: 1000)");


    contract_helper.prove_func(
        &main_circuits,
        &mut mgr,
        0,
        "simple_transfer",
        vec![
            GoldilocksField::from_noncanonical_u64(1),
            GoldilocksField::from_noncanonical_u64(100),
        ]
    )?;
    timer.lap("proved token.simple_transfer(recipient: 2, amount: 100)");

    let new_nonce = GoldilocksField::from_noncanonical_u64(1);
    let sighash = mgr.get_sighash(QED_NETWORK_MAGIC_REGTEST, new_nonce);

    let signature_proof = wallet.zk_sign_for_private_key_value(priv_key_user_0, sighash)?;
    timer.lap("generated zk signature for UPS transaction batch");
    mgr.proof_tree_state.finalize_tree(&main_circuits)?;
    timer.lap("aggregated all UPS proofs into a single proof");
    let public_key_param =SimpleQEDPrivateKey::new(priv_key_user_0).get_public_key_param::<QEDHasher>();
    let end_cap_proof = mgr.prove_end_cap(
        &main_circuits,
         QED_NETWORK_MAGIC_REGTEST,
         new_nonce,
         wallet.circuit.get_fingerprint(),
         public_key_param,
        signature_proof,
         wallet.circuit.get_verifier_config_ref().to_owned()
    )?;
    timer.lap("Proved End Cap for UPS Session 🎉");

    // the end cap proof the proof that we send off to the network 🎉

    //main_circuits.ups_end_cap.circuit_data.verify(end_cap_proof)?;
    timer.lap("✅ Verified End Cap Proof");

/*
    let user_a_api_input = SubmitUserEndCapProofAPIInput{
        input: mgr.get_api_input()?,
        proof: end_cap_proof,
    };*/

    realm_edge_node.handle_recv_end_cap_from_user(
        mgr.get_api_input()?,
        &end_cap_proof
    ).await?;

    realm_processor_node.build_block().await?;
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
    let realm_proof = realm_qps.get_proof_by_id(realm_result.proof_id.get_output_id()).await?;

    coordinator_edge_node.handle_recv_guta_from_realm(SubmitGUTARealmResultAPINoProofInput{
        realm_id: 0,
        checkpoint_id: realm_result.checkpoint_id,
        guta_stats: realm_result.guta_stats,
        top_line_proof: realm_result.top_line_proof,
        checkpoint_tree_root: realm_result.checkpoint_tree_root,
        circuit_type:realm_result.proof_id.circuit_type,
    }, &realm_proof).await?;
    coordinator_processor_node.build_block().await?;


    let latest=store_reader.get_latest_l2_block_state().await?;
    let new_sync = store_reader.get_checkpoint_sync_info_compact(latest.checkpoint_id).await?;

    realm_processor_node.handle_checkpoint_sync(
        new_sync
    ).await?;

    timer.lap("finished jobs");

    Ok(())
}
pub async fn run(args: super::BenchmarkRegisterV2Args) -> anyhow::Result<()> {
    run_fred_test3().await
}
