use plonky2::field::goldilocks_field::GoldilocksField;
use qed_builder::{
    vm::{compile::QEDCompileResult, exec::IExecutionContext, runner::exec_circuit_function_vm},
    Context, ExecContext, SymFeltRef,
};
use qed_interpreter::Interpreter;
use qed_sema::{CheckedValueNode, CheckedValueOrNode, SymbolTable, TypeChecker};
use qed_utils::InterpreterArgs;

pub fn run(args: InterpreterArgs) -> anyhow::Result<()> {
    let path = args.file;
    let params = args.params;
    {
        let mut interpreter = Interpreter::<SymFeltRef, _>::new(ExecContext::new());
        // let cache = SymFeltEvalCache::new();
        // let store = SymFeltStore::new();
        let mut typecheker = TypeChecker::new();
        let mut symbols = SymbolTable::new();

        let params_interpret = params
            .iter()
            .map(|_p| CheckedValueOrNode::from(CheckedValueNode::Felt(interpreter.context.add_input())))
            .collect::<Vec<_>>();
        let res = interpreter
            .interpret(
                &mut typecheker,
                path.into(),
                params_interpret,
                &mut symbols,
            )
            .expect("interpret failed")
            .expect("return value not found");
        interpreter.inputs = params;

        let ctx = interpreter.context.clone();

        let compile_result = QEDCompileResult::compile_exec(
            "test".to_owned(),
            &ctx.store,
            &ctx,
            &[res.try_as_felt().unwrap()],
        );

        for (i, def) in compile_result.definitions.iter().enumerate() {
            println!("def{}: {:?}", i, def);
        }

        let result_vm = exec_circuit_function_vm(
            interpreter.inputs,
            compile_result,
            IExecutionContext::<GoldilocksField>::new(),
        );

        println!("result_vm: {:?}", result_vm);
    };
    Ok(())
}
