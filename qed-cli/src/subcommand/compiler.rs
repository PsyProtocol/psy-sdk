use qed_builder::{QEDCompileResult, QExecContext, SymFeltRef, ToFelts};
use qed_interpreter::Interpreter;
use qed_sema::TypeChecker;
use qed_utils::CompilerArgs;

pub fn run(args: CompilerArgs) -> anyhow::Result<()> {
    let mut interpreter = Interpreter::<SymFeltRef, _>::new(
        QExecContext::new(),
        0,
        SymFeltRef::from(0),
        SymFeltRef::from(0),
    );
    let mut typecheker = TypeChecker::new();
    let res = interpreter
        .compile(&mut typecheker, args.file.into())?
        .expect("return value not found");

    let ctx = interpreter.context.clone();

    let compile_result =
        QEDCompileResult::compile_exec("test".to_owned(), 0, &ctx, &res.borrow().to_felts());

    println!("compile_result: {:?}", compile_result);
    Ok(())
}
