use std::marker::PhantomData;

use kvq::memory::simple::KVQSimpleMemoryBackingStore;
use qed_store::node::coordinator::QEDCoordinatorStoreWriterAsyncImm;
use plonky2::{
    field::{goldilocks_field::GoldilocksField, types::Field},
    plonk::config::PoseidonGoldilocksConfig,
};
use qed_common_circuit::circuits::{
    traits::qstandard::QStandardCircuit, zk_signature3::manager::SimpleQEDZKSignatureManager,
};
use qed_core::{
    config::network_constants::{GLOBAL_USER_TREE_HEIGHT, UPS_SESSION_PROOF_TREE_HEIGHT},
    data::qhashout::QHashOut,
    utils::debug_timer::DebugTimer,
};
use psy_crypto::{
    hash::utils::gen_dapen_contract_function_method_id, signature::zk::wallet::SimpleQEDPrivateKey,
};
use qed_data::{
    protocol::circuit_fingerprints::QEDWorkerToolboxCoreCircuitFingerprints,
    qblock::cmds::{
        core::QEDBlockCommands, deploy_contract::QBCDeployContract, register_user::QBCRegisterUser,
    },
    qdata::contract::{ContractCodeDefinition, ContractFunctionCodeDefinition}, qstore::imm::cmd_processor::QEDReadCommandProcessorSync,
};
use qed_exec::vm::{cfc_input::DapenContractFunctionCircuitInput, exec::QEDEvalSessionResult};
use qed_prover::dpn::{
    circuits::cfc::DapenContractFunctionCircuit,
};
use qed_data::{
    config::store_config::QEDHasher, qblock::process::simple::SimpleBlockProcessor, traits::qdatastore::{qmetadata::QMetaDataStoreReaderSync, qtreedata::QEDComboDataStoreReaderWriterSync}
};
use qed_store::controllers::local::{proving_session::QEDLocalProvingSessionStore, prepare_environment_with_real_contract};
use qedlang_core::dpn::{
    ops::{context_trait::DPNContext, exec_context::QExecContext, sym_felt::SymFeltRef},
    vm::{compile::QEDCompileResult, def::DPNFunctionCircuitDefinition},
};
use qedlang_macros::qcontract;

type Felt = SymFeltRef;

pub struct SimpleContractStateful<C: DPNContext<Felt>> {
    _phantom: PhantomData<C>,
}
impl<C: DPNContext<Felt>> SimpleContractStateful<C> {
    pub fn new() -> Self {
        Self {
            _phantom: PhantomData,
        }
    }
}

#[qcontract]
impl<C: DPNContext<Felt>> SimpleContractStateful<C> {
    pub fn simple_mint_debug(&mut self, ctx: &mut C, amount: Felt) -> Felt {
        let self_user_leaf = ctx.get_state_hash_at(ctx.get_user_id());
        //[balance,alt1,alt2,alt3]
        let current_balance = self_user_leaf[0];

        let new_balance = current_balance + amount;
        ctx.cset_state_hash_at(
            ctx.get_user_id(),
            [
                new_balance,
                self_user_leaf[1],
                self_user_leaf[2],
                self_user_leaf[3],
            ],
        );

        new_balance
    }
    pub fn simple_transfer(&mut self, ctx: &mut C, recipient: Felt, amount: Felt) -> Felt {
        let self_user_id = ctx.get_user_id();
        let self_user_leaf = ctx.get_state_hash_at(self_user_id);

        let current_balance = self_user_leaf[0];

        ctx.assert_true(amount <= current_balance, "insufficient balance");

        let new_balance = current_balance - amount;
        ctx.assert_true(new_balance < current_balance, "user balance overflow");

        ctx.cset_state_hash_at(
            self_user_id,
            [
                new_balance,
                self_user_leaf[1],
                self_user_leaf[2],
                self_user_leaf[3],
            ],
        );

        let p2p_leaf = ctx.get_state_hash_at(recipient);
        let previous_total_sent_to_recipient = p2p_leaf[2];

        let new_total_sent_to_recipient = previous_total_sent_to_recipient + amount;
        ctx.assert_true(
            new_total_sent_to_recipient > previous_total_sent_to_recipient,
            "sent amount overflow",
        );

        ctx.cset_state_hash_at(
            recipient,
            [
                p2p_leaf[0],
                p2p_leaf[1],
                new_total_sent_to_recipient,
                p2p_leaf[3],
            ],
        );
        current_balance
    }
    pub fn simple_claim(&mut self, ctx: &mut C, sender: Felt) -> Felt {
        let self_user_id = ctx.get_user_id();
        ctx.assert_true(sender != self_user_id, "you cannot claim from your self");

        let self_leaf = ctx.get_state_hash_at(self_user_id);
        let current_balance = self_leaf[0];

        let loc_transfer_info_for_sender = ctx.get_state_hash_at(sender);
        let loc_previous_total_recieved_from_sender = loc_transfer_info_for_sender[0];

        let sender_transfer_info_leaf_for_me = ctx.get_other_user_contract_state_hash_at(
            0,
            sender,
            ctx.get_contract_id(),
            self_user_id,
        );

        let sender_total_sent_to_me = sender_transfer_info_leaf_for_me[2];

        ctx.assert_true(
            sender_total_sent_to_me > loc_previous_total_recieved_from_sender,
            "no tokens to claim from this sender",
        );

        let tokens_to_claim = sender_total_sent_to_me - loc_previous_total_recieved_from_sender;

        let loc_new_total_recieved_from_sender = sender_total_sent_to_me;

        ctx.cset_state_hash_at(
            sender,
            [
                loc_new_total_recieved_from_sender,
                loc_transfer_info_for_sender[1],
                loc_transfer_info_for_sender[2],
                loc_transfer_info_for_sender[3],
            ],
        );

        let new_balance = tokens_to_claim + current_balance;
        ctx.assert_true(current_balance < new_balance, "balance overflow");

        ctx.cset_state_hash_at(
            self_user_id,
            [new_balance, self_leaf[1], self_leaf[2], self_leaf[3]],
        );

        new_balance
    }
}

