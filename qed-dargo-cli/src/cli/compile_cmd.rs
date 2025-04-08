use crate::cli::save_build_artifact_to_file;
use crate::errors::Result;
use clap::Args;
use qed_dargo::workspace::Workspace;
use qedlang_core::dpn::vm::def::DPNFunctionCircuitDefinition;
use std::path::PathBuf;

/// Compile the program and its secret execution trace
#[derive(Debug, Clone, Args)]
pub(crate) struct CompileCommand {
    #[clap(flatten)]
    compile_options: CompileOptions,
}

pub(crate) fn run(args: CompileCommand, workspace: Workspace) -> Result<()> {
    compile_workspace_full(&workspace, &args.compile_options)?;
    Ok(())
}

/// Parse and compile the entire workspace, then report errors.
/// This is the main entry point used by all other commands that need compilation.
pub(super) fn compile_workspace_full(
    workspace: &Workspace,
    compile_options: &CompileOptions,
) -> Result<Vec<DPNFunctionCircuitDefinition>> {
    let entry_manager = super::resolve_entries(workspace, compile_options.entry_path.clone())?;
    let compile_results = qed_interpreter::interpret(
        compile_options.contract_name.clone(),
        compile_options.method_names.clone(),
        entry_manager.entry,
        entry_manager.dependencies_entries.into_iter().collect(),
    )?;
    if compile_options.debug {
        println!("workspace: {:?}", workspace);
        println!("compile_result: {:?}", compile_results);
    } else {
        save_build_artifact_to_file(
            &compile_results,
            &workspace.package.name.to_string(),
            &workspace.target_dir,
        )?;
    }
    Ok(compile_results)
}

/// Options for the compile command
#[derive(Args, Clone, Debug, Default)]
pub struct CompileOptions {
    #[clap(short, long, default_value = None)]
    contract_name: Option<String>,
    #[clap(short, long, num_args = 1.., default_values = &["main"])]
    pub method_names: Vec<String>,
    #[clap(long, hide = true, default_value = None)]
    entry_path: Option<PathBuf>,
    #[clap(long, hide = true, default_value = "false")]
    debug: bool,
}
