use plonky2::field::{goldilocks_field::GoldilocksField, types::Field};
use qed_common_circuit::circuits::zk_signature3::manager::SimpleQEDZKSignatureManager;
use qed_core::{config::network_constants::GLOBAL_USER_TREE_HEIGHT, data::qhashout::QHashOut};
use qed_crypto::signature::zk::wallet::SimpleQEDPrivateKey;
use qed_exec::vm::exec::QEDEvalSessionResult;
use qed_interpreter::Interpreter;
use qed_sema::{CheckedValue, CheckedValueRef, TypeChecker};
use qed_utils::{
    gen_contract_deploy_and_circuits_for_functions, prepare_environment_with_real_contract,
    InterpreterArgs, TestArgs, C, D,
};
use qedlang_core::dpn::{
    ops::{context_trait::DPNContext, exec_context::QExecContext, sym_felt::SymFeltRef},
    vm::compile::QEDCompileResult,
};

pub fn run(args: InterpreterArgs) -> anyhow::Result<()> {
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

    let priv_key = QHashOut::rand();
    let mut wallet = SimpleQEDZKSignatureManager::<C, D>::new();
    let pub_key = wallet.add_private_key(SimpleQEDPrivateKey::new(priv_key));
    let contract_state_tree_height = GLOBAL_USER_TREE_HEIGHT as usize;

    let deployer = QHashOut::rand();
    let (mut circuits, deploy_cmd) = gen_contract_deploy_and_circuits_for_functions(
        deployer,
        contract_state_tree_height as u8,
        &compile_results,
    )?;

    let mut lps = prepare_environment_with_real_contract(pub_key, deploy_cmd)?;
    let contract_id = GoldilocksField::from_canonical_u64(2);

    for ((def, parameters), circuit) in compile_results
        .into_iter()
        .zip(args.parameters.into_iter())
        .zip(circuits.into_iter())
    {
        let cfc_input = QEDEvalSessionResult::new().exec_contract_call(
            &mut lps,
            contract_id,
            &def,
            parameters
                .into_iter()
                .map(GoldilocksField::from_noncanonical_u64)
                .collect(),
        )?;
        println!("result_vm: {:?}", cfc_input.outputs);

        let proof = circuit.prove_base(&cfc_input).unwrap();
        println!("public_inputs: {:?}", &proof.public_inputs);
    }

    Ok(())
}
