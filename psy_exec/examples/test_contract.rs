use std::marker::PhantomData;

use kvq::memory::simple::KVQSimpleMemoryBackingStore;
use plonky2::field::{goldilocks_field::GoldilocksField, types::Field};
use psy_core::data::qhashout::QHashOut;
use psy_crypto::hash::utils::gen_dapen_contract_function_method_id;
use psy_data::{
    config::store_config::PsyHasher,
    protocol::circuit_fingerprints::PsyWorkerToolboxCoreCircuitFingerprints,
    qblock::{
        cmds::{core::PsyBlockCommands, deploy_contract::QBCDeployContract, register_user::QBCRegisterUser},
        process::simple::SimpleBlockProcessor,
    },
    qdata::contract::{ContractCodeDefinition, ContractFunctionCodeDefinition},
    qstore::imm::cmd_processor::PsyReadCommandProcessorSync,
    traits::qdatastore::qtreedata::PsyComboDataStoreReaderWriterSync,
};
use psy_exec::vm::exec::PsyEvalSessionResult;
use psy_data::qstore::controllers::proving_session::PsyLocalProvingSessionStore;
use psy_store::{prepare_environment_with_real_contract, node::coordinator::PsyCoordinatorStoreWriterAsyncImm};
use psy_vm::dpn::{
    ops::{context_trait::DPNContext, exec_context::QExecContext, sym_felt::SymFeltRef},
    vm::{compile::PsyCompileResult, def::DPNFunctionCircuitDefinition},
};
use psylang_macros::qcontract;

type Felt = SymFeltRef;

pub struct SimpleContractStateless<C: DPNContext<Felt>> {
    _phantom: PhantomData<C>,
}
impl<C: DPNContext<Felt>> SimpleContractStateless<C> {
    pub fn new() -> Self {
        Self { _phantom: PhantomData }
    }
}

#[qcontract]
impl<C: DPNContext<Felt>> SimpleContractStateless<C> {
    pub fn simple_math(&mut self, ctx: &mut C, a: Felt, b: Felt) -> Felt {
        let k = (a + 2) * (2 * 2) * b - 3 * (a + b);
        let z = k + a;

        ctx.assert_true(z > 12, "z must be gt than 12");
        z
    }
    pub fn simple_state_update(&mut self, ctx: &mut C, read_index: Felt, write_index: Felt, write_value: Felt) -> Felt {
        let read_value = ctx.get_state_hash_at(read_index);
        let read_vv = read_value[0];
        if write_value > 100 {
            ctx.cset_state_hash_at(write_index, [write_value, 0, 0, 0]);
        }
        read_vv
    }
    pub fn if_test_2(&mut self, ctx: &mut C, a: Felt, b: Felt) -> Felt {
        let mut c = a * b;
        let mut k = 123;
        if a < b {
            c = a + b;
        } else if a == b {
            c = a;
        } else if a == 1337 {
            c = b;
        } else {
            k = 456;
        }
        c + k
    }
}

async fn test_run_contract_fn<R: PsyReadCommandProcessorSync<GoldilocksField> + Send + Sync>(
    contract_id: GoldilocksField,
    fn_circuit_def: &DPNFunctionCircuitDefinition,
    lps: &mut PsyLocalProvingSessionStore<GoldilocksField, R>,
    inputs: &[GoldilocksField],
) -> anyhow::Result<Vec<GoldilocksField>> {
    let outputs = PsyEvalSessionResult::new()
        .exec_contract_call(lps, contract_id, &fn_circuit_def, inputs.to_vec())
        .await?
        .outputs;

    //fn_circuit_def.

    Ok(outputs)
}
fn test_compile_contract() -> anyhow::Result<DPNFunctionCircuitDefinition> {
    let mut ctx = QExecContext::new();
    let mut contract = SimpleContractStateless::new();
    let a = ctx.add_input();
    let b = ctx.add_input();
    let c = ctx.add_input();
    let z = contract.simple_state_update(&mut ctx, a, b, c);
    let outputs = vec![z];
    let method_args = [("a".to_string(), 1usize), ("b".to_string(), 1), ("c".to_string(), 1)];
    let method_name = "simple_math".to_string();
    let method_id = gen_dapen_contract_function_method_id(method_name.clone(), &method_args);
    let fn_circuit_def = PsyCompileResult::compile_exec("simple_math".to_string(), method_id, &ctx.store, &ctx, &outputs);

    Ok(fn_circuit_def)
}
#[tokio::main]
async fn main() {
    let compiled = test_compile_contract().unwrap();
    let contract_id = GoldilocksField::ONE;

    let deployer_key = QBCRegisterUser::new_from_u64s([1; 4], [13371, 13372, 13373, 13374]);
    let deploy_contract = QBCDeployContract {
        deployer: deployer_key.get_public_key::<PsyHasher>(),
        code_definition: ContractCodeDefinition {
            state_tree_height: 16,
            functions: vec![ContractFunctionCodeDefinition::default()],
        },
        function_whitelist: vec![
            QHashOut::rand(),
            QHashOut::rand(),
            QHashOut::from_values(1, 0, 0, 0),
            QHashOut::from_values(0, 0, 0, 0),
        ],
    };

    let mut lps = prepare_environment_with_real_contract(
        vec![
            QBCRegisterUser::new_from_u64s([1; 4], [1; 4]),
            deployer_key,
            QBCRegisterUser::new(QHashOut::rand(), QHashOut::rand()),
        ],
        vec![deploy_contract],
        Some(1),
        Some(GoldilocksField::ONE),
        Some(16),
    )
    .await
    .unwrap();

    let result = test_run_contract_fn(
        contract_id,
        &compiled,
        &mut lps,
        &[
            GoldilocksField::from_canonical_u64(1),   // read index
            GoldilocksField::from_canonical_u64(2),   // write index
            GoldilocksField::from_canonical_u64(123), // write value
        ],
    )
    .await
    .unwrap();
    let result2 = test_run_contract_fn(
        contract_id,
        &compiled,
        &mut lps,
        &[
            GoldilocksField::from_canonical_u64(2),    //read index
            GoldilocksField::from_canonical_u64(0),    // write index
            GoldilocksField::from_canonical_u64(1337), // write value
        ],
    )
    .await
    .unwrap();
    println!("outputs: {:?}", result);
    println!("outputs: {:?}", result2);
}
