
#[cfg(test)]
mod tests {
    use plonky2::{
        field::goldilocks_field::GoldilocksField, plonk::config::PoseidonGoldilocksConfig,
    };
    use qed_common_circuit::{
        circuits::{lookalikes::{get_agg_state_transition_type_d_lookalike_circuit_data, get_end_cap_type_e_lookalike_circuit_data, get_guta_type_c_lookalike_circuit_data}, traits::qstandard::QStandardCircuit},
        treeprover::{
            aggregation::{
                state_transition::AggStateTransitionCircuit,
                state_transition_dummy::AggStateTransitionDummyCircuit,
            },
            qrecursion::standard::manager::portable::circuits::PortableQTreeRecursionCircuits,
            traits::TreeProverAggCircuit,
        },
    };
    use qed_core::{
        config::network_constants::{
            QED_NETWORK_MAGIC_REGTEST, UPS_CIRCUIT_WHITELIST_TREE_HEIGHT,
            UPS_SESSION_PROOF_TREE_HEIGHT,
        },
        data::qhashout::QHashOut,
    };
    use qed_crypto::hash::{
        merkle::utils::simple_merkle_tree::SimpleMerkleTree, traits::hasher::PoseidonHasher,
    };

    use crate::{
        guta::circuits::verify_two_end_cap::GUTAVerifyTwoEndCapCircuit,
        ups::circuits::{
            end_cap::UPSStandardEndCapCircuit,
            ups_cfc_deferred_tx::UPSCFCDeferredTransactionCircuit,
            ups_cfc_standard::UPSCFCStandardTransactionCircuit, ups_start::UPSStartSessionCircuit,
        },
    };

    #[test]
    fn check_endcap_lookalike() {
        const D: usize = 2;
        type C = PoseidonGoldilocksConfig;
        type F = GoldilocksField;

        let ups_start = UPSStartSessionCircuit::<C, D>::new();
        let ups_cfc_standard_tx = UPSCFCStandardTransactionCircuit::<C, D>::new();
        let ups_cfc_deferred_tx = UPSCFCDeferredTransactionCircuit::<C, D>::new();
        let network_magic = QED_NETWORK_MAGIC_REGTEST;
        let mut ups_circuit_whitelist_proofs =
            SimpleMerkleTree::<PoseidonHasher, QHashOut<F>>::gen_fast_tree_inclusion_proofs(
                UPS_CIRCUIT_WHITELIST_TREE_HEIGHT,
                &[
                    ups_start.get_fingerprint(),
                    ups_cfc_standard_tx.get_fingerprint(),
                    ups_cfc_deferred_tx.get_fingerprint(),
                ],
            )
            .unwrap();
        let _ups_cfc_deferred_tx_whitelist_proof = ups_circuit_whitelist_proofs.pop().unwrap();
        let ups_cfc_standard_tx_whitelist_proof = ups_circuit_whitelist_proofs.pop().unwrap();
        let _ups_start_whitelist_proof = ups_circuit_whitelist_proofs.pop().unwrap();

        let ups_circuit_whitelist_root = ups_cfc_standard_tx_whitelist_proof.root;

        let proof_tree_agg_circuits = PortableQTreeRecursionCircuits::<C, D>::new(
            UPS_SESSION_PROOF_TREE_HEIGHT as usize,
            1,
            ups_cfc_deferred_tx
                .circuit_data
                .verifier_only
                .constants_sigmas_cap
                .height(),
            &ups_cfc_deferred_tx.circuit_data.common,
        );
        let ups_end_cap = UPSStandardEndCapCircuit::<C, D>::new_with_minifier(
            &proof_tree_agg_circuits
                .circuit_set
                .two_agg_circuit
                .circuit_data
                .common,
            proof_tree_agg_circuits
                .circuit_set
                .two_agg_circuit
                .get_verifier_config_ref()
                .constants_sigmas_cap
                .height(),
            network_magic,
            ups_circuit_whitelist_root,
            proof_tree_agg_circuits
                .circuit_inclusion_proofs
                .circuit_whitelist_tree_root,
        );

        let fake_end_cap = get_end_cap_type_e_lookalike_circuit_data::<C, D>();
        assert_eq!(
            fake_end_cap.common,
            ups_end_cap.get_common_circuit_data_ref().to_owned(),
            "end cap lookalike's common data does not match the real end cap common data",
        );
    }
    #[test]
    fn check_guta_lookalike() {
        const D: usize = 2;
        type C = PoseidonGoldilocksConfig;

        let end_cap_lookalike = get_end_cap_type_e_lookalike_circuit_data::<C, D>();
        let two_guta = GUTAVerifyTwoEndCapCircuit::<C, D>::new(
            &end_cap_lookalike.common,
            end_cap_lookalike
                .verifier_only
                .constants_sigmas_cap
                .height(),
            QHashOut::rand(),
        );
        let guta_lookalike = get_guta_type_c_lookalike_circuit_data::<C, D>();

        assert_eq!(
            two_guta.get_common_circuit_data_ref().to_owned(),
            guta_lookalike.common,
            "GUTA lookalike's common data does not match the real GUTA common data",
        );
    }

    #[test]
    fn check_agg_state_transition_lookalike() {
        const D: usize = 2;
        type C = PoseidonGoldilocksConfig;

        let agg_state_transition_lookalike =
            get_agg_state_transition_type_d_lookalike_circuit_data::<C, D>();
        let agg_state_dummy = AggStateTransitionDummyCircuit::<C, D>::new();
        let agg_state = AggStateTransitionCircuit::<C, D>::new(
            &agg_state_dummy.circuit_data.common,
            agg_state_dummy
                .get_verifier_config_ref()
                .constants_sigmas_cap
                .height(),
        );

        assert_eq!(
            agg_state_transition_lookalike.common,
            agg_state_dummy.get_common_circuit_data_ref().to_owned(),
            "Agg State Transition lookalike's common data does not match the real Dummy Agg State Transition common data",
        );

        assert_eq!(
            agg_state_transition_lookalike.common,
            agg_state.get_common_circuit_data_ref().to_owned(),
            "Agg State Transition lookalike's common data does not match the real Agg State Transition common data",
        );
    }
}
