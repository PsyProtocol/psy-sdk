pub mod builder;
pub mod commitment;
pub mod fft;
pub mod proof;
pub mod setup;
pub mod verifier;

pub use builder::{CircuitBuilderKZG, CircuitBuilderKZGHelpers};
pub use commitment::{KZGCommitment, KZGCommitmentTarget};
pub use fft::{CircuitBuilderFFT, FFTSettingsTarget};
pub use proof::{KZGProof, KZGProofTarget};
pub use setup::{KZGParams, KZGSetup};

#[cfg(test)]
mod tests {
    use super::*;
    use builder::CircuitBuilderKZGHelpers;
    use crate::crypto::bn254::{
        curve::{g1::G1, g2::G2},
        field::{bn128_base::Bn128Base, bn128_scalar::Bn128Scalar},
        gadgets::{
            g1::CircuitBuilderG1,
            g2::CircuitBuilderG2,
            nonnative_fp::CircuitBuilderNonNative,
            pairing::{AffinePointTargetG2, CircuitBuilderCurveG2, CircuitBuilderPairing},
        },
    };
    use crate::crypto::secp256k1::ecdsa::gadgets::biguint::{BigUintTarget, CircuitBuilderBiguint};

    use num::{BigUint, Zero};
    use plonky2::{
        field::types::Field,
        hash::hash_types::RichField,
        iop::witness::{PartialWitness, WitnessWrite},
        plonk::{
            circuit_builder::CircuitBuilder,
            circuit_data::CircuitConfig,
            config::{GenericConfig, PoseidonGoldilocksConfig},
        },
    };

    const D: usize = 2;
    type C = PoseidonGoldilocksConfig;
    type F = <C as GenericConfig<D>>::F;

    fn get_test_config() -> CircuitConfig {
        crate::crypto::bn254::pairing_config()
    }

    mod nonnative_tests {
        use super::*;

        #[test]
        fn test_nonnative_add_basic() {
            let config = get_test_config();
            let mut builder = CircuitBuilder::<F, D>::new(config);

            let x_ff = Bn128Base::from_canonical_u64(123);
            let y_ff = Bn128Base::from_canonical_u64(456);
            let sum_ff = x_ff + y_ff;

            let x = builder.constant_nonnative(x_ff);
            let y = builder.constant_nonnative(y_ff);
            let sum = builder.add_nonnative(&x, &y);
            let sum_expected = builder.constant_nonnative(sum_ff);
            builder.connect_nonnative(&sum, &sum_expected);

            let data = builder.build::<C>();
            let pw = PartialWitness::new();
            let proof = data.prove(pw).unwrap();
            data.verify(proof).unwrap();
        }

        #[test]
        fn test_nonnative_mul_basic() {
            let config = get_test_config();
            let mut builder = CircuitBuilder::<F, D>::new(config);

            let x_ff = Bn128Base::from_canonical_u64(12);
            let y_ff = Bn128Base::from_canonical_u64(34);
            let prod_ff = x_ff * y_ff;

            let x = builder.constant_nonnative(x_ff);
            let y = builder.constant_nonnative(y_ff);
            let prod = builder.mul_nonnative(&x, &y);
            let prod_expected = builder.constant_nonnative(prod_ff);
            builder.connect_nonnative(&prod, &prod_expected);

            let data = builder.build::<C>();
            let pw = PartialWitness::new();
            let proof = data.prove(pw).unwrap();
            data.verify(proof).unwrap();
        }

        #[test]
        fn test_nonnative_sub_basic() {
            let config = get_test_config();
            let mut builder = CircuitBuilder::<F, D>::new(config);

            let x_ff = Bn128Base::from_canonical_u64(456);
            let y_ff = Bn128Base::from_canonical_u64(123);
            let diff_ff = x_ff - y_ff;

            let x = builder.constant_nonnative(x_ff);
            let y = builder.constant_nonnative(y_ff);
            let diff = builder.sub_nonnative(&x, &y);
            let diff_expected = builder.constant_nonnative(diff_ff);
            builder.connect_nonnative(&diff, &diff_expected);

            let data = builder.build::<C>();
            let pw = PartialWitness::new();
            let proof = data.prove(pw).unwrap();
            data.verify(proof).unwrap();
        }
    }

