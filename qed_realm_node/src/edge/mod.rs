mod context;
mod contract_reader;
mod error;
mod realm_config;
mod rpc;

use anyhow::Result;
use plonky2::plonk::config::PoseidonGoldilocksConfig;
use qed_crypto::common::generic_circuit_verifier::GenericCircuitVerifier;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::info;

use self::{
    context::RealmEdgeContext, realm_config::RealmConfig, rpc::start_realm_edge_rpc_server,
};

use crate::{config::RealmNodeConfig, new_proof_store, new_store_reader, new_with_connection};

// Type aliases for convenience
type C = PoseidonGoldilocksConfig;
const D: usize = 2;

/// Start Realm Edge node
pub async fn start_realm_edge_node(config: RealmNodeConfig) -> Result<()> {
    info!(
        "Starting Realm Edge node with realm_id: {}",
        config.realm.realm_id
    );

    // Create storage and queues
    let pool = new_with_connection(&config.redis.url, config.redis.pool_size.unwrap_or(10)).await?;
    let proof_store = new_proof_store(
        pool,
        config.queue.worker_queue_suffix,
        config.queue.notifications_queue_suffix,
    )
    .await?;

    // Create proof storage
    let proof_store = Arc::new(proof_store);
    let checkpoint_queue = proof_store.clone();

    // Create store reader
    let store_reader = new_store_reader(&config.db.db_path).await?;

    // Create proof verifier
    let proof_verifier = Arc::new(GenericCircuitVerifier::<C, D>::new());

    // Create Realm configuration
    let realm_config = RealmConfig::get_standard(config.realm.node_id, config.realm.realm_id);

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
    let edge_ctx = Arc::new(Mutex::new(edge_ctx));

    // Start RPC server
    let server_handle = start_realm_edge_rpc_server(edge_ctx, &config.rpc.listen_addr).await?;

    info!("Realm Edge node started on {}", config.rpc.listen_addr);

    // Keep server running
    server_handle.stopped().await;

    Ok(())
}
