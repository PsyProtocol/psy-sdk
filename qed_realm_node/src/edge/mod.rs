pub mod context;
pub mod error;
pub mod request;
pub mod rpc;
mod sync;

use std::clone;
use self::context::RealmEdgeContext;
use crate::context::spawn_realm_job_update_task;
use crate::rpc::RealmEdgeRpcServer;
use crate::Queue;
use crate::{config::RealmEdgeConfig, C, D};
use anyhow::Result;
use jsonrpsee::server::ServerBuilder;
use kvq::memory::arc_imm::KVQArcImmutableStoreWrapper;
use kvq_store_lmdbx::KVQlibmdbxStore;
use qed_node::realm::state::processor::RealmConfig;
use sync::spawn_active_checkpoint_sync_task;
use qed_node_common::verifier::get_cached_generic_verifier;
use std::sync::Arc;
use tracing::{debug, info};
use qed_node::nimpl::{new_fred_pool, new_redis_async_pool};
use qed_node::nimpl::proof_store_fred::ProofStoreFred;
use qed_node::nimpl::proof_store_redis_async::ProofStoreRedisAsync;
use hyper::Method;
use tower_http::cors::{AllowHeaders, Any, CorsLayer};

pub async fn creat_fred_store(config: RealmEdgeConfig) -> Result<ProofStoreFred> {
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
        &config.queue.worker_queue_suffix,
        &config.queue.notifications_queue_suffix,
        &config.queue.proof_store_key_suffix.as_str(),
        &config.queue.proof_store_key_suffix.as_str(),
    );
    debug!("created proof store successfully!");
    Ok(proof_store)
}

pub async fn creat_redis_store(config: RealmEdgeConfig) -> Result<ProofStoreRedisAsync> {
    let pool = new_redis_async_pool(
        config.redis.redis_uri.as_str(),
        config.redis.pool_size.unwrap_or(10)
    ).await?;
    // Create storage and queues
    let proof_store = ProofStoreRedisAsync::new2(
        pool,
        &config.queue.worker_queue_suffix,
        &config.queue.notifications_queue_suffix,
        &config.queue.proof_store_key_suffix.as_str(),
        &config.queue.proof_store_key_suffix.as_str(),
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
    let proof_store = creat_fred_store(config.clone()).await?;
    // Create proof storage
    let proof_store = Arc::new(proof_store);
    let checkpoint_queue = proof_store.clone();

    // Create store reader
    let store_reader = KVQArcImmutableStoreWrapper::<KVQlibmdbxStore>::new(
        KVQlibmdbxStore::new_read(&config.db.path)?,
    );

    let store_reader = Arc::new(store_reader);

    debug!("created store reader successfully!");
    // Create proof verifier
    let proof_verifier = Arc::new(get_cached_generic_verifier::<C, D>());
    debug!("created proof verifier successfully!");

    // Create Realm configuration
    let realm_config = RealmConfig::get_standard(config.realm.node_id, config.realm.realm_id);
    debug!("created realm config successfully!");

    let queue = Queue::new(config.redis.redis_uri.as_str(), config.redis.pool_size.unwrap_or(10), config.queue.worker_queue_suffix.clone()).await?;

    let queue = Arc::new(queue);

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
        queue,
        config.rpc.coordinator_addr,
    ).await?;


    // Keep server running¶
    handle.stopped().await;
    Ok(())
}
