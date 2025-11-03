use std::{fs::File, io::Write};

use plonky2::{field::goldilocks_field::GoldilocksField, plonk::config::PoseidonGoldilocksConfig};
use psy_common_circuit::circuits::{
    lookalikes::{get_agg_state_transition_type_d_common_data, get_end_cap_type_e_common_data, get_guta_type_c_common_data},
    traits::qstandard::QStandardCircuit,
};
use psy_config::PSY_NETWORK_MAGIC;
use psy_common::{data::qhashout::QHashOut, job::id::ProvingJobCircuitType};
use psy_crypto::common::generic_circuit_verifier::GenericCircuitVerifier;
use psy_data::config::store_config::PsyFelt;
use psy_network_circuit::{coordinator::coordinator_helper::PsyCoordinatorCircuitManager, guta::guta_helper::PsyGUTACircuitManager};
use psy_ups_circuit::circuit_manager::core::PsyUPSStepCircuitManager;

fn run_gen_config() -> anyhow::Result<()> {
    const D: usize = 2;
    type C = PoseidonGoldilocksConfig;
    type F = PsyFelt;

    let mut gcv = GenericCircuitVerifier::<C, D>::new();

    gcv.common
        .insert_common_data(ProvingJobCircuitType::TypeC, get_guta_type_c_common_data::<C, D>());
    gcv.common
        .insert_common_data(ProvingJobCircuitType::TypeD, get_agg_state_transition_type_d_common_data::<C, D>());
    gcv.common
        .insert_common_data(ProvingJobCircuitType::TypeE, get_end_cap_type_e_common_data::<C, D>());

    let main_circuits = PsyUPSStepCircuitManager::<C, D>::new_with_config(PSY_NETWORK_MAGIC);

    gcv.register_circuit_triplet(ProvingJobCircuitType::UserEndCap, main_circuits.ups_end_cap.get_verifier_triplet());

    use psy_config::get_default_worker_public_key;
    let guta_circuits = PsyGUTACircuitManager::<C, D>::new_with_config(
        main_circuits.ups_end_cap.get_common_circuit_data_ref(),
        main_circuits.ups_end_cap.get_verifier_config_ref().constants_sigmas_cap.height(),
        main_circuits.ups_end_cap.get_fingerprint(),
        get_default_worker_public_key::<F>(),
    );

    gcv.register_circuit_triplet(
        ProvingJobCircuitType::GUTASingleEndCap,
        guta_circuits.verify_single_end_cap.get_verifier_triplet(),
    );
    gcv.register_circuit_triplet(
        ProvingJobCircuitType::GUTATwoEndCap,
        guta_circuits.verify_two_end_cap.get_verifier_triplet(),
    );
    gcv.register_circuit_triplet(ProvingJobCircuitType::GUTATwoGUTA, guta_circuits.verify_two_guta.get_verifier_triplet());
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
        ProvingJobCircuitType::GUTATwoGUTAWithCheckpointUpgrade,
        guta_circuits.verify_two_guta_upgrade_checkpoint.get_verifier_triplet(),
    );

    gcv.register_circuit_triplet(
        ProvingJobCircuitType::GUTAVerifyToCapWithCheckpointUpgrade,
        guta_circuits.verify_guta_to_cap_upgrade_checkpoint.get_verifier_triplet(),
    );

    gcv.register_circuit_triplet(
        ProvingJobCircuitType::GUTAOnlyRegisterUsers,
        guta_circuits.only_register_users.get_verifier_triplet(),
    );
    gcv.register_circuit_triplet(ProvingJobCircuitType::GUTANoChange, guta_circuits.no_change.get_verifier_triplet());
    let coordinator_circuits = PsyCoordinatorCircuitManager::<C, D>::new_with_guta(guta_circuits, get_default_worker_public_key::<F>());

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
    let library_path = "./psy_core/psy_crypto/src/common/cached_circuit_library.rs";
    let mut library_file = File::create(&library_path)?;
    write!(
        library_file,
        r##"// AUTOGENERATED - DO NOT MODIFY
use plonky2::hash::hash_types::RichField;
use super::simple_circuit_library::{{SerializableSimpleCircuitLibrary, SimpleCircuitLibrary}};

pub fn get_cached_circuit_library<F: RichField>() -> SimpleCircuitLibrary<F> {{
    SimpleCircuitLibrary::from_serialized(
        serde_json::from_str::<SerializableSimpleCircuitLibrary<F>>(
            r#"{}"#
        ).unwrap()
    )
}}
"##,
        library_data
    )?;

    // Write to cached_common_data.rs
    let common_path = "./psy_node/src/common/verifier/cached_common_data.rs";
    let mut common_file = File::create(&common_path)?;
    write!(
        common_file,
        r##"// AUTOGENERATED - DO NOT MODIFY
use plonky2::plonk::config::{{AlgebraicHasher, GenericConfig}};
use psy_common_circuit::circuits::lookalikes::{{
    get_agg_state_transition_type_d_common_data, get_end_cap_type_e_common_data,
    get_guta_type_c_common_data,
}};
use psy_crypto::common::generic_circuit_verifier::{{
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
"##,
        common_info_data
    )?;

    Ok(())
}

fn main() {
    run_gen_config().unwrap();
}