    mod kzg_core_tests {
        use super::*;

        #[test]
        fn test_kzg_commit_simple() {
            let config = get_test_config();
            let mut builder = CircuitBuilder::<F, D>::new(config);

            let coeffs = vec![builder.constant_nonnative(Bn128Scalar::from_canonical_u64(5))];

            let g1_gen = builder.g1_generator();
            let powers_of_tau = vec![g1_gen.clone()];

            let commitment = builder.kzg_commit(&coeffs, &powers_of_tau);

            let data = builder.build::<C>();
            let pw = PartialWitness::new();
            let proof = data.prove(pw).unwrap();
            data.verify(proof).unwrap();
        }

        #[test]
        fn test_kzg_proof_creation() {
            let config = get_test_config();
            let mut builder = CircuitBuilder::<F, D>::new(config);

            let coeffs = vec![builder.constant_nonnative(Bn128Scalar::from_canonical_u64(5))];

            let g1_gen = builder.g1_generator();
            let powers_of_tau = vec![g1_gen.clone()];

            let point = builder.constant_nonnative(Bn128Scalar::from_canonical_u64(3));
            let (evaluation, proof) =
                builder.kzg_create_opening_proof(&coeffs, &point, &powers_of_tau);

            let expected_eval = builder.constant_nonnative(Bn128Scalar::from_canonical_u64(5));
            builder.connect_nonnative(&evaluation, &expected_eval);

            let data = builder.build::<C>();
            let pw = PartialWitness::new();
            let proof = data.prove(pw).unwrap();
            data.verify(proof).unwrap();
        }

        #[test]
        fn test_polynomial_evaluation() {
            let config = get_test_config();
            let mut builder = CircuitBuilder::<F, D>::new(config);

            // Test case 1: p(x) = 1 + 2x + 3x² at x = 2
            let coeffs = vec![
                builder.constant_nonnative(Bn128Scalar::from_canonical_u64(1)),
                builder.constant_nonnative(Bn128Scalar::from_canonical_u64(2)),
                builder.constant_nonnative(Bn128Scalar::from_canonical_u64(3)),
            ];

            let point = builder.constant_nonnative(Bn128Scalar::from_canonical_u64(2));
            let result = builder.kzg_evaluate_polynomial(&coeffs, &point);

            // p(2) = 1 + 2*2 + 3*4 = 1 + 4 + 12 = 17
            let expected = builder.constant_nonnative(Bn128Scalar::from_canonical_u64(17));
            builder.connect_nonnative(&result, &expected);

            let data = builder.build::<C>();
            let pw = PartialWitness::new();
            let proof = data.prove(pw).unwrap();
            data.verify(proof).unwrap();
        }

