mod compile_cmd;
mod completion_cmd;
mod execute_cmd;
mod fmt_cmd;
mod init_cmd;
mod new_cmd;
mod test_cmd;

use crate::errors::{CliError, Result};
use clap::{Args, Parser, Subcommand};
use qed_dargo::package::Dependency;
use qed_dargo::workspace::Workspace;
use qed_dargo_toml::files::{find_file_manifest_root, get_package_manifest};
use qed_dargo_toml::resolve_workspace_from_toml;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

pub(crate) fn start_cli() -> Result<()> {
    let DargoCli { command, config } = DargoCli::parse();
    match command {
        DargoCommand::New(args) => new_cmd::run(args, config),
        DargoCommand::Init(args) => init_cmd::run(args, config),
        DargoCommand::Compile(args) => with_workspace(args, config, compile_cmd::run),
        DargoCommand::Execute(args) => with_workspace(args, config, execute_cmd::run),
        DargoCommand::Test(args) => test_cmd::run(args),
        DargoCommand::Fmt(args) => fmt_cmd::run(args),
        DargoCommand::Completion(args) => completion_cmd::run(args),
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
    New(new_cmd::NewCommand),
    Init(init_cmd::InitCommand),
    #[command(alias = "build")]
    Compile(compile_cmd::CompileCommand),
    Execute(execute_cmd::ExecuteCommand),
    Test(test_cmd::TestCommand),
    Fmt(fmt_cmd::FmtCommand),
    Completion(completion_cmd::CompletionCommand),
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
    let mut path: PathBuf = path
        .parse()
        .map_err(|e| format!("failed to parse path: {e}"))?;
    if !path.is_absolute() {
        path = std::env::current_dir().unwrap().join(path);
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
    pub entry: PathBuf,
    pub dependencies_entries: HashSet<PathBuf>,
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

pub fn resolve_entries(workspace: &Workspace, entry_path: Option<PathBuf>) -> Result<EntryManager> {
    let package = workspace.package.clone();
    let package_entry_path = match entry_path {
        Some(entry_path) => entry_path,
        None => package.entry_canonical_path(),
    };

    if !package_entry_path.exists() {
        return Err(CliError::MissingEntryFile {
            toml: workspace.root_dir.join("Dargo.toml"),
            entry: package_entry_path,
        });
    }
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
    Ok(entry_manager)
}

pub(crate) fn save_build_artifact_to_file<T: ?Sized + serde::Serialize>(
    build_artifact: &T,
    artifact_name: &str,
    output_dir: &Path,
) -> Result<PathBuf> {
    let artifact_path = output_dir.join(artifact_name).with_extension("json");
    let bytes = serde_json::to_vec(build_artifact)?;
    write_to_file(&bytes, &artifact_path)?;
    Ok(artifact_path)
}

// Create the parent directory if needed and write the bytes to a file.
pub fn write_to_file(bytes: &[u8], path: &Path) -> Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    std::fs::write(path, bytes)?;
    Ok(())
}
