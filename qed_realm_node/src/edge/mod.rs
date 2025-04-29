pub mod context;
pub mod error;
pub mod request;
pub mod rpc;

use self::context::RealmEdgeContext;
use crate::context::spawn_realm_job_update_task;
use crate::rpc::RealmEdgeRpcServer;
use crate::{config::RealmEdgeConfig, C, D, REALM_PROCESSOR_SUFFIX};
use anyhow::Result;
use jsonrpsee::core::client::ClientT;
use jsonrpsee::rpc_params;
use jsonrpsee::server::ServerBuilder;
use kvq::memory::arc_imm::KVQArcImmutableStoreWrapper;
use kvq_store_lmdbx::KVQlibmdbxStore;
use qed_crypto::common::generic_circuit_verifier::GenericCircuitVerifier;
use qed_node::nimpl::new_fred_pool;
use qed_node::nimpl::proof_store_fred::ProofStoreFred;
use qed_node::realm::state::processor::RealmConfig;
use std::sync::Arc;
use tracing::{debug, info};

/// Start Realm Edge node
pub async fn run_realm_edge(config: RealmEdgeConfig) -> Result<()> {
    info!(
        "Starting Realm Edge node with realm_id: {}",
        config.realm.realm_id
    );

    debug!("Realm Edge node config: {:?}", config);

    // Create storage and queues

    let pool = new_fred_pool(
        &config.redis.redis_uri,
        config.redis.pool_size.unwrap_or(10),
    )
    .await
    .map_err(|e| anyhow::anyhow!("Failed to create Redis pool: {}", e))?;
    debug!("created redis pool successfully!");
    let proof_store = ProofStoreFred::new2(
        pool,
        config.queue.worker_queue_suffix,
        config.queue.notifications_queue_suffix,
        Some(REALM_PROCESSOR_SUFFIX),
        Some(REALM_PROCESSOR_SUFFIX),
    );
    debug!("created proof store successfully!");
    // Create proof storage
    let proof_store = Arc::new(proof_store);
    let checkpoint_queue = proof_store.clone();

    // Create store reader
    let store_reader = KVQArcImmutableStoreWrapper::<KVQlibmdbxStore>::new(
        KVQlibmdbxStore::new_read(&config.db.path)?,
    );

    debug!("created store reader successfully!");
    // Create proof verifier
    let proof_verifier = Arc::new(GenericCircuitVerifier::<C, D>::new());
    debug!("created proof verifier successfully!");

    // Create Realm configuration
    let realm_config = RealmConfig::get_standard(config.realm.node_id, config.realm.realm_id);
    debug!("created realm config successfully!");

    let coordinator_addr = config.rpc.coordinator_addr;
    let realm_id = config.realm.realm_id;
    let realm_register_addr = config.rpc.register_addr;
    // Create Edge node context
    let edge_ctx = RealmEdgeContext::new(
        realm_config,
        Arc::new(store_reader),
        checkpoint_queue,
        proof_store.clone(),
        proof_verifier,
        proof_store.clone(),
        coordinator_addr.clone(),
    )
    .await?;

    // Start RPC server
    let server_handle = ServerBuilder::default()
        .build(&config.rpc.listen_addr)
        .await?;

    let handle = server_handle.start(edge_ctx.into_rpc());

    // Spawn task to send proof to coordinator
    spawn_realm_job_update_task(
        proof_store,
        realm_config.realm_id as u64,
        coordinator_addr.clone(),
    )
    .await?;

    // Register Realm
    register_realm_edge(coordinator_addr, realm_id, realm_register_addr).await?;

    info!("Realm Edge node started on {}", config.rpc.listen_addr);

    // Keep server running
    handle.stopped().await;
    Ok(())
}

pub async fn register_realm_edge(
    coordinator_addr: String,
    realm_id: u32,
    realm_register_addr: String,
) -> anyhow::Result<()> {
    let client = jsonrpsee::http_client::HttpClientBuilder::default()
        .build(coordinator_addr)
        .map_err(|e| anyhow::anyhow!("Failed to create RPC client: {}", e))?;

    let params = rpc_params![realm_id.to_string(), realm_register_addr];

    // 发起RPC调用
    match client
        .request::<bool, _>("register_realm_rpc", params)
        .await
    {
        Ok(true) => {
            info!(
                "Successfully registered realm {} with coordinator",
                realm_id
            );
            Ok(())
        }
        Ok(false) => {
            anyhow::bail!("Coordinator rejected realm registration")
        }
        Err(e) => {
            anyhow::bail!("Failed to register realm with coordinator: {}", e)
        }
    }
}