        #[test]
        fn test_polynomial_evaluation_comprehensive() {
            let config = get_test_config();
            let mut builder = CircuitBuilder::<F, D>::new(config);

            // Test case 1: Constant polynomial p(x) = 42
            let coeffs1 = vec![builder.constant_nonnative(Bn128Scalar::from_canonical_u64(42))];
            let point1 = builder.constant_nonnative(Bn128Scalar::from_canonical_u64(100));
            let result1 = builder.kzg_evaluate_polynomial(&coeffs1, &point1);
            let expected1 = builder.constant_nonnative(Bn128Scalar::from_canonical_u64(42));
            builder.connect_nonnative(&result1, &expected1);

            // Test case 2: Linear polynomial p(x) = 5 + 3x at x = 7
            let coeffs2 = vec![
                builder.constant_nonnative(Bn128Scalar::from_canonical_u64(5)),
                builder.constant_nonnative(Bn128Scalar::from_canonical_u64(3)),
            ];
            let point2 = builder.constant_nonnative(Bn128Scalar::from_canonical_u64(7));
            let result2 = builder.kzg_evaluate_polynomial(&coeffs2, &point2);
            // p(7) = 5 + 3*7 = 5 + 21 = 26
            let expected2 = builder.constant_nonnative(Bn128Scalar::from_canonical_u64(26));
            builder.connect_nonnative(&result2, &expected2);

            // Test case 3: Zero polynomial at x = 0
            let coeffs3 = vec![builder.constant_nonnative(Bn128Scalar::ZERO)];
            let point3 = builder.constant_nonnative(Bn128Scalar::ZERO);
            let result3 = builder.kzg_evaluate_polynomial(&coeffs3, &point3);
            let expected3 = builder.constant_nonnative(Bn128Scalar::ZERO);
            builder.connect_nonnative(&result3, &expected3);

            // Test case 4: Higher degree p(x) = 1 + x + x² + x³ + x⁴ at x = 2
            let coeffs4 = vec![
                builder.constant_nonnative(Bn128Scalar::from_canonical_u64(1)),
                builder.constant_nonnative(Bn128Scalar::from_canonical_u64(1)),
                builder.constant_nonnative(Bn128Scalar::from_canonical_u64(1)),
                builder.constant_nonnative(Bn128Scalar::from_canonical_u64(1)),
                builder.constant_nonnative(Bn128Scalar::from_canonical_u64(1)),
            ];
            let point4 = builder.constant_nonnative(Bn128Scalar::from_canonical_u64(2));
            let result4 = builder.kzg_evaluate_polynomial(&coeffs4, &point4);
            // p(2) = 1 + 2 + 4 + 8 + 16 = 31
            let expected4 = builder.constant_nonnative(Bn128Scalar::from_canonical_u64(31));
            builder.connect_nonnative(&result4, &expected4);

            // Test case 5: Empty polynomial (should return 0)
            let coeffs5 = vec![];
            let point5 = builder.constant_nonnative(Bn128Scalar::from_canonical_u64(10));
            let result5 = builder.kzg_evaluate_polynomial(&coeffs5, &point5);
            let expected5 = builder.constant_nonnative(Bn128Scalar::ZERO);
            builder.connect_nonnative(&result5, &expected5);

            let data = builder.build::<C>();
            let pw = PartialWitness::new();
            let proof = data.prove(pw).unwrap();
            data.verify(proof).unwrap();

            println!("✅ All polynomial evaluation test cases passed!");
        }

        #[test]
        fn test_quotient_polynomial() {
            let config = get_test_config();
            let mut builder = CircuitBuilder::<F, D>::new(config);

            let coeffs = vec![
                builder.constant_nonnative(Bn128Scalar::from_canonical_u64(1)),
                builder.constant_nonnative(Bn128Scalar::from_canonical_u64(1)),
            ];

            let point = builder.constant_nonnative(Bn128Scalar::from_canonical_u64(2));
            let evaluation = builder.constant_nonnative(Bn128Scalar::from_canonical_u64(3));

            let quotient = builder.kzg_compute_quotient_polynomial(&coeffs, &point, &evaluation);

            assert_eq!(quotient.len(), 1);
            let expected_q0 = builder.constant_nonnative(Bn128Scalar::from_canonical_u64(1));
            builder.connect_nonnative(&quotient[0], &expected_q0);

            let data = builder.build::<C>();
            let pw = PartialWitness::new();
            let proof = data.prove(pw).unwrap();
            data.verify(proof).unwrap();
        }
    }

    mod zero_polynomial_tests {
        use super::*;

        #[test]
        fn test_zero_polynomial_commit() {
            let config = get_test_config();
            let mut builder = CircuitBuilder::<F, D>::new(config);

            let coeffs = vec![builder.constant_nonnative(Bn128Scalar::ZERO)];

            let g1_gen = builder.g1_generator();
            let powers_of_tau = vec![g1_gen.clone()];

            let commitment = builder.kzg_commit(&coeffs, &powers_of_tau);

            let data = builder.build::<C>();
            let pw = PartialWitness::new();
            let proof = data.prove(pw).unwrap();
            data.verify(proof).unwrap();
        }

        #[test]
        fn test_zero_polynomial_eval() {
            let config = get_test_config();
            let mut builder = CircuitBuilder::<F, D>::new(config);

            let coeffs = vec![builder.constant_nonnative(Bn128Scalar::ZERO)];

            let point = builder.constant_nonnative(Bn128Scalar::from_canonical_u64(5));
            let result = builder.kzg_evaluate_polynomial(&coeffs, &point);

            let expected = builder.constant_nonnative(Bn128Scalar::ZERO);
            builder.connect_nonnative(&result, &expected);

            let data = builder.build::<C>();
            let pw = PartialWitness::new();
            let proof = data.prove(pw).unwrap();
            data.verify(proof).unwrap();
        }
    }

