use std::{cell::RefCell, path::PathBuf, rc::Rc};

use plonky2::field::goldilocks_field::GoldilocksField;
use qed_ast::{AstVisitor, DefaultVisitorContext, IdentId, ModuleId, NodeType, Program};
use qed_builder::{
    exec_circuit_function_vm, DPNContext, IExecutionContext, QEDCompileResult, QExecContext,
    SymFeltRef, ToFelts,
};
use qed_fmt::Formatter;
use qed_interpreter::{Interpreter, StorageProcessor};
use qed_parser::Parser;
use qed_sema::{
    Artifact, CheckedFunctionNode, CheckedValue, CheckedValueNode, SymbolTable, Type, TypeChecker,
    TypeCheckerVisitorContext,
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

    let mut program = Program::new();
    let mut parser = Parser::new(&mut program);
    parser.parse(&mut interpreter.context, entry).unwrap();

    let mut storage_preprocessor: StorageProcessor = StorageProcessor::new();
    let mut default_visitor_context: DefaultVisitorContext<'_, SymFeltRef, QExecContext> =
        DefaultVisitorContext::new(&mut program);
    storage_preprocessor.visit_program(&mut default_visitor_context);

    let mut formatter = Formatter::new();
    formatter.visit_program(&mut default_visitor_context);
    println!("formatted:\n{}", formatter.get_output());
    println!("ast:\n{:#?}", program);

    let mut typechecker_context = TypeCheckerVisitorContext::new(program);
    typechecker.visit_program(&mut typechecker_context)?;

    let scope_id = typechecker_context.symbols[ModuleId::root()].scope_id;
    let type_id = typechecker_context.symbols[scope_id]
        .types
        .get(&IdentId::MAIN.into())
        .unwrap();
    let node: CheckedFunctionNode = (typechecker_context.symbols[type_id.clone()]
        .clone()
        .as_ref() as &CheckedFunctionNode)
        .clone();

    let mut parameters = vec![];
    for (id, _, ty) in node.parameters.iter() {
        parameters.push(Rc::new(RefCell::new(
            typechecker_context.symbols[ty.clone()]
                .clone()
                .to_value(&mut typechecker_context.symbols, &mut interpreter.context),
        )));
    }

    let res = interpreter
        .interpret_function(
            &mut typechecker,
            &node,
            parameters,
            &mut typechecker_context.symbols,
            Some(NodeType::Module),
        )
        .unwrap()
        .transpose()?
        .expect("return value not found");

    let ctx = interpreter.context.clone();

    let compile_result =
        QEDCompileResult::compile_exec("test".to_owned(), 0, &ctx, &res.borrow().to_felts());

    println!("compile_result: {:?}", compile_result);
    Ok(())
}
