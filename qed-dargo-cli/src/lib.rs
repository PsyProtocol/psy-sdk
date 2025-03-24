mod compile_cmd;
mod errors;

use crate::errors::CliError;
use qed_dargo::package::Workspace;
use qed_dargo_toml::files::{find_file_manifest_root, get_package_manifest};
use qed_dargo_toml::resolve_workspace_from_toml;
use clap::Args;
use std::path::PathBuf;

#[derive(Args, Clone, Debug)]
pub(crate) struct DargoConfig {
    // REMINDER: Also change this flag in the LSP test lens if renamed
    #[arg(long, hide = true, global = true, default_value = "./", value_parser = parse_path)]
    program_dir: PathBuf,

    /// Override the default target directory.
    #[arg(long, hide = true, global = true, value_parser = parse_path)]
    target_dir: Option<PathBuf>,
}

/// Parses a path and turns it into an absolute one by joining to the current directory.
fn parse_path(path: &str) -> Result<PathBuf, String> {
    use qed_dargo_toml::fm::NormalizePath;
    let mut path: PathBuf = path
        .parse()
        .map_err(|e| format!("failed to parse path: {e}"))?;
    if !path.is_absolute() {
        path = std::env::current_dir().unwrap().join(path).normalize();
    }
    Ok(path)
}

fn with_workspace<C, R>(cmd: C, config: DargoConfig, run: R) -> Result<(), CliError>
where
    R: FnOnce(C, Workspace) -> Result<(), CliError>,
{
    // All commands need to run on the workspace level, because that's where the `target` directory is.
    let package_dir = find_file_manifest_root(&config.program_dir)?;
    let toml_path = get_package_manifest(&package_dir)?;
    // Resolve the workspace from the toml file. It will download dependencies as well.
    let mut workspace = resolve_workspace_from_toml(&toml_path)?;
    workspace.target_dir = config.target_dir.clone();
    run(cmd, workspace)
}
