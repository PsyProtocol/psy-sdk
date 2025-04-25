pub mod context;
pub mod error;
pub mod request;
pub mod rpc;

use self::{context::RealmEdgeContext, rpc::start_realm_edge_rpc_server};
use crate::{config::RealmEdgeConfig, C, D};
use anyhow::Result;
use kvq::memory::arc_imm::KVQArcImmutableStoreWrapper;
use kvq_store_lmdbx::KVQlibmdbxStore;
use qed_crypto::common::generic_circuit_verifier::GenericCircuitVerifier;
use qed_node::nimpl::new_fred_pool;
use qed_node::nimpl::proof_store_fred::ProofStoreFred;
use qed_node::realm::state::processor::RealmConfig;
use qed_node_common::store::new_lmdbx_env;
use reth_libmdbx::{Mode, RO};
use std::clone::Clone;
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
    let proof_store = ProofStoreFred::new(
        pool,
        config.queue.worker_queue_suffix,
        config.queue.notifications_queue_suffix,
    );
    debug!("created proof store successfully!");
    // Create proof storage
    let proof_store = Arc::new(proof_store);
    let checkpoint_queue = proof_store.clone();

    // Create store reader
    let store_reader = KVQArcImmutableStoreWrapper::<KVQlibmdbxStore>::new(
        KVQlibmdbxStore::new_read(&config.db.path)?,
    );

    let cmd_store = store_reader.dup();
    debug!("created store reader successfully!");
    // Create proof verifier
    let proof_verifier = Arc::new(GenericCircuitVerifier::<C, D>::new());
    debug!("created proof verifier successfully!");

    // Create Realm configuration
    let realm_config = RealmConfig::get_standard(config.realm.node_id, config.realm.realm_id);
    debug!("created realm config successfully!");
    // Create Edge node context
    let edge_ctx = RealmEdgeContext::new(
        realm_config,
        Arc::new(store_reader),
        checkpoint_queue,
        proof_store,
        proof_verifier,
    )
    .await?;

    // Wrap in thread-safe context
    let edge_ctx = Arc::new(edge_ctx);

    // Start RPC server
    let server_handle =
        start_realm_edge_rpc_server(cmd_store, edge_ctx, &config.rpc.listen_addr).await?;

    info!("Realm Edge node started on {}", config.rpc.listen_addr);

    // Keep server running
    server_handle.stopped().await;
    Ok(())
}
