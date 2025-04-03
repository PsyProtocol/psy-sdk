use crate::package::Package;
use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct Workspace {
    pub root_dir: PathBuf,
    /// Optional target directory override.
    pub target_dir: PathBuf,
    pub package: Package,
}
