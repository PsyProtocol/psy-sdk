mod processor;

pub use processor::*;

pub type C = plonky2::plonk::config::PoseidonGoldilocksConfig;
pub const D: usize = 2;
pub type F = qed_store::config::store_config::QEDFelt;

pub type H = QEDHasher;

pub mod config;
pub mod edge;

use kvq::memory::arc_imm::KVQArcImmutableStoreWrapper;
use kvq_store_lmdbx::KVQlibmdbxStore;
use qed_store::config::store_config::QEDHasher;
use reth_libmdbx::{Environment, EnvironmentFlags, Mode, SyncMode, RW};
use std::path::PathBuf;

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

    Ok(store_reader)
}
