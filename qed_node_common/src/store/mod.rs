use kvq::memory::arc_imm::KVQArcImmutableStoreWrapper;
use kvq_store_lmdbx::KVQlibmdbxStore;
use reth_libmdbx::{Environment, EnvironmentFlags, Mode, SyncMode, RW};
use std::path::PathBuf;

pub fn new_lmdbx_store(
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
    let store: KVQArcImmutableStoreWrapper<KVQlibmdbxStore<RW>> =
        KVQArcImmutableStoreWrapper::<KVQlibmdbxStore<RW>>::new(KVQlibmdbxStore::new(
            txn.clone(),
            None,
        )?);

    Ok(store)
}
