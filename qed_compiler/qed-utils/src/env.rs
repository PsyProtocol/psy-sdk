use kvq::memory::simple::KVQSimpleMemoryBackingStore;
use plonky2::{
    field::{goldilocks_field::GoldilocksField, types::Field},
    plonk::config::PoseidonGoldilocksConfig,
};
use qed_common_circuit::circuits::traits::qstandard::QStandardCircuit;
use qed_core::{
    config::network_constants::UPS_SESSION_PROOF_TREE_HEIGHT, data::qhashout::QHashOut,
};
use qed_data::{
    protocol::circuit_fingerprints::QEDWorkerToolboxCoreCircuitFingerprints,
    qblock::cmds::{
        core::QEDBlockCommands, deploy_contract::QBCDeployContract, register_user::QBCRegisterUser,
    },
    qdata::contract::{ContractCodeDefinition, ContractFunctionCodeDefinition},
};
use qed_prover::dpn::{
    circuits::cfc::DapenContractFunctionCircuit, data::dapen_fc_to_cfc_code_definition,
};
use qed_data::{
    config::store_config::QEDHasher,
    qblock::process::simple::SimpleBlockProcessor,
    traits::qdatastore::{
        qmetadata::QMetaDataStoreReaderSync, qtreedata::QEDComboDataStoreReaderWriterSync,
    },
};
use qed_store::controllers::local::proving_session::QEDLocalProvingSessionStore;
use qedlang_core::dpn::vm::def::DPNFunctionCircuitDefinition;

pub const D: usize = 2;
pub type C = PoseidonGoldilocksConfig;

pub fn prepare_environment_with_real_contract(
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

pub fn gen_contract_deploy_and_circuits_for_functions(
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
    let mut fingerprints = Vec::with_capacity(defs.len() * 2);
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

            // sibling is [method_id, (num_outputs<<32)|num_inputs, 0, 0]
            let inputs_outputs_combo =
                ((x.circuit_outputs.len() as u64) << 32u64) | (x.circuit_inputs.len() as u64);
            fingerprints.push(QHashOut::from_values(
                x.method_id as u64,
                inputs_outputs_combo,
                0,
                0,
            ));
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
