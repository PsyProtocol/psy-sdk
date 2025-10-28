pub mod error;
pub mod handler;
pub mod rpc;
pub mod sync;
pub(crate) use sync::spawn_active_checkpoint_sync_task;
pub mod edge_v2;
use std::sync::Arc;

use anyhow::Result;
pub use edge_v2::run_realm_edge_v2;
use hyper::Method;
use jsonrpsee::server::ServerBuilder;
use psy_store::{
    queue::{new_redis_async_pool, task_queue::QProvingTaskStoreImpl, ProofStoreRedis},
    store::PsyStore,
};
use tower_http::cors::{Any, CorsLayer};
use tracing::{debug, info};

use super::{config::RealmEdgeConfig, rpc::RealmEdgeRpcServer, C, D, F};
use crate::{
    common::{health::HealthLayer, jobs::JobSchedulerRpcServer, verifier::get_cached_generic_verifier, whitelist::WhiteListCache},
    realm::{
        edge::sync::spawn_realm_job_update_task,
        handler::RealmEdgeHandler,
        state::{edge::RealmEdgeContext, edge_queue_helper::RealmEdgeQueueHelper, processor::RealmConfig, queue_factory::QueueFactory},
    },
    watcher::watcher_client::WatcherClient,
};

const WATCHER_NODE_ID_PREFIX: &str = "realm_edge_node_";
pub async fn creat_redis_store(config: RealmEdgeConfig) -> Result<ProofStoreRedis> {
    // Create storage and queues
    let proof_store = ProofStoreRedis::new(&config.redis.redis_uri, config.queue.queue_biz_key.clone()).await?;
    debug!("created proof store successfully!");
    Ok(proof_store)
}

/// Start Realm Edge node
pub async fn run_realm_edge(config: RealmEdgeConfig) -> Result<()> {
    info!("Starting Realm Edge node with realm_id: {}", config.realm.realm_id);

    debug!("Realm Edge node config: {:?}", config);

    // Create storage and queues
    let proof_store = creat_redis_store(config.clone()).await?;
    // Create task queue for jobs
    let task_store = QProvingTaskStoreImpl::new(
        config.redis.redis_uri.as_str(),
        config.redis.pool_size.unwrap_or(20),
        &config.queue.queue_biz_key,
    )
    .await?;

    // Create proof storage
    let proof_store = Arc::new(proof_store);
    let checkpoint_queue = proof_store.clone();

    // Create storage reader based on backend configuration
    let store = PsyStore::new(&config.backend.to_backend()).await?;
    let store_reader = Arc::new(store);

    let queue_helper = QueueFactory::create_rsmq_helper::<F>(
        &config.redis.redis_uri,
        config.redis.pool_size.unwrap_or(10),
        config.realm.realm_id,
        store_reader.clone(),
    )
    .await?;

    debug!("created store reader successfully!");
    // Create proof verifier
    let proof_verifier = Arc::new(get_cached_generic_verifier::<C, D>());
    debug!("created proof verifier successfully!");

    // Create Realm configuration
    let realm_config = RealmConfig::get_standard(config.realm.realm_id);
    debug!("created realm config successfully!");

    // Use the same ProofStoreRedis for checkpoint sync
    let sync_queue = proof_store.clone();

    // Create Edge node context
    let edge_ctx = RealmEdgeContext::new(realm_config, store_reader.clone(), checkpoint_queue, proof_store.clone(), proof_verifier).await?;

    let cors_opts = CorsLayer::new()
        .allow_methods([Method::POST, Method::OPTIONS])
        .allow_origin(Any)
        .allow_headers(Any);
    let cors = tower::ServiceBuilder::new().layer(HealthLayer).layer(cors_opts);

    // Start RPC server
    let server_handle = ServerBuilder::default().set_http_middleware(cors).build(&config.rpc.listen_addr).await?;
    let job_notify_queue = proof_store.clone();

    let whitelist_cache = WhiteListCache::new(&config.config_path)?;

    // Initialize watcher
    info!("📡 Initializing watcher client...");
    let watcher = WatcherClient::new(
        &config.redis.redis_uri,
        config.redis.pool_size.unwrap_or(20),
        &config.queue.queue_biz_key,
        Some(&format!("{}{}", WATCHER_NODE_ID_PREFIX, config.realm.realm_id)),
    )
    .await?;
    let watcher_client = Arc::new(watcher);
    info!("✅ Watcher client initialized successfully");

    let handler = RealmEdgeHandler::new(
        edge_ctx.clone(),
        job_notify_queue,
        Arc::new(task_store),
        whitelist_cache,
        watcher_client,
        &config.rpc.coordinator_addr,
        Arc::new(queue_helper),
    )?;

    let mut rpc_module = RealmEdgeRpcServer::into_rpc(handler.clone());
    let job_rpc_module = JobSchedulerRpcServer::into_rpc(handler);
    rpc_module.merge(job_rpc_module)?;
    let handle = server_handle.start(rpc_module);

    info!("Realm Edge node started on {}", config.rpc.listen_addr.clone());

    // Spawn task to send proof to coordinator
    // spawn_realm_job_update_task(
    //     Arc::from(proof_store),
    //     realm_config.realm_id as u64,
    //     config.rpc.coordinator_addr.clone(),
    //     Arc::new(edge_ctx),
    //     None,
    // )
    // .await?;
    // spawn_active_checkpoint_sync_task(realm_config.realm_id, store_reader,
    // sync_queue, config.rpc.coordinator_addr)     .await?;

    // Keep server running¶
    handle.stopped().await;
    Ok(())
}
