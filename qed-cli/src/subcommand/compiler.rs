use qed_interpreter::{error::Error, Interpreter};
use qed_utils::CompilerArgs;
use qedlang_core::dpn::{
    ops::{exec_context::QExecContext, sym_felt::SymFeltRef},
    vm::compile::QEDCompileResult,
};

pub fn run(args: CompilerArgs) -> anyhow::Result<()> {
    let mut interpreter = Interpreter::<SymFeltRef, _>::new(QExecContext::new());
    let (typechecker, mut ctx) = interpreter.typecheck(args.file.into()).unwrap();
    let compile_results = interpreter
        .interpret(
            &typechecker,
            &mut ctx,
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
        )
        .unwrap_or_else(|err| {
            let report = qed_interpreter::error::lowering_error_to_report(err, &mut ctx);
            println!("{}", report);
            std::process::exit(1);
        });

    println!("compile_result: {:?}", compile_results);
    Ok(())
}
