use std::{marker::PhantomData, sync::Arc};

use kvq::memory::simple::KVQSimpleMemoryBackingStore;
use plonky2::{field::{goldilocks_field::GoldilocksField, types::{Field, PrimeField64}}, plonk::config::PoseidonGoldilocksConfig};
use qed_common_circuit::circuits::{traits::qstandard::QStandardCircuit, zk_signature3::manager::SimpleQEDZKSignatureManager};
use qed_core::{config::network_constants::{GLOBAL_USER_TREE_HEIGHT, QED_NETWORK_MAGIC_REGTEST, UPS_SESSION_PROOF_TREE_HEIGHT}, data::qhashout::QHashOut, ups::circuits::{LocalCircuitId, LocalCircuitType}, utils::debug_timer::DebugTimer};
use qed_crypto::{hash::utils::gen_dapen_contract_function_method_id, signature::zk::wallet::SimpleQEDPrivateKey};
use qed_data::{
    guta::api::SubmitUserEndCapProofAPIInput, proof_store::simple::SimpleProofStoreMemory, protocol::circuit_fingerprints::QEDWorkerToolboxCoreCircuitFingerprints, qblock::cmds::{
        core::QEDBlockCommands, deploy_contract::QBCDeployContract, register_user::QBCRegisterUser,
    }, qdata::contract::{ContractCodeDefinition, ContractFunctionCodeDefinition}
};
use qed_prover::{dpn::{circuits::cfc::DapenContractFunctionCircuit, data::dapen_fc_to_cfc_code_definition}, local::{provider::ProveProxyRpcTrait, simple::SimpleAPI}, ups::{circuit_manager::core::{QCircuitManager, QEDUPSStepCircuitManager}, session::UserProvingSessionManager}};
use qed_rollup_circuit::guta::guta_helper::QEDGUTACircuitManager;
use qed_data::{
    config::store_config::QEDHasher, qblock::process::simple::SimpleBlockProcessor, traits::qdatastore::{qmetadata::QMetaDataStoreReaderSync, qtreedata::QEDComboDataStoreReaderWriterSync}
};
use qed_store::{controllers::local::{proving_session::QEDLocalProvingSessionStore, session_info::SessionCircuitInfoStore}, node::coordinator::QEDCoordinatorStoreWriterAsyncImm};
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
    pub fn simple_mint_debug(
        &mut self,
        ctx: &mut C,
        amount: Felt,
    ) -> Felt {


        let self_user_leaf = ctx.get_state_hash_at(ctx.get_user_id());
        //[balance,alt1,alt2,alt3]
        let current_balance = self_user_leaf[0];

        let new_balance = current_balance+amount;
        ctx.cset_state_hash_at(ctx.get_user_id(), [
            new_balance,
            self_user_leaf[1],
            self_user_leaf[2],
            self_user_leaf[3],
        ]);

        new_balance
    }
    pub fn simple_transfer(
        &mut self,
        ctx: &mut C,
        recipient: Felt,
        amount: Felt,
    ) -> Felt {

        let self_user_id = ctx.get_user_id();
        let self_user_leaf = ctx.get_state_hash_at(self_user_id);

        let current_balance= self_user_leaf[0];

        ctx.assert_true(amount <= current_balance, "insufficient balance");

        let new_balance = current_balance - amount;
        ctx.assert_true(new_balance < current_balance, "user balance overflow");

        ctx.cset_state_hash_at(self_user_id, [
            new_balance,
            self_user_leaf[1],
            self_user_leaf[2],
            self_user_leaf[3],
        ]);


        let p2p_leaf = ctx.get_state_hash_at(recipient);
        let previous_total_sent_to_recipient = p2p_leaf[2];

        let new_total_sent_to_recipient = previous_total_sent_to_recipient + amount;
        ctx.assert_true(new_total_sent_to_recipient > previous_total_sent_to_recipient, "sent amount overflow");



        ctx.cset_state_hash_at(recipient, [
            p2p_leaf[0],
            p2p_leaf[1],
            new_total_sent_to_recipient,
            p2p_leaf[3],
        ]);
        current_balance

    }
    pub fn simple_claim(
        &mut self,
        ctx: &mut C,
        sender: Felt,
    ) -> Felt {

        let self_user_id = ctx.get_user_id();
        ctx.assert_true(sender != self_user_id, "you cannot claim from your self");

        let self_leaf = ctx.get_state_hash_at(self_user_id);
        let current_balance = self_leaf[0];

        let loc_transfer_info_for_sender = ctx.get_state_hash_at(sender);
        let loc_previous_total_recieved_from_sender = loc_transfer_info_for_sender[0];

        let sender_transfer_info_leaf_for_me = ctx.get_other_user_contract_state_hash_at(0, sender, ctx.get_contract_id(), self_user_id);

        let sender_total_sent_to_me = sender_transfer_info_leaf_for_me[2];

        ctx.assert_true(sender_total_sent_to_me > loc_previous_total_recieved_from_sender, "no tokens to claim from this sender");

        let tokens_to_claim = sender_total_sent_to_me - loc_previous_total_recieved_from_sender;

        let loc_new_total_recieved_from_sender = sender_total_sent_to_me;

        ctx.cset_state_hash_at(sender,[
            loc_new_total_recieved_from_sender,
            loc_transfer_info_for_sender[1],
            loc_transfer_info_for_sender[2],
            loc_transfer_info_for_sender[3],
        ]);

        let new_balance = tokens_to_claim+current_balance;
        ctx.assert_true(current_balance < new_balance, "balance overflow");


        ctx.cset_state_hash_at(self_user_id,[
            new_balance,
            self_leaf[1],
            self_leaf[2],
            self_leaf[3],
        ]);

        new_balance

    }
}

