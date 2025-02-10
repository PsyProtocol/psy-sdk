use plonky2::field::goldilocks_field::GoldilocksField;
use qed_ast::IdentId;
use qed_interpreter::Interpreter;
use qed_sema::TypeChecker;
use qed_utils::CompilerArgs;
use qedlang_core::dpn::{
    ops::{context_trait::ToFelts, exec_context::QExecContext, sym_felt::SymFeltRef},
    vm::compile::QEDCompileResult,
};

pub fn run(args: CompilerArgs) -> anyhow::Result<()> {
    let mut interpreter = Interpreter::<SymFeltRef, GoldilocksField, _>::new(QExecContext::new());
    let res = interpreter.compile(args.file.into(), args.contract_name, args.method_names)?;

    for (method_name, method_id, outputs) in res {
        let compile_result = QEDCompileResult::compile_exec(
            method_name,
            method_id,
            &interpreter.context.store,
            &interpreter.context,
            &outputs,
        );

        println!("compile_result: {:?}", compile_result);
    }
    Ok(())
}
