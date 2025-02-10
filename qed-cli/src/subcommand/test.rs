use plonky2::field::{goldilocks_field::GoldilocksField, types::Field};
use qed_common_circuit::circuits::zk_signature3::manager::SimpleQEDZKSignatureManager;
use qed_core::{config::network_constants::GLOBAL_USER_TREE_HEIGHT, data::qhashout::QHashOut};
use qed_crypto::signature::zk::wallet::SimpleQEDPrivateKey;
use qed_exec::vm::exec::QEDEvalSessionResult;
use qed_interpreter::Interpreter;
use qed_sema::TypeChecker;
use qed_utils::{
    gen_contract_deploy_and_circuits_for_functions, prepare_environment_with_real_contract,
    TestArgs, C, D,
};
use qedlang_core::dpn::{
    ops::{exec_context::QExecContext, sym_felt::SymFeltRef},
    vm::compile::QEDCompileResult,
};

pub fn run(args: TestArgs) -> anyhow::Result<()> {
    let mut interpreter = Interpreter::<SymFeltRef, GoldilocksField, _>::new(QExecContext::new());
    let res = interpreter.test(args.file.into())?;

    let priv_key = QHashOut::rand();
    let mut wallet = SimpleQEDZKSignatureManager::<C, D>::new();
    let pub_key = wallet.add_private_key(SimpleQEDPrivateKey::new(priv_key));
    let contract_state_tree_height = GLOBAL_USER_TREE_HEIGHT as usize;

    for (method_name, method_id, outputs) in res {
        let compile_result = QEDCompileResult::compile_exec(
            method_name,
            method_id,
            &interpreter.context.store,
            &interpreter.context,
            &outputs,
        );

        for (i, def) in compile_result.definitions.iter().enumerate() {
            println!("def{}: {:?}", i, def);
        }

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
    }
    Ok(())
}
