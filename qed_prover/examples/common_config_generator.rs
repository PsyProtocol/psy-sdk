use plonky2::{field::goldilocks_field::GoldilocksField, plonk::config::PoseidonGoldilocksConfig};
use qed_common_circuit::circuits::{lookalikes::{get_agg_state_transition_type_d_common_data, get_end_cap_type_e_common_data, get_guta_type_c_common_data}, traits::qstandard::QStandardCircuit};
use qed_core::{config::network_constants::QED_NETWORK_MAGIC_REGTEST, data::qhashout::QHashOut, job::id::ProvingJobCircuitType};
use qed_crypto::common::generic_circuit_verifier::GenericCircuitVerifier;
use qed_prover::ups::circuit_manager::core::QEDUPSStepCircuitManager;
use qed_rollup_circuit::{coordinator::coordinator_helper::QEDCoordinatorCircuitManager, guta::guta_helper::QEDGUTACircuitManager};
use qed_data::config::store_config::QEDFelt;
use std::fs::File;
use std::io::Write;

fn run_gen_config() -> anyhow::Result<()> {
    const D: usize = 2;
    type C = PoseidonGoldilocksConfig;
    type F = QEDFelt;

    let mut gcv = GenericCircuitVerifier::<C,D>::new();

    gcv.common.insert_common_data(ProvingJobCircuitType::TypeC, get_guta_type_c_common_data::<C,D>());
    gcv.common.insert_common_data(ProvingJobCircuitType::TypeD, get_agg_state_transition_type_d_common_data::<C,D>());
    gcv.common.insert_common_data(ProvingJobCircuitType::TypeE, get_end_cap_type_e_common_data::<C,D>());

    let main_circuits = QEDUPSStepCircuitManager::<C, D>::new_with_config(QED_NETWORK_MAGIC_REGTEST);

    gcv.register_circuit_triplet(
        ProvingJobCircuitType::UserEndCap,
        main_circuits.ups_end_cap.get_verifier_triplet(),
    );

    let guta_circuits = QEDGUTACircuitManager::<C,D>::new_with_config(
        main_circuits.ups_end_cap.get_common_circuit_data_ref(),
        main_circuits.ups_end_cap.get_verifier_config_ref().constants_sigmas_cap.height(),
        main_circuits.ups_end_cap.get_fingerprint(),
    );

    gcv.register_circuit_triplet(
        ProvingJobCircuitType::GUTASingleEndCap,
        guta_circuits.verify_single_end_cap.get_verifier_triplet(),
    );
    gcv.register_circuit_triplet(
        ProvingJobCircuitType::GUTATwoEndCap,
        guta_circuits.verify_two_end_cap.get_verifier_triplet(),
    );
    gcv.register_circuit_triplet(
        ProvingJobCircuitType::GUTATwoGUTA,
        guta_circuits.verify_two_guta.get_verifier_triplet(),
    );
    gcv.register_circuit_triplet(
        ProvingJobCircuitType::GUTALeftGUTARightEndCap,
        guta_circuits.verify_left_guta_right_end_cap.get_verifier_triplet(),
    );
    gcv.register_circuit_triplet(
        ProvingJobCircuitType::GUTALeftEndCapRightGUTA,
        guta_circuits.verify_left_end_cap_right_guta.get_verifier_triplet(),
    );
    gcv.register_circuit_triplet(
        ProvingJobCircuitType::GUTARegisterUsers,
        guta_circuits.verify_guta_register_users.get_verifier_triplet(),
    );
    gcv.register_circuit_triplet(
        ProvingJobCircuitType::GUTAVerifyToCap,
        guta_circuits.verify_guta_to_cap.get_verifier_triplet(),
    );
    gcv.register_circuit_triplet(
        ProvingJobCircuitType::GUTAOnlyRegisterUsers,
        guta_circuits.only_register_users.get_verifier_triplet(),
    );
    gcv.register_circuit_triplet(
        ProvingJobCircuitType::GUTANoChange,
        guta_circuits.no_change.get_verifier_triplet(),
    );
    let coordinator_circuits = QEDCoordinatorCircuitManager::<C,D>::new_with_guta(guta_circuits);

    coordinator_circuits.register_library(&mut gcv.library);

    gcv.register_circuit_triplet(
        ProvingJobCircuitType::AppendUserRegistrationTree,
        coordinator_circuits.append_user_registration_tree.get_verifier_triplet(),
    );
    gcv.register_circuit_triplet(
        ProvingJobCircuitType::AppendUserRegistrationTreeAggregate,
        coordinator_circuits.agg_state_transition.get_verifier_triplet(),
    );
    gcv.register_circuit_triplet(
        ProvingJobCircuitType::DummyAppendUserRegistrationTreeAggregate,
        coordinator_circuits.dummy_agg_state_transition.get_verifier_triplet(),
    );

    gcv.register_circuit_triplet(
        ProvingJobCircuitType::BatchDeployContracts,
        coordinator_circuits.batch_deploy_contracts.get_verifier_triplet(),
    );
    gcv.register_circuit_triplet(
        ProvingJobCircuitType::BatchDeployContractsAggregate,
        coordinator_circuits.agg_state_transition.get_verifier_triplet(),
    );
    gcv.register_circuit_triplet(
        ProvingJobCircuitType::DummyBatchDeployContractsAggregate,
        coordinator_circuits.dummy_agg_state_transition.get_verifier_triplet(),
    );

    gcv.register_circuit_triplet(
        ProvingJobCircuitType::GenerateRollupStateTransitionProof,
        coordinator_circuits.checkpoint_root_transition.get_verifier_triplet(),
    );

    gcv.common.print_common();

    let gcv_ser = gcv.to_serialized();

    let library_data = serde_json::to_string(&gcv_ser.library)?;
    let common_info_data = serde_json::to_string(&gcv_ser.common)?;

    // Write to cached_circuit_library.rs
    let library_path = "./qed_crypto/src/common/cached_circuit_library.rs";
    let mut library_file = File::create(&library_path)?;
    write!(library_file, r##"// AUTOGENERATED - DO NOT MODIFY
use plonky2::hash::hash_types::RichField;
use super::simple_circuit_library::{{SerializableSimpleCircuitLibrary, SimpleCircuitLibrary}};

pub fn get_cached_circuit_library<F: RichField>() -> SimpleCircuitLibrary<F> {{
    SimpleCircuitLibrary::from_serialized(
        serde_json::from_str::<SerializableSimpleCircuitLibrary<F>>(
            r#"{}"#
        ).unwrap()
    )
}}
"##, library_data)?;

    // Write to cached_common_data.rs
    let common_path = "./qed_node_common/src/verifier/cached_common_data.rs";
    let mut common_file = File::create(&common_path)?;
    write!(common_file, r##"// AUTOGENERATED - DO NOT MODIFY
use plonky2::plonk::config::{{AlgebraicHasher, GenericConfig}};
use qed_common_circuit::circuits::lookalikes::{{
    get_agg_state_transition_type_d_common_data, get_end_cap_type_e_common_data,
    get_guta_type_c_common_data,
}};
use qed_crypto::common::generic_circuit_verifier::{{
    GenericCircuitCommonDataLibrary, SerializedGenericCircuitCommonDataLibraryInfo,
}};

pub fn get_cached_common_data_library<C: GenericConfig<D>, const D: usize>(
) -> GenericCircuitCommonDataLibrary<C, D>
where
    C::Hasher: AlgebraicHasher<C::F>,
{{
    let serialized_library =
        serde_json::from_str::<SerializedGenericCircuitCommonDataLibraryInfo>(
            r#"{}"#
        ).unwrap();

    GenericCircuitCommonDataLibrary::<C, D>::from_serialized(
        &serialized_library,
        vec![
            get_guta_type_c_common_data::<C, D>(),
            get_agg_state_transition_type_d_common_data::<C, D>(),
            get_end_cap_type_e_common_data::<C, D>(),
        ],
    )
    .unwrap()
}}
"##, common_info_data)?;

    Ok(())
}

fn main() {
    run_gen_config().unwrap();
}
