mod processor;

pub use processor::*;

pub type C = plonky2::plonk::config::PoseidonGoldilocksConfig;
pub const D: usize = 2;
pub type F = qed_store::config::store_config::QEDFelt;

pub type H = QEDHasher;

pub mod config;
pub mod edge;

use fred::prelude::*;
use kvq::memory::arc_imm::KVQArcImmutableStoreWrapper;
use kvq_store_lmdbx::KVQlibmdbxStore;
use qed_node::nimpl::proof_store_fred::ProofStoreFred;
use qed_store::config::store_config::QEDHasher;
use qed_store::traits::qdatastore::qtreedata::QEDComboDataStoreReaderWriterSync;
use reth_libmdbx::{Environment, EnvironmentFlags, Mode, SyncMode, RW};
use std::path::PathBuf;
use std::time::Duration;

/// Create a new Redis connection pool and checkpoint queue
///
/// # Arguments
///
/// * `redis_url` - Redis URL to connect to
/// * `pool_size` - Number of connections in the pool
pub async fn new_with_connection(redis_url: &str, pool_size: usize) -> anyhow::Result<Pool> {
    let config = Config::from_url(redis_url)?;
    let pool = Builder::from_config(config)
        .with_connection_config(|config| {
            config.connection_timeout = Duration::from_secs(10);
        })
        // Use exponential backoff, starting at 100 ms and doubling on each failed attempt up to 30 sec
        .set_policy(ReconnectPolicy::new_exponential(0, 100, 30_000, 2))
        .build_pool(pool_size)?;

    pool.init().await?;
    Ok(pool)
}

pub async fn new_proof_store(
    pool: Pool,
    worker_queue_suffix: String,
    notifications_queue_suffix: String,
) -> anyhow::Result<ProofStoreFred> {
    let proof_store = ProofStoreFred::new(pool, worker_queue_suffix, notifications_queue_suffix);
    Ok(proof_store)
}

pub async fn new_store_reader(
    db_path: &str,
) -> anyhow::Result<KVQArcImmutableStoreWrapper<KVQlibmdbxStore<RW>>> {
    let flags = EnvironmentFlags {
        no_sub_dir: false,
        mode: Mode::ReadWrite {
            sync_mode: SyncMode::Durable,
        },
        coalesce: true,
        ..Default::default()
    };

    let env = Environment::builder()
        .set_max_dbs(10)
        .set_flags(flags)
        .open(PathBuf::from(db_path).as_path())?;

    let txn = env.begin_rw_txn()?;
    let store_reader: KVQArcImmutableStoreWrapper<KVQlibmdbxStore<RW>> =
        KVQArcImmutableStoreWrapper::<KVQlibmdbxStore<RW>>::new(KVQlibmdbxStore::new(
            txn.clone(),
            None,
        )?);

    store_reader.initialize_store()?;

    Ok(store_reader)
}
