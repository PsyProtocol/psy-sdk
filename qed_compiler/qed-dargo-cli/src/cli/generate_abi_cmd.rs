use crate::cli::{save_build_artifact_to_file, resolve_crate_path_graph};
use crate::errors::Result;
use clap::Args;
use qed_ast::AbiExtractor;
use qed_dargo::workspace::Workspace;
use qed_interpreter::Interpreter;
use qedlang_core::dpn::ops::{exec_context::QExecContext, sym_felt::SymFeltRef};
use std::path::PathBuf;

/// Generate ABI (Application Binary Interface) file for the contract
#[derive(Debug, Clone, Args)]
pub(crate) struct GenerateAbiCommand {
    /// Name of the contract to generate ABI for
    #[clap(short, long)]
    pub contract_name: String,

    /// Path to the entry file (optional, uses package entry by default)
    #[clap(long)]
    pub entry_path: Option<PathBuf>,

    /// Output directory for the generated ABI file
    #[clap(short, long)]
    pub output_dir: Option<PathBuf>,

    /// Pretty print the ABI JSON output
    #[clap(long, default_value = "true")]
    pub pretty: bool,
}

pub(crate) fn run(args: GenerateAbiCommand, workspace: Workspace) -> Result<()> {
    // Resolve the crate path graph
    let crate_path_graph = resolve_crate_path_graph(&workspace, args.entry_path.clone());

    // Parse and type-check the code to build the AST
    let mut interpreter = Interpreter::<SymFeltRef, _>::new(QExecContext::new());
    let (_typechecker, mut ctx) = interpreter.typecheck(crate_path_graph)?;

    // Create ABI extractor
    let extractor = AbiExtractor::new(args.contract_name.clone());

    // Extract ABI from the program
    // SAFETY: We're extending the lifetime to 'static for the duration of this function call only.
    // This is safe because we own the program and it lives for the entire duration of this function.
    let program_ptr: *mut _ = &mut ctx.program;
    let contract_abi = unsafe {
        let static_program = &mut *(program_ptr as *mut qed_ast::Program<qedlang_core::dpn::ops::sym_felt::SymFeltRef>);
        extractor.extract_from_program(static_program)?
    };

    // Determine output directory
    let output_dir = args.output_dir.unwrap_or_else(|| workspace.target_dir.clone());

    // Generate ABI file name
    let abi_filename = format!("{}_abi", args.contract_name);

    if args.pretty {
        // Pretty print to console
        let json = contract_abi.to_json().map_err(|e| {
            crate::errors::CliError::Generic(format!("Failed to serialize ABI to JSON: {}", e))
        })?;
        println!("{}", json);
    }

    // Save ABI to file
    let abi_path = save_build_artifact_to_file(&contract_abi, &abi_filename, &output_dir)?;
    Ok(())
}