    mod lagrange_tests {
        use super::*;

        #[test]
        fn test_simple_lagrange_setup() {
            let tau = Bn128Scalar::from_canonical_u64(12345);
            let domain_size = 4;

            let monomial_params = KZGSetup::new_trusted_setup(tau, domain_size);
            let lagrange_params = KZGSetup::new_lagrange_setup(tau, domain_size);

            assert!(lagrange_params.lagrange_g1.is_some());
            assert!(lagrange_params.roots_of_unity.is_some());
            assert!(lagrange_params.is_lagrange_form);
        }

        #[test]
        fn test_lagrange_commitment_simple() {
            let config = CircuitConfig {
                num_wires: 400,
                ..CircuitConfig::wide_ecc_config()
            };
            let mut builder = CircuitBuilder::<F, D>::new(config);

            let tau = Bn128Scalar::from_canonical_u64(12345);
            let domain_size = 4;
            let params = KZGSetup::new_lagrange_setup(tau, domain_size);

            let evaluations = vec![
                builder.constant_nonnative(Bn128Scalar::from_canonical_u64(1)),
                builder.constant_nonnative(Bn128Scalar::from_canonical_u64(2)),
                builder.constant_nonnative(Bn128Scalar::from_canonical_u64(3)),
                builder.constant_nonnative(Bn128Scalar::from_canonical_u64(4)),
            ];

            let lagrange_g1 = params
                .lagrange_g1
                .unwrap()
                .into_iter()
                .map(|p| builder.constant_g1_affine(p))
                .collect::<Vec<_>>();

            let commitment = builder.kzg_commit_lagrange(&evaluations, &lagrange_g1);

            let data = builder.build::<C>();
            let pw = PartialWitness::new();
            let proof = data.prove(pw).unwrap();
            data.verify(proof).unwrap();
        }
    }

    mod setup_tests {
        use super::*;

        #[test]
        fn test_trusted_setup() {
            let tau = Bn128Scalar::from_canonical_u64(123);
            let params = KZGSetup::new_trusted_setup(tau, 8);

            assert_eq!(params.powers_of_tau_g1.len(), 8);
            assert_eq!(params.powers_of_tau_g2.len(), 2);
            assert!(KZGSetup::verify_setup(&params));
        }

        #[test]
        fn test_powers_of_tau_consistency() {
            let tau = Bn128Scalar::from_canonical_u64(7);
            let params = KZGSetup::new_trusted_setup(tau, 4);

            use crate::crypto::bn254::curve::g1::G1;
            use crate::crypto::secp256k1::ecdsa::curve::curve_types::Curve;
            let g1_gen = G1::GENERATOR_AFFINE;

            assert_eq!(params.powers_of_tau_g1[0], g1_gen);
        }
    }

    mod fft_tests {
        use super::*;

        #[test]
        fn test_fft_roundtrip() {
            let config = CircuitConfig {
                num_wires: 400,
                ..CircuitConfig::wide_ecc_config()
            };
            let mut builder = CircuitBuilder::<F, D>::new(config);

            let domain_size = 4;
            let settings = builder.fft_settings(domain_size);

            let coeffs = vec![
                builder.constant_nonnative(Bn128Scalar::from_canonical_u64(1)),
                builder.constant_nonnative(Bn128Scalar::from_canonical_u64(2)),
                builder.constant_nonnative(Bn128Scalar::from_canonical_u64(3)),
                builder.constant_nonnative(Bn128Scalar::from_canonical_u64(4)),
            ];

            let evals = builder.fft_forward(&coeffs, &settings);
            let recovered = builder.fft_inverse(&evals, &settings);

            for i in 0..domain_size {
                builder.connect_nonnative(&coeffs[i], &recovered[i]);
            }

            let data = builder.build::<C>();
            let pw = PartialWitness::new();
            let proof = data.prove(pw).unwrap();
            data.verify(proof).unwrap();
        }
    }

    mod debug_tests {
        use crate::crypto::bn254::pairing_config;

