use crate::package::Package;
use std::path::PathBuf;

#[derive(Clone)]
pub struct Workspace {
    pub root_dir: PathBuf,
    /// Optional target directory override.
    pub target_dir: Option<PathBuf>,
    pub package: Package,
}
