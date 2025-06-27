use std::marker::PhantomData;

use kvq::memory::simple::KVQSimpleMemoryBackingStore;
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
use qed_crypto::{
    hash::utils::gen_dapen_contract_function_method_id, signature::zk::wallet::SimpleQEDPrivateKey,
};
use qed_data::{
    protocol::circuit_fingerprints::QEDWorkerToolboxCoreCircuitFingerprints,
    qblock::cmds::{
        core::QEDBlockCommands, deploy_contract::QBCDeployContract, register_user::QBCRegisterUser,
    },
    qdata::contract::{ContractCodeDefinition, ContractFunctionCodeDefinition},
};
use qed_exec::vm::{cfc_input::DapenContractFunctionCircuitInput, exec::QEDEvalSessionResult};
use qed_prover::dpn::{
    circuits::cfc::DapenContractFunctionCircuit, data::dapen_fc_to_cfc_code_definition,
};
use qed_store::{
    config::store_config::QEDHasher,
    controllers::local::proving_session::QEDLocalProvingSessionStore,
    qblock::process::simple::SimpleBlockProcessor,
    store::imm::cmd_processor::QEDReadCommandProcessorSync,
    traits::qdatastore::{
        qmetadata::QMetaDataStoreReaderSync, qtreedata::QEDComboDataStoreReaderWriterSync,
    },
};
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
fn gen_contract_deploy_and_circuits_for_functions(
    deployer: QHashOut<GoldilocksField>,
    contract_state_tree_height: u8,
    defs: &[DPNFunctionCircuitDefinition],
) -> anyhow::Result<(
    Vec<DapenContractFunctionCircuit<C, D>>,
    QBCDeployContract<GoldilocksField>,
)> {
    let code_defs = defs
        .iter()
        .map(|x| dapen_fc_to_cfc_code_definition(x))
        .collect::<Vec<_>>();
    let mut fingerprints = Vec::with_capacity(defs.len());
    let circuits = defs
        .iter()
        .map(|x| {
            let c = DapenContractFunctionCircuit::<C, D>::new(
                x,
                contract_state_tree_height as usize,
                UPS_SESSION_PROOF_TREE_HEIGHT as usize,
                false,
            );
            fingerprints.push(c.get_fingerprint());
            c
        })
        .collect::<Vec<_>>();

    let deploy = QBCDeployContract {
        deployer,
        code_definition: ContractCodeDefinition {
            state_tree_height: contract_state_tree_height as u16,
            functions: code_defs,
        },
        function_whitelist: fingerprints,
    };

    Ok((circuits, deploy))
}
fn prepare_environment_with_real_contract(
    new_user_public_key: QBCRegisterUser<GoldilocksField>,
    deploy_contract: QBCDeployContract<GoldilocksField>,
) -> anyhow::Result<
    QEDLocalProvingSessionStore<
        GoldilocksField,
        KVQSimpleMemoryBackingStore,
    >,
