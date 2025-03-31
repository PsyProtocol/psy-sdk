use crate::errors::CliError;
use clap::Args;
use qed_dargo::workspace::Workspace;

/// Compile the program and its secret execution trace into ACIR format
#[derive(Debug, Clone, Args)]
pub(crate) struct CompileCommand {
    #[clap(flatten)]
    compile_options: CompileOptions,
}

pub(crate) fn run(args: CompileCommand, workspace: Workspace) -> Result<(), CliError> {
    compile_workspace_full(&workspace, &args.compile_options)?;

    Ok(())
}

/// Parse and compile the entire workspace, then report errors.
/// This is the main entry point used by all other commands that need compilation.
pub(super) fn compile_workspace_full(
    workspace: &Workspace,
    compile_options: &CompileOptions,
) -> Result<(), CliError> {
    Ok(())
}

/// Options for the compile command
#[derive(Args, Clone, Debug, Default)]
pub struct CompileOptions {}