        use super::*;

        #[test]
        fn test_debug_kzg_commit_only() {
            let mut builder = CircuitBuilder::<F, D>::new(pairing_config());

            let params = KZGSetup::new_test_setup(3);
            let g1_powers = params.get_g1_powers(3).unwrap();
            let powers_of_tau = g1_powers
                .iter()
                .map(|p| builder.constant_g1_affine(*p))
                .collect::<Vec<_>>();

            let coeffs = vec![
                builder.constant_nonnative(Bn128Scalar::from_canonical_u64(1)),
                builder.constant_nonnative(Bn128Scalar::from_canonical_u64(1)),
            ];

            let commitment = builder.kzg_commit(&coeffs, &powers_of_tau[..2]);

            let data = builder.build::<C>();
            let pw = PartialWitness::new();
            let proof = data.prove(pw).unwrap();
            data.verify(proof).unwrap();
        }

        #[test]
        fn test_debug_simple_nonnative_arithmetic() {
            let mut builder = CircuitBuilder::<F, D>::new(pairing_config());

            let one = builder.constant_nonnative(Bn128Scalar::from_canonical_u64(1));
            let three = builder.constant_nonnative(Bn128Scalar::from_canonical_u64(3));
            let diff = builder.sub_nonnative(&one, &three);

            let neg_two = Bn128Scalar::ZERO - Bn128Scalar::from_canonical_u64(2);
            let expected = builder.constant_nonnative(neg_two);

            builder.connect_nonnative(&diff, &expected);

            let data = builder.build::<C>();
            let pw = PartialWitness::new();
            let proof = data.prove(pw).unwrap();
            data.verify(proof).unwrap();
        }
    }

    mod end_to_end_tests {
        use super::*;

        #[test]
        fn test_kzg_full_workflow() {
            let config = get_test_config();
            let mut builder = CircuitBuilder::<F, D>::new(config);

            // Setup parameters
            let tau = Bn128Scalar::from_canonical_u64(12345);
            let max_degree = 4;
            let setup_params = KZGSetup::new_trusted_setup(tau, max_degree);

            // Convert setup to circuit targets
            let powers_of_tau_g1: Vec<_> = setup_params.powers_of_tau_g1
                .iter()
                .map(|p| builder.constant_g1_affine(*p))
                .collect();
            
            let (g2_gen_point, g2_tau_point) = setup_params.get_g2_powers();
            let g2_tau = builder.constant_affine_point_g2::<G2, Bn128Base>(*g2_tau_point);

            // Create a polynomial: p(x) = 1 + 2x + 3x²
            let coeffs = vec![
                builder.constant_nonnative(Bn128Scalar::from_canonical_u64(1)),
                builder.constant_nonnative(Bn128Scalar::from_canonical_u64(2)),
                builder.constant_nonnative(Bn128Scalar::from_canonical_u64(3)),
            ];

            // Step 1: Commit to the polynomial
            let commitment = builder.kzg_commit(&coeffs, &powers_of_tau_g1[..3]);

            // Step 2: Create opening proof at point z = 5
            let point = builder.constant_nonnative(Bn128Scalar::from_canonical_u64(5));
            let (evaluation, proof) = builder.kzg_create_opening_proof(
                &coeffs, 
                &point, 
                &powers_of_tau_g1[..3]
            );

            // Verify the evaluation is correct: p(5) = 1 + 2*5 + 3*5² = 1 + 10 + 75 = 86
            let expected_eval = builder.constant_nonnative(Bn128Scalar::from_canonical_u64(86));
            builder.connect_nonnative(&evaluation, &expected_eval);

            // Step 3: Verify the proof
            let is_valid = builder.kzg_verify(
                &commitment,
                &point,
                &evaluation,
                &proof,
                &g2_tau
            );

            // Assert the proof is valid
            builder.assert_one(is_valid.target);

            // Build and prove the circuit
            let data = builder.build::<C>();
            let pw = PartialWitness::new();
            let proof = data.prove(pw).unwrap();
            data.verify(proof).unwrap();
        }