> {
    let whitelist_items_fake = vec![
        QHashOut::rand(),
        QHashOut::rand(),
        QHashOut::rand(),
        QHashOut::rand(),
    ];
    let st = KVQSimpleMemoryBackingStore::new();
    st.initialize_store()?;
    let dummy_fingerprints = QEDWorkerToolboxCoreCircuitFingerprints::default();
    SimpleBlockProcessor::process_block(
        &st,
        &QEDBlockCommands {
            register_users: vec![
                QBCRegisterUser::new_from_u64s([1; 4], [1; 4]),
                QBCRegisterUser::new_from_u64s([1; 4], [13371, 13372, 13373, 13374]),
                QBCRegisterUser::new_from_u64s([1; 4], [13375, 13376, 13377, 13378]),
                QBCRegisterUser::new(QHashOut::rand(), QHashOut::rand()),
                QBCRegisterUser::new(QHashOut::rand(), QHashOut::rand()),
                new_user_public_key,
            ],
            deploy_contracts: vec![
                QBCDeployContract {
                    deployer: QBCRegisterUser::new_from_u64s([1; 4], [13371, 13372, 13373, 13374])
                        .get_public_key::<QEDHasher>(),
                    code_definition: ContractCodeDefinition {
                        state_tree_height: 12 as u16,
                        functions: vec![ContractFunctionCodeDefinition::default()],
                    },
                    function_whitelist: whitelist_items_fake.to_vec(),
                },
                QBCDeployContract {
                    deployer: QBCRegisterUser::new_from_u64s([1; 4], [13375, 13376, 13377, 13378])
                        .get_public_key::<QEDHasher>(),
                    code_definition: ContractCodeDefinition {
                        state_tree_height: 13 as u16,
                        functions: vec![ContractFunctionCodeDefinition::default()],
                    },
                    function_whitelist: whitelist_items_fake.to_vec(),
                },
                deploy_contract,
            ],
            update_users: vec![],
        },
        &dummy_fingerprints,
    )?;

    SimpleBlockProcessor::process_block(
        &st,
        &QEDBlockCommands {
            register_users: vec![
                QBCRegisterUser::new(QHashOut::rand(), QHashOut::rand()),
                QBCRegisterUser::new(QHashOut::rand(), QHashOut::rand()),
            ],
            deploy_contracts: vec![],
            update_users: vec![],
        },
        &dummy_fingerprints,
    )?;

    SimpleBlockProcessor::process_block(
        &st,
        &QEDBlockCommands {
            register_users: vec![
                QBCRegisterUser::new(QHashOut::rand(), QHashOut::rand()),
                QBCRegisterUser::new(QHashOut::rand(), QHashOut::rand()),
            ],
            deploy_contracts: vec![],
            update_users: vec![],
        },
        &dummy_fingerprints,
    )?;

    let latest_l2_block_state = st.get_latest_l2_block_state()?;

    let lps: QEDLocalProvingSessionStore<
        GoldilocksField,
        KVQSimpleMemoryBackingStore,
    > = QEDLocalProvingSessionStore::new_at(
        st,
        GoldilocksField::from_noncanonical_u64(latest_l2_block_state.checkpoint_id),
        GoldilocksField::from_noncanonical_u64(5),
        GoldilocksField::ONE,
        UPS_SESSION_PROOF_TREE_HEIGHT as usize,
    );

    Ok(lps)
}

fn test_run_contract_fn<R: QEDReadCommandProcessorSync<GoldilocksField> + Send + Sync>(
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
    )
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

fn test_prove_simple() -> anyhow::Result<()> {
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

    let (result_circuits, deploy_cmd) = gen_contract_deploy_and_circuits_for_functions(
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
        QBCRegisterUser::new(wallet.get_zksig_circuit_fingerprint(), pub_key_param),
        deploy_cmd,
    )?;
    timer.lap("prepared environement");

    let contract_id = GoldilocksField::from_canonical_u64(2);

    let [simple_mint_debug_def, simple_transfer_def, _simple_claim_def] = defs_array;

    let cfc_input = test_run_contract_fn(
        contract_id,
        &simple_mint_debug_def,
        &mut lps,
        &[GoldilocksField::from_noncanonical_u64(133700)],
    )?;

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
    )?;

    timer.lap("generated witness input");
    println!("witnesss_json:\n{:?}", &cfc_input);
    //println!("witnesss_json:\n{}",serde_json::to_string(&result).unwrap());

    //println!("common_looks_like: \n{:?}\n\n\n", simple_mint_debug_circuit.get_common_circuit_data_ref());
    let proof = simple_transfer_circuit.prove_base(&cfc_input).unwrap();

    timer.lap("proved");
    println!("public_inputs: {:?}", &proof.public_inputs);

    Ok(())
}

fn main() {
    test_prove_simple().unwrap();
}
