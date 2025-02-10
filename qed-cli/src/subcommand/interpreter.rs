use plonky2::field::{goldilocks_field::GoldilocksField, types::Field};
use qed_common_circuit::circuits::zk_signature3::manager::SimpleQEDZKSignatureManager;
use qed_core::{config::network_constants::GLOBAL_USER_TREE_HEIGHT, data::qhashout::QHashOut};
use qed_crypto::signature::zk::wallet::SimpleQEDPrivateKey;
use qed_exec::vm::exec::QEDEvalSessionResult;
use qed_interpreter::Interpreter;
use qed_sema::{CheckedValue, TypeChecker};
use qed_utils::{
    gen_contract_deploy_and_circuits_for_functions, prepare_environment_with_real_contract,
    InterpreterArgs, TestArgs, C, D,
};
use qedlang_core::dpn::{
    ops::{context_trait::DPNContext, exec_context::QExecContext, sym_felt::SymFeltRef},
    vm::compile::QEDCompileResult,
};

pub fn run(args: InterpreterArgs) -> anyhow::Result<()> {
    let mut interpreter = Interpreter::<SymFeltRef, GoldilocksField, _>::new(QExecContext::new());
    let params = args
        .params
        .iter()
        .map(|_| CheckedValue::Felt(interpreter.context.add_input()))
        .collect::<Vec<_>>();
    let (_, method_id, outputs) = interpreter.interpret(
        args.file.into(),
        args.contract_name.as_deref(),
        args.method_name.as_str(),
        params,
    )?;
    interpreter.inputs.extend(
        args.params
            .iter()
            .map(|x| GoldilocksField::from_canonical_u64(x.clone())),
    );

    let compile_result = QEDCompileResult::compile_exec(
        args.method_name,
        method_id,
        &interpreter.context.store,
        &interpreter.context,
        &outputs,
    );

    for (i, def) in compile_result.definitions.iter().enumerate() {
        println!("def{}: {:?}", i, def);
    }

    let priv_key = QHashOut::rand();
    let mut wallet = SimpleQEDZKSignatureManager::<C, D>::new();
    let pub_key = wallet.add_private_key(SimpleQEDPrivateKey::new(priv_key));
    let contract_state_tree_height = GLOBAL_USER_TREE_HEIGHT as usize;

    let deployer = QHashOut::rand();
    let defs_array = [compile_result.clone()];
    let (mut result_circuits, deploy_cmd) = gen_contract_deploy_and_circuits_for_functions(
        deployer,
        contract_state_tree_height as u8,
        &defs_array,
    )?;

    let mut lps = prepare_environment_with_real_contract(pub_key, deploy_cmd)?;
    let contract_id = GoldilocksField::ONE;

    let cfc_input = QEDEvalSessionResult::new().exec_contract_call(
        &mut lps,
        contract_id,
        &compile_result,
        interpreter.inputs.clone(),
    )?;
    println!("result_vm: {:?}", cfc_input.outputs);

    let circuit = result_circuits.pop().unwrap();
    let proof = circuit.prove_base(&cfc_input).unwrap();

    println!("public_inputs: {:?}", &proof.public_inputs);

    Ok(())
}
