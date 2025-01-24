use std::path::PathBuf;

use plonky2::field::goldilocks_field::GoldilocksField;
use qed_ast::{AstVisitor, IdentId, ModuleId, Program};
use qed_builder::{
    exec_circuit_function_vm, DPNContext, IExecutionContext, QEDCompileResult, QExecContext,
    SymFeltRef,
};
use qed_fmt::{Formatter, FormatterContext};
use qed_interpreter::{Interpreter, PreprocessorContext, StorageProcessor};
use qed_parser::Parser;
use qed_sema::{
    Artifact, CheckedFunctionNode, CheckedValue, CheckedValueNode, CheckedValueOrNode, SymbolTable,
    Type, TypeChecker,
};
use qed_utils::{CompilerArgs, InterpreterArgs};

pub fn run(args: CompilerArgs) -> anyhow::Result<()> {
    let entry: PathBuf = args.file.into();
    let mut interpreter = Interpreter::<SymFeltRef, _>::new(
        QExecContext::new(),
        0,
        SymFeltRef::from(0),
        SymFeltRef::from(0),
    );
    let mut typechecker = TypeChecker::new();
    let mut symbols = SymbolTable::new();

    let mut program = Program::new();
    let mut parser = Parser::new(&mut program);
    parser.parse(&mut interpreter.context, entry).unwrap();

    let mut storage_preprocessor: StorageProcessor = StorageProcessor::new();
    let mut preprocessor_context: PreprocessorContext<'_, SymFeltRef, QExecContext> =
        PreprocessorContext::new(&mut program);
    storage_preprocessor.visit_program(&mut preprocessor_context);

    let mut formatter_context: FormatterContext<SymFeltRef, QExecContext> =
        FormatterContext::new(&program);

    let mut formatter = Formatter::new();
    formatter.visit_program(&mut formatter_context);
    println!("formatted:\n{}", formatter.get_output());
    println!("ast:\n{:#?}", program);

    let mut artifact = Artifact::new(program);
    typechecker.typecheck_program(&mut symbols, &mut artifact)?;

    let scope_id = symbols[ModuleId::root()].scope_id;
    let type_id = symbols[scope_id].types.get(&IdentId::MAIN.into()).unwrap();
    let node: CheckedFunctionNode =
        (symbols[type_id.clone()].clone().as_ref() as &CheckedFunctionNode).clone();

    let mut parameters = vec![];
    for (id, _, ty) in node.parameters.iter() {
        parameters.push(
            symbols[ty.clone()]
                .clone()
                .to_value(&mut symbols, &mut interpreter.context),
        );
    }

    let res = interpreter
        .interpret_function(&mut typechecker, &artifact, &node, parameters, &mut symbols)?
        .expect("return value not found");

    let ctx = interpreter.context.clone();

    let compile_result =
        QEDCompileResult::compile_exec("test".to_owned(), &ctx.store, &ctx, &res.to_array());

    println!("compile_result: {:?}", compile_result);
    Ok(())
}
