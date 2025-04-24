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
use reth_libmdbx::{Environment, EnvironmentFlags, Mode, RO};
use std::clone::Clone;
use std::path::Path;
use std::sync::Arc;
use tracing::info;

/// Start Realm Edge node
pub async fn start_realm_edge_node(config: RealmEdgeConfig) -> Result<()> {
    info!(
        "Starting Realm Edge node with realm_id: {}",
        config.realm.realm_id
    );

    // Create storage and queues
    let pool = new_fred_pool(&config.redis.url, config.redis.pool_size.unwrap_or(10)).await?;
    //todo! maybe it shoule use new2
    let proof_store = ProofStoreFred::new(
        pool,
        config.queue.worker_queue_suffix,
        config.queue.notifications_queue_suffix,
    );
    // Create proof storage
    let proof_store = Arc::new(proof_store);
    let checkpoint_queue = proof_store.clone();

    let env = Environment::builder()
        .set_max_dbs(10)
        .set_flags(EnvironmentFlags {
            no_sub_dir: false,
            mode: Mode::ReadOnly,
            coalesce: true,
            ..Default::default()
        })
        .open(Path::new(config.db.path.as_str()))?;

    let txn = env.begin_ro_txn()?;
    let store_reader: KVQArcImmutableStoreWrapper<KVQlibmdbxStore<RO>> =
        KVQArcImmutableStoreWrapper::<KVQlibmdbxStore<RO>>::new(KVQlibmdbxStore::new(
            txn.clone(),
            None,
        )?);
    let cmd_store = store_reader.dup();

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
    let edge_ctx = Arc::new(edge_ctx);

    // Start RPC server
    let server_handle =
        start_realm_edge_rpc_server(cmd_store, edge_ctx, &config.rpc.listen_addr).await?;

    info!("Realm Edge node started on {}", config.rpc.listen_addr);

    // Keep server running
    server_handle.stopped().await;

    Ok(())
}
