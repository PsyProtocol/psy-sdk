use clap::Args;
use plonky2::field::{goldilocks_field::GoldilocksField, types::Field};
use qed_common_circuit::circuits::zk_signature3::manager::SimpleQEDZKSignatureManager;
use qed_core::{config::network_constants::GLOBAL_USER_TREE_HEIGHT, data::qhashout::QHashOut};
use qed_crypto::signature::zk::wallet::SimpleQEDPrivateKey;
use qed_dargo::workspace::Workspace;
use qed_data::qblock::cmds::register_user::QBCRegisterUser;
use qed_exec::vm::exec::QEDEvalSessionResult;
use qed_interpreter::Interpreter;
use qed_store::config::store_config::QEDHasher;
use qed_utils::{
    gen_contract_deploy_and_circuits_for_functions, prepare_environment_with_real_contract, C, D,
};
use qedlang_core::dpn::{
    ops::{exec_context::QExecContext, sym_felt::SymFeltRef},
    vm::compile::QEDCompileResult,
};
use std::path::PathBuf;

#[derive(Debug, Clone, Args)]
pub(crate) struct TestCommand {
    #[clap(short, env, long)]
    pub file: PathBuf,
}
pub(crate) fn run(args: TestCommand) -> crate::errors::Result<()> {
    let mut interpreter = Interpreter::<SymFeltRef, _>::new(QExecContext::new());
    let (mut typechecker, mut ctx) = interpreter.typecheck(args.file, vec![])?;
    let compile_results = interpreter.test(
        &mut typechecker,
        &mut ctx,
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
    let wallet = SimpleQEDZKSignatureManager::<C, D>::new();
    let priv_key_w = SimpleQEDPrivateKey::new(priv_key);
    let pub_key_param = priv_key_w.get_public_key_param::<QEDHasher>();
    let contract_state_tree_height = GLOBAL_USER_TREE_HEIGHT as usize;

    let deployer = QHashOut::rand();
    let (circuits, deploy_cmd) = gen_contract_deploy_and_circuits_for_functions(
        deployer,
        contract_state_tree_height as u8,
        &compile_results,
    )?;

    let mut lps = prepare_environment_with_real_contract(
        QBCRegisterUser::new(wallet.get_zksig_circuit_fingerprint(), pub_key_param),
        deploy_cmd,
    )?;
    let contract_id = GoldilocksField::from_canonical_u64(2);

    for (def, circuit) in compile_results.into_iter().zip(circuits.into_iter()) {
        let cfc_input =
            QEDEvalSessionResult::new().exec_contract_call(&mut lps, contract_id, &def, vec![])?;
        println!("result_vm: {:?}", cfc_input.outputs);

        let proof = circuit.prove_base(&cfc_input).unwrap();
        println!("public_inputs: {:?}", &proof.public_inputs);
    }

    Ok(())
}
