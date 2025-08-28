use crate::cli::doc_cmd::{extract_function_metadata_from_context, FunctionNode};
use crate::cli::save_build_artifact_to_file;
use crate::errors::Result;

use clap::Args;
use qed_package::Workspace;
use qedlang_core::dpn::vm::def::DPNFunctionCircuitDefinition;
use std::collections::HashMap;
use std::path::PathBuf;

/// Compile the program and its secret execution trace
#[derive(Debug, Clone, Args)]
pub struct CompileCommand {
    #[clap(flatten)]
    pub compile_options: CompileOptions,
}

pub fn run(args: CompileCommand, workspace: Workspace) -> Result<()> {
    compile_workspace_full(&workspace, &args.compile_options)?;
    Ok(())
}

pub struct CompilationResult {
    pub circuit_definitions: Vec<DPNFunctionCircuitDefinition>,
    pub function_metadata: HashMap<String, FunctionNode>,
}

/// Parse and compile the entire workspace, then report errors.
/// This is the main entry point used by all other commands that need compilation.
pub fn compile_workspace_full(
    workspace: &Workspace,
    compile_options: &CompileOptions,
) -> Result<CompilationResult> {
    let crate_path_graph =
        super::resolve_crate_path_graph(workspace, compile_options.entry_path.clone());

    let mut interpret_result = qed_interpreter::interpret(
        compile_options.contract_name.clone(),
        compile_options.method_names.clone(),
        crate_path_graph,
    )?;

    let function_metadata = extract_function_metadata_from_context(
        &mut interpret_result.ctx,
        &mut interpret_result.typechecker,
        &interpret_result.compile_results,
    );

    if compile_options.debug {
        println!("workspace: {:?}", workspace);
        println!("compile_result: {:?}", interpret_result.compile_results);
    } else {
        save_build_artifact_to_file(
            &interpret_result.compile_results,
            &workspace.package.name.to_string(),
            &workspace.target_dir,
        )?;
    }

    Ok(CompilationResult {
        circuit_definitions: interpret_result.compile_results,
        function_metadata,
    })
}

/// Options for the compile command
#[derive(Args, Clone, Debug, Default)]
pub struct CompileOptions {
    #[clap(short, long, default_value = None)]
    pub contract_name: Option<String>,
    #[clap(short, long, num_args = 1.., default_values = &["main"])]
    pub method_names: Vec<String>,
    #[clap(long, hide = true, default_value = None)]
    pub entry_path: Option<PathBuf>,
    #[clap(long, hide = true, default_value = "false")]
    pub debug: bool,
}