const D: usize = 2;
type C = PoseidonGoldilocksConfig;

async fn test_run_contract_fn<R: QEDReadCommandProcessorSync<GoldilocksField> + Send + Sync>(
    contract_id: GoldilocksField,
    fn_circuit_def: &DPNFunctionCircuitDefinition,
    lps: &mut QEDLocalProvingSessionStore<GoldilocksField, R>,
    inputs: &[GoldilocksField],
) -> anyhow::Result<DapenContractFunctionCircuitInput<GoldilocksField>> {
    QEDEvalSessionResult::new().exec_contract_call(
        lps,
        contract_id,
        fn_circuit_def,
        inputs.to_vec(),
    ).await
}

fn compile_simple_mint_debug() -> anyhow::Result<DPNFunctionCircuitDefinition> {
    let mut ctx = QExecContext::new();
    let mut contract = SimpleContractStateful::new();
    let amount = ctx.add_input();
    let z = contract.simple_mint_debug(&mut ctx, amount);
    let outputs = vec![z];
    let method_args = [("amount".to_string(), 1usize)];
    let method_name = "simple_mint_debug".to_string();
    let method_id = gen_dapen_contract_function_method_id(method_name.clone(), &method_args);
    let fn_circuit_def = QEDCompileResult::compile_exec(
        "simple_mint_debug".to_string(),
        method_id,
        &ctx.store,
        &ctx,
        &outputs,
    );

    Ok(fn_circuit_def)
}
fn compile_simple_transfer() -> anyhow::Result<DPNFunctionCircuitDefinition> {
    let mut ctx = QExecContext::new();
    let mut contract = SimpleContractStateful::new();
    let recipient = ctx.add_input();
    let amount = ctx.add_input();
    let z = contract.simple_transfer(&mut ctx, recipient, amount);
    let outputs = vec![z];
    let method_args = [
        ("recipient".to_string(), 1usize),
        ("amount".to_string(), 1usize),
    ];
    let method_name = "simple_transfer".to_string();
    let method_id = gen_dapen_contract_function_method_id(method_name.clone(), &method_args);
    let fn_circuit_def = QEDCompileResult::compile_exec(
        "simple_transfer".to_string(),
        method_id,
        &ctx.store,
        &ctx,
        &outputs,
    );

    Ok(fn_circuit_def)
}

