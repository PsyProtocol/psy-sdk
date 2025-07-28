pub mod builder;
pub mod commitment;
pub mod fft;
pub mod proof;
pub mod setup;
pub mod verifier;

pub use builder::CircuitBuilderKZG;
pub use commitment::{KZGCommitment, KZGCommitmentTarget};
pub use fft::{CircuitBuilderFFT, FFTSettingsTarget};
pub use proof::{KZGProof, KZGProofTarget};
pub use setup::{KZGParams, KZGSetup};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::bn254::{
        curve::{g1::G1, g2::G2},
        field::{bn128_scalar::Bn128Scalar, bn128_base::Bn128Base},
        gadgets::{
            g1::CircuitBuilderG1,
            g2::CircuitBuilderG2,
            nonnative_fp::CircuitBuilderNonNative,
            pairing::{CircuitBuilderPairing, CircuitBuilderCurveG2, AffinePointTargetG2},
            biguint::{CircuitBuilderBiguint, BigUintTarget},
        },
    };
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
    use num::{BigUint, Zero};

    const D: usize = 2;
    type C = PoseidonGoldilocksConfig;
    type F = <C as GenericConfig<D>>::F;

    fn get_test_config() -> CircuitConfig {
        CircuitConfig {
            num_wires: 400,
            ..CircuitConfig::wide_ecc_config()
        }
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

            let coeffs = vec![
                builder.constant_nonnative(Bn128Scalar::from_canonical_u64(5)),
            ];

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

            let coeffs = vec![
                builder.constant_nonnative(Bn128Scalar::from_canonical_u64(5)),
            ];

            let g1_gen = builder.g1_generator();
            let powers_of_tau = vec![g1_gen.clone()];

            let point = builder.constant_nonnative(Bn128Scalar::from_canonical_u64(3));
            let (evaluation, proof) = builder.kzg_create_opening_proof(&coeffs, &point, &powers_of_tau);

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

            let coeffs = vec![
                builder.constant_nonnative(Bn128Scalar::from_canonical_u64(1)),
                builder.constant_nonnative(Bn128Scalar::from_canonical_u64(2)),
                builder.constant_nonnative(Bn128Scalar::from_canonical_u64(3)),
            ];

            let point = builder.constant_nonnative(Bn128Scalar::from_canonical_u64(2));
            let result = builder.kzg_evaluate_polynomial(&coeffs, &point);

            let expected = builder.constant_nonnative(Bn128Scalar::from_canonical_u64(17));
            builder.connect_nonnative(&result, &expected);

            let data = builder.build::<C>();
            let pw = PartialWitness::new();
            let proof = data.prove(pw).unwrap();
            data.verify(proof).unwrap();
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

            let coeffs = vec![
                builder.constant_nonnative(Bn128Scalar::ZERO),
            ];

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

            let coeffs = vec![
                builder.constant_nonnative(Bn128Scalar::ZERO),
            ];

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

            let lagrange_g1 = params.lagrange_g1.unwrap()
                .into_iter()
                .map(|p| builder.constant_g1_affine(p))
                .collect::<Vec<_>>();

            let commitment = builder.kzg_commit_lagrange_basis(&evaluations, &lagrange_g1);

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
        use super::*;

        fn get_debug_config() -> CircuitConfig {
            CircuitConfig {
                num_wires: 400,
                num_routed_wires: 80,
                num_constants: 2,
                use_base_arithmetic_gate: true,
                security_bits: 100,
                num_challenges: 2,
                zero_knowledge: false,
                max_quotient_degree_factor: 8,
                fri_config: plonky2::fri::FriConfig {
                    rate_bits: 3,
                    cap_height: 4,
                    proof_of_work_bits: 16,
                    reduction_strategy: plonky2::fri::reduction_strategies::FriReductionStrategy::ConstantArityBits(4, 5),
                    num_query_rounds: 28,
                },
            }
        }

        #[test]
        fn test_debug_kzg_commit_only() {
            let mut builder = CircuitBuilder::<F, D>::new(get_debug_config());

            let params = KZGSetup::new_test_setup(3);
            let g1_powers = params.get_g1_powers(3).unwrap();
            let powers_of_tau = g1_powers.iter()
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
            let mut builder = CircuitBuilder::<F, D>::new(get_debug_config());

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
}
