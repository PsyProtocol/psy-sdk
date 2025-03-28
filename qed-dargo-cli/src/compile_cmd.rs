use crate::Result;
use clap::Args;
use qed_dargo::workspace::Workspace;
use qed_interpreter::Interpreter;
use qedlang_core::dpn::{
    ops::{exec_context::QExecContext, sym_felt::SymFeltRef},
    vm::compile::QEDCompileResult,
};
/// Compile the program and its secret execution trace into ACIR format
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
) -> anyhow::Result<()> {
    let entry_manager = crate::resolve_entries(workspace);
    let mut interpreter = Interpreter::<SymFeltRef, _>::new(QExecContext::new());
    let compile_results = interpreter.interpret(
        entry_manager.entry,
        entry_manager.dependencies_entries.into_iter().collect(),
        compile_options.contract_name.clone(),
        compile_options.method_names.clone(),
        |context, (method_name, method_id, outputs)| {
            QEDCompileResult::compile_exec(
                method_name,
                method_id,
                &context.store,
                &context,
                &outputs,
            )
        },
    )?;
    println!("compile_result: {:?}", compile_results);
    Ok(())
}

/// Options for the compile command
#[derive(Args, Clone, Debug, Default)]
pub struct CompileOptions {
    #[clap(short, env, long, default_value = None)]
    contract_name: Option<String>,
    #[clap(short, env, long, num_args = 1.., default_values = &["main"])]
    method_names: Vec<String>,
}
