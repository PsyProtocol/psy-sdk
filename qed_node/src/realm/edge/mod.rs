pub mod handler;
pub mod error;
pub mod rpc;
mod sync;

use std::clone;
use super::handler::spawn_realm_job_update_task;
use super::rpc::RealmEdgeRpcServer;
use super::{config::RealmEdgeConfig, C, D};
use anyhow::Result;
use jsonrpsee::server::ServerBuilder;
use qed_store::store::QEDStore;
use crate::realm::state::edge::RealmEdgeContext;
use crate::realm::state::processor::RealmConfig;
use sync::spawn_active_checkpoint_sync_task;
use crate::common::verifier::get_cached_generic_verifier;
use std::sync::Arc;
use tracing::{debug, info};
use qed_store::queue::new_redis_async_pool;
use qed_store::queue::ProofStoreFred;
use qed_store::queue::ProofStoreRedisAsync;
use hyper::Method;
use tower_http::cors::{AllowHeaders, Any, CorsLayer};

pub async fn creat_redis_store(config: RealmEdgeConfig) -> Result<ProofStoreRedisAsync> {
    let pool = new_redis_async_pool(
        config.redis.redis_uri.as_str(),
        config.redis.pool_size.unwrap_or(10)
    ).await?;
    // Create storage and queues
    let proof_store = ProofStoreRedisAsync::new(
        pool,
        config.queue.worker_queue_suffix.clone(),
    ).await?;
    debug!("created proof store successfully!");
    Ok(proof_store)
}


/// Start Realm Edge node
pub async fn run_realm_edge(config: RealmEdgeConfig) -> Result<()> {
    info!(
        "Starting Realm Edge node with realm_id: {}",
        config.realm.realm_id
    );

    debug!("Realm Edge node config: {:?}", config);

    // Create storage and queues
    let proof_store = creat_redis_store(config.clone()).await?;
    // Create proof storage
    let proof_store = Arc::new(proof_store);
    let checkpoint_queue = proof_store.clone();

    // Create storage reader based on backend configuration
    let store = QEDStore::new(&config.backend.to_backend()).await?;
    let store_reader = Arc::new(store);

    debug!("created store reader successfully!");
    // Create proof verifier
    let proof_verifier = Arc::new(get_cached_generic_verifier::<C, D>());
    debug!("created proof verifier successfully!");

    // Create Realm configuration
    let realm_config = RealmConfig::get_standard(config.realm.node_id, config.realm.realm_id);
    debug!("created realm config successfully!");

    // Use the same ProofStoreRedisAsync for checkpoint sync
    let sync_queue = proof_store.clone();

    // Create Edge node context
    let edge_ctx = RealmEdgeContext::new(
        realm_config,
        store_reader.clone(),
        checkpoint_queue,
        proof_store.clone(),
        proof_verifier,
    )
    .await?;

    let cors_opts = CorsLayer::new()
        .allow_methods([
            Method::POST,
            Method::OPTIONS,
        ])
        .allow_origin(Any)
        .allow_headers(Any);
    let cors = tower::ServiceBuilder::new().layer(cors_opts);

    // Start RPC server
    let server_handle = ServerBuilder::default()
        .set_http_middleware(cors)
        .build(&config.rpc.listen_addr)
        .await?;

    let handle = server_handle.start(edge_ctx.into_rpc());
    info!("Realm Edge node started on {}", config.rpc.listen_addr.clone());

    let proof_store = creat_redis_store(config.clone()).await?;

    // Spawn task to send proof to coordinator
    spawn_realm_job_update_task(
        Arc::from(proof_store),
        realm_config.realm_id as u64,
        config.rpc.coordinator_addr.clone(),
    )
    .await?;
    spawn_active_checkpoint_sync_task(
        store_reader,
        sync_queue,
        config.rpc.coordinator_addr,
    ).await?;


    // Keep server running¶
    handle.stopped().await;
    Ok(())
}
