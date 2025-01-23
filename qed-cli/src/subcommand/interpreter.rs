use plonky2::field::goldilocks_field::GoldilocksField;
use qed_builder::{
    exec_circuit_function_vm, DPNContext, IExecutionContext, QEDCompileResult, QExecContext,
    SymFeltRef,
};
use qed_interpreter::Interpreter;
use qed_sema::{CheckedValue, CheckedValueNode, CheckedValueOrNode, SymbolTable, TypeChecker};
use qed_utils::InterpreterArgs;

pub fn run(args: InterpreterArgs) -> anyhow::Result<()> {
    let mut interpreter = Interpreter::<SymFeltRef, _>::new(
        QExecContext::new(),
        0,
        SymFeltRef::from(0),
        SymFeltRef::from(0),
    );
    let mut typecheker = TypeChecker::new();
    let mut symbols = SymbolTable::new();
    let a = CheckedValue::Felt(interpreter.context.add_input());
    let b = CheckedValue::Felt(interpreter.context.add_input());
    let res = interpreter
        .interpret(&mut typecheker, args.file.into(), vec![a, b], &mut symbols)
        .expect("interpret failed")
        .expect("return value not found");
    interpreter.inputs.push(2);
    interpreter.inputs.push(3);

    let ctx = interpreter.context.clone();

    let compile_result = QEDCompileResult::compile_exec(
        "test".to_owned(),
        &ctx.store,
        &ctx,
        &[res.try_as_felt().unwrap()],
    );

    for (i, def) in compile_result.definitions.iter().enumerate() {
        println!("def: {}: {:?}", i, def);
    }

    let result_vm = exec_circuit_function_vm(
        interpreter.inputs,
        compile_result,
        IExecutionContext::<GoldilocksField>::new(),
    );

    println!("result_vm: {:?}", result_vm);
    Ok(())
}
