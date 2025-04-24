use kvq::memory::arc_imm::KVQArcImmutableStoreWrapper;
use kvq_store_lmdbx::KVQlibmdbxStore;
use reth_libmdbx::{Environment, EnvironmentFlags, Mode, SyncMode, RW};
use std::path::PathBuf;

pub fn new_lmdbx_env(mode: Mode, db_path: &str) -> anyhow::Result<Environment> {
    let flags = EnvironmentFlags {
        no_sub_dir: false,
        mode,
        coalesce: true,
        ..Default::default()
    };

    let env = Environment::builder()
        .set_max_dbs(1)
        .set_flags(flags)
        .open(PathBuf::from(db_path).as_path())?;

    Ok(env)
}