fn compile_simple_claim() -> anyhow::Result<DPNFunctionCircuitDefinition> {
    let mut ctx = QExecContext::new();
    let mut contract = SimpleContractStateful::new();
    let sender = ctx.add_input();
    let z = contract.simple_claim(&mut ctx, sender);
    let outputs = vec![z];
    let method_args = [("sender".to_string(), 1usize)];
    let method_name = "simple_claim".to_string();
    let method_id = gen_dapen_contract_function_method_id(method_name.clone(), &method_args);
    let fn_circuit_def = QEDCompileResult::compile_exec(
        "simple_claim".to_string(),
        method_id,
        &ctx.store,
        &ctx,
        &outputs,
    );

    Ok(fn_circuit_def)
}

async fn test_prove_simple() -> anyhow::Result<()> {
    let mut timer = DebugTimer::new("test_prove_simple");

    timer.lap("start");

    let simple_mint_debug_def = compile_simple_mint_debug()?;
    timer.lap("compiled simple_mint_debug");
    let simple_transfer_def = compile_simple_transfer()?;
    timer.lap("compiled simple_transfer");
    let simple_claim_def = compile_simple_claim()?;
    timer.lap("compiled simple_claim");

    let contract_state_tree_height = GLOBAL_USER_TREE_HEIGHT as usize;
    const D: usize = 2;
    type C = PoseidonGoldilocksConfig;

    let deployer = QHashOut::rand();
    let defs_array = [simple_mint_debug_def, simple_transfer_def, simple_claim_def];
    timer.lap("start building circuits");

    use qed_prover::session::gen_contract_deploy_and_circuits_for_functions;

    let (result_circuits, deploy_cmd) = gen_contract_deploy_and_circuits_for_functions::<C, D>(
        deployer,
        contract_state_tree_height as u8,
        &defs_array,
    )?;
    let mut result_circuits = result_circuits;
    timer.lap("finished building fn circuits");
    let priv_key = QHashOut::rand();
    let mut wallet = SimpleQEDZKSignatureManager::<C, D>::new();
    let priv_key_w = SimpleQEDPrivateKey::new(priv_key);
    let pub_key_param = priv_key_w.get_public_key_param::<QEDHasher>();
    let pub_key = wallet.add_private_key(priv_key_w);
    timer.lap("finished building wallet/zksig circuits");

    let mut lps = prepare_environment_with_real_contract(
        vec![QBCRegisterUser::new(wallet.get_zksig_circuit_fingerprint(), pub_key_param)],
        vec![deploy_cmd],
        None,
        None,
        Some(UPS_SESSION_PROOF_TREE_HEIGHT as usize),
    ).await?;
    timer.lap("prepared environement");

    let contract_id = GoldilocksField::from_canonical_u64(2);

    let [simple_mint_debug_def, simple_transfer_def, _simple_claim_def] = defs_array;

    let cfc_input = test_run_contract_fn(
        contract_id,
        &simple_mint_debug_def,
        &mut lps,
        &[GoldilocksField::from_noncanonical_u64(133700)],
    ).await?;

    timer.lap("generated witness input");
    println!("witnesss_json:\n{:?}", &cfc_input);
    //println!("witnesss_json:\n{}",serde_json::to_string(&result).unwrap());

    timer.lap("start: setup circuits");

    let _simple_claim_circuit = result_circuits.pop().unwrap();
    let simple_transfer_circuit = result_circuits.pop().unwrap();
    let simple_mint_debug_circuit = result_circuits.pop().unwrap();
    timer.lap("end: setup circuits");

    //println!("common_looks_like: \n{:?}\n\n\n", simple_mint_debug_circuit.get_common_circuit_data_ref());
    let proof = simple_mint_debug_circuit.prove_base(&cfc_input).unwrap();

    timer.lap("proved");
    println!("public_inputs: {:?}", &proof.public_inputs);

    let cfc_input = test_run_contract_fn(
        contract_id,
        &simple_transfer_def,
        &mut lps,
        &[
            GoldilocksField::from_noncanonical_u64(2),
            GoldilocksField::from_noncanonical_u64(1000),
        ],
    ).await?;

    timer.lap("generated witness input");
    println!("witnesss_json:\n{:?}", &cfc_input);
    //println!("witnesss_json:\n{}",serde_json::to_string(&result).unwrap());

    //println!("common_looks_like: \n{:?}\n\n\n", simple_mint_debug_circuit.get_common_circuit_data_ref());
    let proof = simple_transfer_circuit.prove_base(&cfc_input).unwrap();

    timer.lap("proved");
    println!("public_inputs: {:?}", &proof.public_inputs);

    Ok(())
}

#[tokio::main]
async fn main() {
    test_prove_simple().await.unwrap();
}
