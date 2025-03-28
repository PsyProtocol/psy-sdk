mod compile_cmd;

use crate::errors::Result;
use clap::{Args, Parser, Subcommand};
use qed_dargo::package::Dependency;
use qed_dargo::workspace::Workspace;
use qed_dargo_toml::files::{find_file_manifest_root, get_package_manifest};
use qed_dargo_toml::resolve_workspace_from_toml;
use std::collections::HashSet;
use std::path::PathBuf;

pub(crate) fn start_cli() -> Result<()> {
    let DargoCli { command, config } = DargoCli::parse();
    match command {
        DargoCommand::Compile(args) => with_workspace(args, config, compile_cmd::run),
    }?;
    Ok(())
}

#[derive(Parser, Debug)]
#[command(name="dargo", author, about, long_about = None)]
struct DargoCli {
    #[command(subcommand)]
    command: DargoCommand,

    #[clap(flatten)]
    config: DargoConfig,
}

#[non_exhaustive]
#[derive(Subcommand, Clone, Debug)]
enum DargoCommand {
    #[command(alias = "build")]
    Compile(compile_cmd::CompileCommand),
}

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
fn parse_path(path: &str) -> std::result::Result<PathBuf, String> {
    use qed_dargo::fm::NormalizePath;
    let mut path: PathBuf = path
        .parse()
        .map_err(|e| format!("failed to parse path: {e}"))?;
    if !path.is_absolute() {
        path = std::env::current_dir().unwrap().join(path).normalize();
    }
    Ok(path)
}

fn with_workspace<C, R>(cmd: C, config: DargoConfig, run: R) -> Result<()>
where
    R: FnOnce(C, Workspace) -> Result<()>,
{
    // All commands need to run on the workspace level, because that's where the `target` directory is.
    let package_dir = find_file_manifest_root(&config.program_dir)?;
    let toml_path = get_package_manifest(&package_dir)?;
    // Resolve the workspace from the toml file. It will download dependencies as well.
    let mut workspace = resolve_workspace_from_toml(&toml_path)?;
    if let Some(target_dir) = &config.target_dir {
        workspace.target_dir = target_dir.clone();
    }
    run(cmd, workspace)
}

#[derive(Clone, Debug)]
pub struct EntryManager {
    entry: PathBuf,
    dependencies_entries: HashSet<PathBuf>,
}

impl EntryManager {
    pub fn new(entry: PathBuf) -> Self {
        Self {
            entry,
            dependencies_entries: HashSet::new(),
        }
    }

    pub fn add_dependency_entry(&mut self, entry: PathBuf) -> bool {
        self.dependencies_entries.insert(entry)
    }
}

fn resolve_entries(workspace: &Workspace) -> EntryManager {
    let package = workspace.package.clone();
    let package_entry_path = package.entry_canonical_path();
    let mut entry_manager = EntryManager::new(package_entry_path);
    let mut package_stack = vec![package];
    while let Some(package) = package_stack.pop() {
        for dep in package.dependencies.values() {
            match dep {
                Dependency::Remote { package } | Dependency::Local { package } => {
                    let entry_path = package.entry_canonical_path();
                    if entry_manager.add_dependency_entry(entry_path) {
                        package_stack.push(package.clone());
                    }
                }
            }
        }
    }
    entry_manager
}