        #[test]
        fn test_kzg_multiple_openings() {
            let config = get_test_config();
            let mut builder = CircuitBuilder::<F, D>::new(config);

            // Setup
            let tau = Bn128Scalar::from_canonical_u64(54321);
            let setup_params = KZGSetup::new_trusted_setup(tau, 8);
            
            let powers_of_tau_g1: Vec<_> = setup_params.powers_of_tau_g1
                .iter()
                .map(|p| builder.constant_g1_affine(*p))
                .collect();
            
            let (_, g2_tau_point) = setup_params.get_g2_powers();
            let g2_tau = builder.constant_affine_point_g2::<G2, Bn128Base>(*g2_tau_point);

            // Polynomial: p(x) = 2 + 3x + x² + 4x³
            let coeffs = vec![
                builder.constant_nonnative(Bn128Scalar::from_canonical_u64(2)),
                builder.constant_nonnative(Bn128Scalar::from_canonical_u64(3)),
                builder.constant_nonnative(Bn128Scalar::from_canonical_u64(1)),
                builder.constant_nonnative(Bn128Scalar::from_canonical_u64(4)),
            ];

            // Commit
            let commitment = builder.kzg_commit(&coeffs, &powers_of_tau_g1[..4]);

            // Open at multiple points
            let points = vec![
                builder.constant_nonnative(Bn128Scalar::from_canonical_u64(0)), // p(0) = 2
                builder.constant_nonnative(Bn128Scalar::from_canonical_u64(1)), // p(1) = 10
                builder.constant_nonnative(Bn128Scalar::from_canonical_u64(2)), // p(2) = 40
            ];

            let expected_evals = vec![
                Bn128Scalar::from_canonical_u64(2),
                Bn128Scalar::from_canonical_u64(10),
                Bn128Scalar::from_canonical_u64(40),
            ];

            // Create and verify proofs for each point
            for (i, point) in points.iter().enumerate() {
                let (eval, proof) = builder.kzg_create_opening_proof(
                    &coeffs,
                    point,
                    &powers_of_tau_g1[..4]
                );

                // Check evaluation
                let expected = builder.constant_nonnative(expected_evals[i]);
                builder.connect_nonnative(&eval, &expected);

                // Verify proof
                let is_valid = builder.kzg_verify(
                    &commitment,
                    point,
                    &eval,
                    &proof,
                    &g2_tau
                );
                builder.assert_one(is_valid.target);
            }

            let data = builder.build::<C>();
            let pw = PartialWitness::new();
            let proof = data.prove(pw).unwrap();
            data.verify(proof).unwrap();
        }

        #[test]
        fn test_kzg_batch_verify() {
            let config = get_test_config();
            let mut builder = CircuitBuilder::<F, D>::new(config);

            // Setup
            let tau = Bn128Scalar::from_canonical_u64(98765);
            let setup_params = KZGSetup::new_trusted_setup(tau, 8);
            
            let powers_of_tau_g1: Vec<_> = setup_params.powers_of_tau_g1
                .iter()
                .map(|p| builder.constant_g1_affine(*p))
                .collect();
            
            let (_, g2_tau_point) = setup_params.get_g2_powers();
            let g2_tau = builder.constant_affine_point_g2::<G2, Bn128Base>(*g2_tau_point);

            // Create two polynomials
            let coeffs1 = vec![
                builder.constant_nonnative(Bn128Scalar::from_canonical_u64(1)),
                builder.constant_nonnative(Bn128Scalar::from_canonical_u64(2)),
            ];
            let coeffs2 = vec![
                builder.constant_nonnative(Bn128Scalar::from_canonical_u64(3)),
                builder.constant_nonnative(Bn128Scalar::from_canonical_u64(4)),
            ];

            // Commit to both
            let commitment1 = builder.kzg_commit(&coeffs1, &powers_of_tau_g1[..2]);
            let commitment2 = builder.kzg_commit(&coeffs2, &powers_of_tau_g1[..2]);

            // Create batch opening proofs
            let all_coeffs = vec![coeffs1, coeffs2];
            let points = vec![
                builder.constant_nonnative(Bn128Scalar::from_canonical_u64(2)),
                builder.constant_nonnative(Bn128Scalar::from_canonical_u64(3)),
            ];

            let (evaluations, proofs) = builder.kzg_create_batch_opening_proofs(
                &all_coeffs,
                &points,
                &powers_of_tau_g1[..2]
            );

            // Batch verify
            let commitments = vec![commitment1, commitment2];
            let is_valid = builder.kzg_batch_verify(
                &commitments,
                &points,
                &evaluations,
                &proofs,
                &g2_tau
            );

            builder.assert_one(is_valid.target);

            let data = builder.build::<C>();
            let pw = PartialWitness::new();
            let proof = data.prove(pw).unwrap();
            data.verify(proof).unwrap();
        }

