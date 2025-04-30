use std::fs::{self, File};
use std::io::Read;
use std::path::Path;
use std::process::exit;
use fs2::FileExt;
use anyhow::{Context, Result};
use tracing::info;
use kvq::memory::arc_imm::KVQArcImmutableStoreWrapper;
use kvq_store_lmdbx::KVQlibmdbxStore;
use qed_store::traits::qdatastore::qtreedata::QEDComboDataStoreReaderWriterSync;


const PROCESSOR_LOCK_MARKER: &[u8] = b"initialized";

pub fn acquire_processor_lock<P: AsRef<Path>>(path: P) -> Result<File> {
    let file = File::create(&path)?;
    file.try_lock_exclusive()?;
    Ok(file)
}
fn acquire_processor_lock_and_check_init(lock_path: &Path) -> Result<(File, bool)> {
    // ensure the lock path exists
    let mut file = File::options()
        .create(true)
        .write(true)
        .read(true)
        .open(lock_path)
        .with_context(|| format!("Failed to create/open lock file: {:?}", lock_path))?;

    // try to acquire the lock
    if let Err(e) = file.try_lock_exclusive() {
        tracing::error!(
            "❌ Failed to acquire exclusive processor lock. Another Coordinator Processor instance may be running.\nLock file: {:?}\nError: {}",
            lock_path,
            e
        );
        exit(1);
    }
    // read the first 16 bytes to check if initialized
    let mut buf = [0u8; 16];
    let read_bytes = file.read(&mut buf)?;
    let already_initialized = buf[..read_bytes].eq(PROCESSOR_LOCK_MARKER);

    Ok((file, !already_initialized))
}
/// Checks and locks the processor startup lock file (located under the DB path).
pub fn prepare_processor_lock_and_init_if_needed<P: AsRef<Path>>(
    db_path: P,
    store: &KVQArcImmutableStoreWrapper<KVQlibmdbxStore>,
) -> anyhow::Result<(File, bool)> {
    let db_path = db_path.as_ref();
    let lock_path = db_path.join("processor.lock");
    info!("Acquiring processor lock at: {:?}", lock_path);
    // ensure the DB path exists
    fs::create_dir_all(&db_path)?;

    //try to acquire the lock and check if initialization is needed
    let (lock_file, need_init) = acquire_processor_lock_and_check_init(&lock_path)?;

    if need_init {
        tracing::info!("🛠️ First-time processor launch detected — initializing DB store...");
        store.initialize_store()?;
        fs::write(&lock_path, PROCESSOR_LOCK_MARKER)?;
        tracing::info!("✅ Store initialized and lock marked.");
    } else {
        tracing::info!("✅ Processor lock acquired — DB already initialized.");
    }

    Ok((lock_file, need_init))
}