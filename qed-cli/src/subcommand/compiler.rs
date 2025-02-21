use qed_interpreter::Interpreter;
use qed_utils::CompilerArgs;
use qedlang_core::dpn::{
    ops::{exec_context::QExecContext, sym_felt::SymFeltRef},
    vm::compile::QEDCompileResult,
};

pub fn run(args: CompilerArgs) -> anyhow::Result<()> {
    let mut interpreter = Interpreter::<SymFeltRef, _>::new(QExecContext::new());
    let compile_results = interpreter.interpret(
        args.file.into(),
        args.contract_name,
        args.method_names,
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