        #[test]
        fn test_kzg_invalid_proof_rejection() {
            let config = get_test_config();
            let mut builder = CircuitBuilder::<F, D>::new(config);

            // Setup
            let tau = Bn128Scalar::from_canonical_u64(11111);
            let setup_params = KZGSetup::new_trusted_setup(tau, 4);
            
            let powers_of_tau_g1: Vec<_> = setup_params.powers_of_tau_g1
                .iter()
                .map(|p| builder.constant_g1_affine(*p))
                .collect();
            
            let (_, g2_tau_point) = setup_params.get_g2_powers();
            let g2_tau = builder.constant_affine_point_g2::<G2, Bn128Base>(*g2_tau_point);

            // Create polynomial
            let coeffs = vec![
                builder.constant_nonnative(Bn128Scalar::from_canonical_u64(5)),
                builder.constant_nonnative(Bn128Scalar::from_canonical_u64(7)),
            ];

            let commitment = builder.kzg_commit(&coeffs, &powers_of_tau_g1[..2]);
            let point = builder.constant_nonnative(Bn128Scalar::from_canonical_u64(3));
            let (correct_eval, correct_proof) = builder.kzg_create_opening_proof(
                &coeffs,
                &point,
                &powers_of_tau_g1[..2]
            );

            // Create an incorrect evaluation
            let wrong_eval = builder.constant_nonnative(Bn128Scalar::from_canonical_u64(999));

            // Verify with wrong evaluation should fail
            let is_valid = builder.kzg_verify(
                &commitment,
                &point,
                &wrong_eval,  // Wrong evaluation
                &correct_proof,
                &g2_tau
            );

            // Assert the proof is INVALID
            builder.assert_zero(is_valid.target);

            let data = builder.build::<C>();
            let pw = PartialWitness::new();
            let proof = data.prove(pw).unwrap();
            data.verify(proof).unwrap();
        }

        #[test]
        fn test_kzg_zero_polynomial() {
            let config = get_test_config();
            let mut builder = CircuitBuilder::<F, D>::new(config);

            // Setup
            let tau = Bn128Scalar::from_canonical_u64(22222);
            let setup_params = KZGSetup::new_trusted_setup(tau, 4);
            
            let powers_of_tau_g1: Vec<_> = setup_params.powers_of_tau_g1
                .iter()
                .map(|p| builder.constant_g1_affine(*p))
                .collect();
            
            let (_, g2_tau_point) = setup_params.get_g2_powers();
            let g2_tau = builder.constant_affine_point_g2::<G2, Bn128Base>(*g2_tau_point);

            // Zero polynomial
            let coeffs = vec![
                builder.constant_nonnative(Bn128Scalar::ZERO),
                builder.constant_nonnative(Bn128Scalar::ZERO),
                builder.constant_nonnative(Bn128Scalar::ZERO),
            ];

            let commitment = builder.kzg_commit(&coeffs, &powers_of_tau_g1[..3]);
            let point = builder.constant_nonnative(Bn128Scalar::from_canonical_u64(7));
            let (eval, proof) = builder.kzg_create_opening_proof(
                &coeffs,
                &point,
                &powers_of_tau_g1[..3]
            );

            // Evaluation should be zero
            let zero = builder.constant_nonnative(Bn128Scalar::ZERO);
            builder.connect_nonnative(&eval, &zero);

            // Verify proof
            let is_valid = builder.kzg_verify(
                &commitment,
                &point,
                &eval,
                &proof,
                &g2_tau
            );
            builder.assert_one(is_valid.target);

            let data = builder.build::<C>();
            let pw = PartialWitness::new();
            let proof = data.prove(pw).unwrap();
            data.verify(proof).unwrap();
        }
    }
}