const D: usize = 2;
type C = PoseidonGoldilocksConfig;
fn gen_contract_deploy_and_circuits_for_functions(

    deployer: QHashOut<GoldilocksField>,
    contract_state_tree_height: u8,
    defs: &[DPNFunctionCircuitDefinition],
) -> anyhow::Result<(Vec<DapenContractFunctionCircuit<C,D>>, QBCDeployContract<GoldilocksField>)>{

    let code_defs = defs.iter().map(|x| dapen_fc_to_cfc_code_definition(x)).collect::<Vec<_>>();
    let mut fingerprints = Vec::with_capacity(defs.len()*2);
    let circuits = defs.iter().map(|x| {

        let c = DapenContractFunctionCircuit::<C, D>::new(x, contract_state_tree_height as usize, UPS_SESSION_PROOF_TREE_HEIGHT as usize, false);
        fingerprints.push(c.get_fingerprint());

        // sibling is [method_id, (num_outputs<<32)|num_inputs, 0, 0]
        let inputs_outputs_combo = ((x.circuit_outputs.len() as u64)<<32u64)|(x.circuit_inputs.len() as u64);
        fingerprints.push(QHashOut::from_values(x.method_id as u64, inputs_outputs_combo, 0,0));
        c
    }).collect::<Vec<_>>();

    let deploy = QBCDeployContract{
        deployer,
        code_definition: ContractCodeDefinition {
            state_tree_height: contract_state_tree_height as u16,
            functions: code_defs,
        },
        function_whitelist: fingerprints,
    };

    Ok((circuits, deploy))
}
async fn prepare_environment_with_real_contract(
    new_user_public_keys: Vec<QBCRegisterUser<GoldilocksField>>,
    deploy_contract: QBCDeployContract<GoldilocksField>,
) -> anyhow::Result<
    (QEDLocalProvingSessionStore<
        GoldilocksField,
        Arc<KVQSimpleMemoryBackingStore>,
    >,
    Arc<KVQSimpleMemoryBackingStore>
)
> {
    let whitelist_items_fake = vec![
        QHashOut::rand(),
        QHashOut::rand(),
        QHashOut::rand(),
        QHashOut::rand(),
    ];
    let st = Arc::new(KVQSimpleMemoryBackingStore::new());

    st.initialize_store().await?;


    let dummy_fingerprints = QEDWorkerToolboxCoreCircuitFingerprints::default();
    SimpleBlockProcessor::process_block(
        &st,
        &QEDBlockCommands {
            register_users: [vec![
                QBCRegisterUser::new_from_u64s([1;4], [1;4]),
                QBCRegisterUser::new_from_u64s([1;4], [13371, 13372, 13373, 13374]),
                QBCRegisterUser::new_from_u64s([1;4], [13375, 13376, 13377, 13378]),
                QBCRegisterUser::new(QHashOut::rand(),QHashOut::rand()),
                QBCRegisterUser::new(QHashOut::rand(),QHashOut::rand()),
            ], new_user_public_keys].concat(),
            deploy_contracts: vec![
                QBCDeployContract {
                    deployer: QHashOut::from_values(13371, 13372, 13373, 13374),
                    code_definition: ContractCodeDefinition {
                        state_tree_height: 12 as u16,
                        functions: vec![ContractFunctionCodeDefinition::default()],
                    },
                    function_whitelist: whitelist_items_fake.to_vec(),
                },
                QBCDeployContract {
                    deployer: QHashOut::from_values(13375, 13376, 13377, 13378),
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
                QBCRegisterUser::new(QHashOut::rand(),QHashOut::rand()),
                QBCRegisterUser::new(QHashOut::rand(),QHashOut::rand()),
            ],
            deploy_contracts: vec![
            ],
            update_users: vec![],
        },
        &dummy_fingerprints,
    )?;

    SimpleBlockProcessor::process_block(
        &st,
        &QEDBlockCommands {
            register_users: vec![
                QBCRegisterUser::new(QHashOut::rand(),QHashOut::rand()),
                QBCRegisterUser::new(QHashOut::rand(),QHashOut::rand()),
            ],
            deploy_contracts: vec![
            ],
            update_users: vec![],
        },
        &dummy_fingerprints,
    )?;

    let latest_l2_block_state = st.get_latest_l2_block_state()?;


    let lps: QEDLocalProvingSessionStore<
        GoldilocksField,
        Arc<KVQSimpleMemoryBackingStore>,
    > = QEDLocalProvingSessionStore::new_at(
        st.clone(),
        GoldilocksField::from_noncanonical_u64(latest_l2_block_state.checkpoint_id),
        GoldilocksField::from_noncanonical_u64(5),
        GoldilocksField::ONE,
        UPS_SESSION_PROOF_TREE_HEIGHT as usize
    );

    Ok((lps, st))
}

/*

fn test_run_contract_fn<R: QEDReadCommandProcessorSync<GoldilocksField>>(
    contract_id: GoldilocksField,
    fn_circuit_def: &DPNFunctionCircuitDefinition,
    lps: &mut QEDLocalProvingSessionStore<GoldilocksField, R>,
    inputs: &[GoldilocksField],
) -> anyhow::Result<DapenContractFunctionCircuitInput<GoldilocksField>> {
    QEDEvalSessionResult::new()
        .exec_contract_call( lps,contract_id, fn_circuit_def, inputs.to_vec())
}
*/

fn compile_simple_mint_debug() -> anyhow::Result<DPNFunctionCircuitDefinition> {

    let mut ctx = QExecContext::new();
    let mut contract = SimpleContractStateful::new();
    let amount = ctx.add_input();
    let z = contract.simple_mint_debug(&mut ctx, amount);
    let outputs = vec![z];
    let method_args = [
        ("amount".to_string(), 1usize),
    ];
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
    let method_args = [
        ("sender".to_string(), 1usize),
    ];
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

async fn demo_user_proving_session() -> anyhow::Result<()> {
    let mut timer = DebugTimer::new("demo_user_proving_session");

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
    let defs_array = [
        simple_mint_debug_def,
        simple_transfer_def,
        simple_claim_def,
    ];
    timer.lap("start building circuits");

    let (result_circuits, deploy_cmd) = gen_contract_deploy_and_circuits_for_functions(
        deployer,
        contract_state_tree_height as u8,
        &defs_array,
    )?;
    let mut result_circuits = result_circuits;
    timer.lap("finished building fn circuits");
    let priv_key_0 = QHashOut::rand();
    let priv_key_1 = QHashOut::rand();
    let mut wallet = SimpleQEDZKSignatureManager::<C,D>::new();
    let pub_key_0 = wallet.add_private_key_get_info(SimpleQEDPrivateKey::new(priv_key_0));
    let pub_key_1 = wallet.add_private_key_get_info(SimpleQEDPrivateKey::new(priv_key_1));
    timer.lap("finished building wallet/zksig circuits");


    timer.lap("prepared environement");


    let contract_id = GoldilocksField::from_canonical_u64(2);

    let [
        simple_mint_debug_def,
        simple_transfer_def,
        simple_claim_def,
    ] = defs_array;

    timer.lap("start: setup circuits");

    let simple_claim_circuit = result_circuits.pop().unwrap();
    let simple_transfer_circuit = result_circuits.pop().unwrap();
    let simple_mint_debug_circuit = result_circuits.pop().unwrap();
    timer.lap("end: setup circuits");

    /*
    println!("\n\n[simple_claim_circuit.common]:\n{:?}",simple_claim_circuit.get_common_circuit_data_ref());
    println!("\n\n[simple_transfer_circuit.common]:\n{:?}",simple_transfer_circuit.get_common_circuit_data_ref());
    println!("\n\n[simple_mint_debug_circuit.common]:\n{:?}\n\n",simple_mint_debug_circuit.get_common_circuit_data_ref());
    */

    timer.lap("start: init QEDUPSStepCircuitManager");

    let main_circuits = QCircuitManager::Local(QEDUPSStepCircuitManager::<C, D>::new_with_config(QED_NETWORK_MAGIC_REGTEST));
    //main_circuits.print_common_config();

    timer.lap("end: init QEDUPSStepCircuitManager");



    let (lps, st) = prepare_environment_with_real_contract(
        vec![pub_key_0.into(), pub_key_1.into()],
        deploy_cmd,
    ).await?;

    timer.lap("start build guta circuits");

    let end_cap_proof_common_data = match &main_circuits {
        QCircuitManager::Local(manager) => manager.ups_end_cap.get_common_circuit_data_ref(),
        QCircuitManager::Rpc(provider) => unimplemented!(),
    };

    let guta_circuits = QEDGUTACircuitManager::<C,D>::new_with_config(
        end_cap_proof_common_data,
        main_circuits.ups_end_cap_circuit_verifier_config()?.constants_sigmas_cap.height(),
        main_circuits.ups_end_cap_circuit_fingerprint()?,
    );
    timer.lap("built guta circuits");
    let proof_store = SimpleProofStoreMemory::new();

    let mut api = SimpleAPI::<_,_,GoldilocksField,C,D>::new(proof_store, st, guta_circuits)?;
    //main_circuits.print_common_config();
    api.guta_circuits.print_common_config();



    let mut circuit_info = SessionCircuitInfoStore::new();

    circuit_info.register_circuit(
        LocalCircuitType::SimpleZKSignature.into(),
        wallet.circuit.get_fingerprint(),
        wallet.circuit.get_verifier_config_ref().into(),
    );

    main_circuits.register_info(&mut circuit_info);
    circuit_info.register_circuit(
        LocalCircuitId::new_cfc(
            contract_id.to_canonical_u64() as u32,
            simple_mint_debug_def.method_id
        ),
        simple_mint_debug_circuit.get_fingerprint(),
        simple_mint_debug_circuit.get_verifier_config_ref().into(),
    );
    circuit_info.register_circuit(
        LocalCircuitId::new_cfc(
            contract_id.to_canonical_u64() as u32,
            simple_transfer_def.method_id
        ),
        simple_transfer_circuit.get_fingerprint(),
        simple_transfer_circuit.get_verifier_config_ref().into(),
    );
    circuit_info.register_circuit(
        LocalCircuitId::new_cfc(
            contract_id.to_canonical_u64() as u32,
            simple_claim_def.method_id
        ),
        simple_claim_circuit.get_fingerprint(),
        simple_claim_circuit.get_verifier_config_ref().into(),
    );

    let mut mgr = UserProvingSessionManager::<GoldilocksField,QEDHasher,_,C,D>::new(
        lps,
        circuit_info,
        main_circuits.ups_circuit_whitelist_root()?
    )?;

    timer.lap("START USER PROVING SESSION");

    mgr.prove_ups_start(&main_circuits)?;
    timer.lap("proved ups_start");

    mgr.prove_contract_call(
        &main_circuits,
        contract_id,
        0,
        &simple_mint_debug_def,
        vec![
            GoldilocksField::from_noncanonical_u64(1000)
        ]
    )?;
    timer.lap("proved token.simple_mint_debug(amount: 1000)");



    mgr.prove_contract_call(
        &main_circuits,
        contract_id,
        1,
        &simple_transfer_def,
        vec![
            GoldilocksField::from_noncanonical_u64(2),
            GoldilocksField::from_noncanonical_u64(100),
        ]
    )?;
    timer.lap("proved token.simple_transfer(recipient: 2, amount: 100)");

    let new_nonce = GoldilocksField::from_noncanonical_u64(1);
    let sighash = mgr.get_sighash(QED_NETWORK_MAGIC_REGTEST, new_nonce);

    let signature_proof = wallet.zk_sign_for_private_key_value(priv_key_0, sighash)?;
    timer.lap("generated zk signature for UPS transaction batch");
    mgr.proof_tree_state.finalize_tree(&main_circuits)?;
    timer.lap("aggregated all UPS proofs into a single proof");
    let public_key_param =SimpleQEDPrivateKey::new(priv_key_0).get_public_key_param::<QEDHasher>();
    let end_cap_proof = mgr.prove_end_cap(
        &main_circuits,
         QED_NETWORK_MAGIC_REGTEST,
         new_nonce,
         wallet.circuit.get_fingerprint(),
         public_key_param,
        signature_proof,
         wallet.circuit.get_verifier_config_ref().to_owned()
    )?;
    timer.lap("Proved End Cap for UPS Session 🎉");

    // the end cap proof the proof that we send off to the network 🎉

    //main_circuits.ups_end_cap.circuit_data.verify(end_cap_proof)?;
    timer.lap("✅ Verified End Cap Proof");


    let user_a_api_input = SubmitUserEndCapProofAPIInput{
        input: mgr.get_api_input()?,
        proof: end_cap_proof,
    };

    let mut mgr = mgr.into_clean_for_user(GoldilocksField::from_canonical_u32(6))?;



    timer.lap("START USER PROVING SESSION");

    mgr.prove_ups_start(&main_circuits)?;
    timer.lap("proved ups_start");

    mgr.prove_contract_call(
        &main_circuits,
        contract_id,
        0,
        &simple_mint_debug_def,
        vec![
            GoldilocksField::from_noncanonical_u64(10000)
        ]
    )?;
    timer.lap("proved token.simple_mint_debug(amount: 10000)");



    mgr.prove_contract_call(
        &main_circuits,
        contract_id,
        1,
        &simple_transfer_def,
        vec![
            GoldilocksField::from_noncanonical_u64(3),
            GoldilocksField::from_noncanonical_u64(1337),
        ]
    )?;
    timer.lap("proved token.simple_transfer(recipient: 3, amount: 1337)");

    let new_nonce = GoldilocksField::from_noncanonical_u64(1);
    let sighash = mgr.get_sighash(QED_NETWORK_MAGIC_REGTEST, new_nonce);

    let signature_proof = wallet.zk_sign_for_private_key_value(priv_key_1, sighash)?;
    timer.lap("generated zk signature for UPS transaction batch");
    mgr.proof_tree_state.finalize_tree(&main_circuits)?;
    timer.lap("aggregated all UPS proofs into a single proof");
    let public_key_param =SimpleQEDPrivateKey::new(priv_key_1).get_public_key_param::<QEDHasher>();
    let end_cap_proof = mgr.prove_end_cap(
        &main_circuits,
         QED_NETWORK_MAGIC_REGTEST,
         new_nonce,
         wallet.circuit.get_fingerprint(),
         public_key_param,
        signature_proof,
         wallet.circuit.get_verifier_config_ref().to_owned()
    )?;
    timer.lap("Proved End Cap for UPS Session 🎉");

    // the end cap proof the proof that we send off to the network 🎉

    //main_circuits.ups_end_cap.circuit_data.verify(end_cap_proof)?;
    timer.lap("✅ Verified End Cap Proof");


    let user_b_api_input = SubmitUserEndCapProofAPIInput{
        input: mgr.get_api_input()?,
        proof: end_cap_proof,
    };

    api.submit_proof(user_a_api_input)?;
    api.submit_proof(user_b_api_input)?;
    timer.lap("start generating start witnesses");

    let (pairs, left_over) = api.get_start_witnesses()?;
    timer.lap("finished generating start witnesses");
    for p in pairs.into_iter() {
        let _proof = api.proof_start_dbg(p, &main_circuits.ups_end_cap_circuit_verifier_config()?)?;
        timer.lap("Proved Recursive Global User Tree Aggregation");

    }





    //guta_circuits.verify_two_end_cap.prove_base(input, child_a_proof, child_b_proof, end_cap_verifier_data)







    Ok(())
}

#[tokio::main]
async fn main() {
    demo_user_proving_session().await.unwrap();
}
