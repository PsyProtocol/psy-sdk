use plonky2::field::{goldilocks_field::GoldilocksField, types::Field};
use qed_builder::{
    exec_circuit_function_vm, DPNContext, IExecutionContext, QEDCompileResult, QExecContext,
    SymFeltRef, ToFelts,
};
use qed_interpreter::Interpreter;
use qed_sema::{CheckedValue, CheckedValueNode, SymbolTable, TypeChecker};
use qed_utils::InterpreterArgs;

pub fn run(args: InterpreterArgs) -> anyhow::Result<()> {
    let mut interpreter = Interpreter::<SymFeltRef, _>::new(QExecContext::new());
    let mut typecheker = TypeChecker::new();
    let params = args
        .params
        .iter()
        .map(|_| CheckedValue::Felt(interpreter.context.add_input()))
        .collect::<Vec<_>>();
    let res = interpreter
        .interpret(&mut typecheker, args.file.into(), params)?
        .expect("return value not found");
    interpreter.inputs.extend(args.params);

    let ctx = interpreter.context.clone();

    let compile_result =
        QEDCompileResult::compile_exec("test".to_owned(), 0, &ctx, &res.to_felts());

    for (i, def) in compile_result.definitions.iter().enumerate() {
        println!("def{}: {:?}", i, def);
    }

    let result_vm = exec_circuit_function_vm(
        interpreter.inputs,
        compile_result,
        IExecutionContext::<GoldilocksField>::new(
            GoldilocksField::from_canonical_u64(1),
            GoldilocksField::from_canonical_u64(2),
            GoldilocksField::from_canonical_u64(3),
            GoldilocksField::from_canonical_u64(4),
            [GoldilocksField::from_canonical_u64(5); 4],
        ),
    );

    println!("result_vm: {:?}", result_vm);
    Ok(())
}
