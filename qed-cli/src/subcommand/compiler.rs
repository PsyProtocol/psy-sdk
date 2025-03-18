use qed_interpreter::{error::Error, Interpreter};
use qed_utils::CompilerArgs;
use qedlang_core::dpn::{
    ops::{exec_context::QExecContext, sym_felt::SymFeltRef},
    vm::compile::QEDCompileResult,
};

pub fn run(args: CompilerArgs) -> anyhow::Result<()> {
    let mut interpreter = Interpreter::<SymFeltRef, _>::new(QExecContext::new());
    let compile_results = interpreter
        .interpret(
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
        )
        .unwrap_or_else(|err| {
            let report = qed_interpreter::error::lowering_error_to_report(err);
            report
                .eprint(ariadne::FnCache::new(|x: &String| {
                    Ok::<_, Error>(
                        std::fs::read_to_string(std::path::Path::new(x.as_str())).unwrap(),
                    )
                }))
                .unwrap();
            std::process::exit(1);
        });

    println!("compile_result: {:?}", compile_results);
    Ok(())
}
