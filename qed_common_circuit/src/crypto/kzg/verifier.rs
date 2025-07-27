/// KZG Verifier implementation using our BN254 pairing
use plonky2::{
    field::{
        extension::Extendable,
        types::{Field, PrimeField},
    },
    hash::hash_types::RichField,
    iop::target::{BoolTarget, Target},
    plonk::circuit_builder::CircuitBuilder,
};

use crate::crypto::bn254::{
    curve::g2::G2,
    field::{
        bn128_base::Bn128Base,
        bn128_scalar::Bn128Scalar,
        extension::quadratic::QuadraticExtension,
    },
    gadgets::{
        g1::{CircuitBuilderG1, G1AffineTarget},
        g2::{CircuitBuilderG2, G2AffineTarget},
        pairing::{CircuitBuilderPairing, CircuitBuilderCurveG2, AffinePointTargetG2},
        nonnative_fp::{CircuitBuilderNonNative, NonNativeTarget},
        nonnative_fp2::CircuitBuilderNonNativeExt2,
        nonnative_fp6::{CircuitBuilderNonNativeExt6, NonNativeTargetExt6},
        nonnative_fp12::{CircuitBuilderNonNativeExt12, NonNativeTargetExt12},
    },
};

use super::{
    commitment::KZGCommitmentTarget,
    proof::KZGProofTarget,
};

/// KZG Verifier functionality
pub trait KZGVerifier<F: RichField + Extendable<D>, const D: usize> {
    /// Verify a KZG proof using pairing check:
    /// e(C - y*g, h) = e(W, h^tau - z*h)
    /// where:
    /// - C is the commitment
    /// - y is the evaluation at point z
    /// - W is the witness (proof)
    /// - h^tau is from trusted setup
    fn kzg_verify(
        &mut self,
        commitment: &KZGCommitmentTarget<F, D>,
        point: &NonNativeTarget<Bn128Scalar>,
        evaluation: &NonNativeTarget<Bn128Scalar>,
        proof: &KZGProofTarget<F, D>,
        g2_tau: &AffinePointTargetG2<Bn128Base>,
    ) -> BoolTarget;
    
    /// Batch verify multiple KZG proofs
    fn kzg_batch_verify(
        &mut self,
        commitments: &[KZGCommitmentTarget<F, D>],
        points: &[NonNativeTarget<Bn128Scalar>],
        evaluations: &[NonNativeTarget<Bn128Scalar>],
        proofs: &[KZGProofTarget<F, D>],
        g2_tau: &AffinePointTargetG2<Bn128Base>,
    ) -> BoolTarget;
}

impl<F: RichField + Extendable<D>, const D: usize> KZGVerifier<F, D> for CircuitBuilder<F, D> {
    fn kzg_verify(
        &mut self,
        commitment: &KZGCommitmentTarget<F, D>,
        point: &NonNativeTarget<Bn128Scalar>,
        evaluation: &NonNativeTarget<Bn128Scalar>,
        proof: &KZGProofTarget<F, D>,
        g2_tau: &AffinePointTargetG2<Bn128Base>,
    ) -> BoolTarget {
        // Get generators
        let g1_gen = self.g1_generator();
        let g2_gen = self.g2_generator();
        
        // Compute C - y*g (left side of pairing equation)
        let y_g = self.scalar_mul_g1(&g1_gen, evaluation);
        let neg_y_g = self.neg_g1_affine(&y_g);
        let left_g1 = self.add_g1_affine(&commitment.commitment, &neg_y_g);
        
        // Compute h^tau - z*h (right side of pairing equation)
        let z_h = self.scalar_mul_g2(&g2_gen, point);
        let neg_z_h = crate::crypto::bn254::gadgets::pairing::CircuitBuilderCurveG2::neg_g2(self, &z_h);
        let right_g2 = crate::crypto::bn254::gadgets::pairing::CircuitBuilderCurveG2::add_g2::<G2, Bn128Base>(self, g2_tau, &neg_z_h);
        
        // Compute pairings
        let left_pairing = self.pairing_bn254(&left_g1, &g2_gen);
        let right_pairing = self.pairing_bn254(&proof.w, &right_g2);
        
        // Check equality of pairings
        self.is_equal_ext12(&left_pairing, &right_pairing)
    }
    
    fn kzg_batch_verify(
        &mut self,
        commitments: &[KZGCommitmentTarget<F, D>],
        points: &[NonNativeTarget<Bn128Scalar>],
        evaluations: &[NonNativeTarget<Bn128Scalar>],
        proofs: &[KZGProofTarget<F, D>],
        g2_tau: &AffinePointTargetG2<Bn128Base>,
    ) -> BoolTarget {
        assert_eq!(commitments.len(), points.len());
        assert_eq!(commitments.len(), evaluations.len());
        assert_eq!(commitments.len(), proofs.len());
        
        if commitments.is_empty() {
            return self._true();
        }
        
        // For single proof, use regular verify
        if commitments.len() == 1 {
            return self.kzg_verify(
                &commitments[0],
                &points[0],
                &evaluations[0],
                &proofs[0],
                g2_tau,
            );
        }
        
        // Generate random challenges for batching
        let mut challenges = Vec::new();
        for i in 0..commitments.len() {
            // Use deterministic "randomness" based on index for simplicity
            let challenge = self.constant_nonnative(Bn128Scalar::from_canonical_u64((i + 1) as u64 * 12345));
            challenges.push(challenge);
        }
        
        // Accumulate left side: sum(r_i * (C_i - y_i*g))
        let g1_gen = self.g1_generator();
        let identity = self.constant_g1_affine(crate::crypto::secp256k1::ecdsa::curve::curve_types::AffinePoint::INFINITY);
        let mut left_acc = identity.clone();
        
        for i in 0..commitments.len() {
            let y_g = self.scalar_mul_g1(&g1_gen, &evaluations[i]);
            let neg_y_g = self.neg_g1_affine(&y_g);
            let c_minus_yg = self.add_g1_affine(&commitments[i].commitment, &neg_y_g);
            let scaled = self.scalar_mul_g1(&c_minus_yg, &challenges[i]);
            left_acc = self.add_g1_affine(&left_acc, &scaled);
        }
        
        // Accumulate right side: sum(r_i * W_i)
        let mut right_acc = identity;
        for i in 0..proofs.len() {
            let scaled = self.scalar_mul_g1(&proofs[i].w, &challenges[i]);
            right_acc = self.add_g1_affine(&right_acc, &scaled);
        }
        
        // Compute G2 side: sum(r_i * z_i) * h
        let g2_gen = self.g2_generator();
        let mut z_acc = self.constant_nonnative(Bn128Scalar::ZERO);
        for i in 0..points.len() {
            let r_z = self.mul_nonnative(&challenges[i], &points[i]);
            z_acc = self.add_nonnative(&z_acc, &r_z);
        }
        let z_h = self.scalar_mul_g2(&g2_gen, &z_acc);
        let neg_z_h = crate::crypto::bn254::gadgets::pairing::CircuitBuilderCurveG2::neg_g2(self, &z_h);
        let right_g2 = crate::crypto::bn254::gadgets::pairing::CircuitBuilderCurveG2::add_g2::<G2, Bn128Base>(self, g2_tau, &neg_z_h);
        
        // Verify pairing equation
        let left_pairing = self.pairing_bn254(&left_acc, &g2_gen);
        let right_pairing = self.pairing_bn254(&right_acc, &right_g2);
        
        self.is_equal_ext12(&left_pairing, &right_pairing)
    }
}

/// Helper trait for G2 operations and pairing equality
trait KZGVerifierHelpers<F: RichField + Extendable<D>, const D: usize> {
    fn g2_generator(&mut self) -> AffinePointTargetG2<Bn128Base>;
    fn scalar_mul_g2(
        &mut self,
        point: &AffinePointTargetG2<Bn128Base>,
        scalar: &NonNativeTarget<Bn128Scalar>,
    ) -> AffinePointTargetG2<Bn128Base>;
    fn is_equal_ext12(
        &mut self,
        a: &NonNativeTargetExt12<Bn128Base>,
        b: &NonNativeTargetExt12<Bn128Base>,
    ) -> BoolTarget;
}

impl<F: RichField + Extendable<D>, const D: usize> KZGVerifierHelpers<F, D> for CircuitBuilder<F, D> {
    fn g2_generator(&mut self) -> AffinePointTargetG2<Bn128Base> {
        use crate::crypto::bn254::curve::g2::G2;
        use crate::crypto::secp256k1::ecdsa::curve::curve_types::Curve;
        
        let g2_gen = G2::GENERATOR_AFFINE;
        self.constant_affine_point_g2::<G2, Bn128Base>(g2_gen)
    }
    
    fn scalar_mul_g2(
        &mut self,
        point: &AffinePointTargetG2<Bn128Base>,
        scalar: &NonNativeTarget<Bn128Scalar>,
    ) -> AffinePointTargetG2<Bn128Base> {
        // TODO: Implement proper scalar multiplication for G2
        // For now, return the point itself as a placeholder
        point.clone()
    }
    
    fn is_equal_ext12(
        &mut self,
        a: &NonNativeTargetExt12<Bn128Base>,
        b: &NonNativeTargetExt12<Bn128Base>,
    ) -> BoolTarget {
        // Compare all 12 components of Fp12 elements
        let c0_eq = self.is_equal_ext6(&a.c0, &b.c0);
        let c1_eq = self.is_equal_ext6(&a.c1, &b.c1);
        self.and(c0_eq, c1_eq)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::kzg::{
        commitment::CircuitBuilderKZG,
        proof::CircuitBuilderKZGProof,
        setup::KZGSetup,
    };
    use plonky2::{
        iop::witness::PartialWitness,
        plonk::{
            circuit_data::CircuitConfig,
            config::{GenericConfig, PoseidonGoldilocksConfig},
        },
    };

    const D: usize = 2;
    type C = PoseidonGoldilocksConfig;
    type F = <C as GenericConfig<D>>::F;

    #[test]
    fn test_kzg_full_flow() {
        let config = CircuitConfig {
            num_wires: 400,
            ..CircuitConfig::wide_ecc_config()
        };
        let mut builder = CircuitBuilder::<F, D>::new(config);
        
        println!("=== KZG Full Flow Test ===");
        
        // Create test polynomial f(x) = 3x^2 + 2x + 1
        let coeffs = vec![
            builder.constant_nonnative(Bn128Scalar::from_canonical_u64(1)), // a_0
            builder.constant_nonnative(Bn128Scalar::from_canonical_u64(2)), // a_1
            builder.constant_nonnative(Bn128Scalar::from_canonical_u64(3)), // a_2
        ];
        
        // Setup phase (outside circuit)
        let params = KZGSetup::new_test_setup(16);
        
        // Convert setup to circuit targets
        let g1_gen = builder.g1_generator();
        let powers_of_tau = vec![g1_gen.clone(), g1_gen.clone(), g1_gen.clone()];
        
        // Get G2 tau
        let (_, g2_tau_value) = params.get_g2_powers();
        let g2_tau = builder.constant_affine_point_g2::<G2, Bn128Base>(*g2_tau_value);
        
        println!("Step 1: Creating commitment...");
        // Commitment phase
        let commitment = builder.kzg_commit(&coeffs, &powers_of_tau);
        
        println!("Step 2: Creating opening proof...");
        // Opening phase - prove evaluation at x = 2
        let point = builder.constant_nonnative(Bn128Scalar::from_canonical_u64(2));
        let (evaluation, proof) = builder.kzg_create_opening_proof(&coeffs, &point, &powers_of_tau);
        
        println!("Step 3: Verifying proof...");
        // Verification phase
        let is_valid = builder.kzg_verify(&commitment, &point, &evaluation, &proof, &g2_tau);
        
        // Assert the proof is valid
        builder.assert_one(is_valid.target);
        
        println!("Building circuit...");
        let data = builder.build::<C>();
        println!("Circuit stats:");
        println!("  - Gates: {}", data.common.gates.len());
        println!("  - Degree bits: {}", data.common.degree_bits());
        
        let pw = PartialWitness::new();
        println!("Generating proof...");
        let proof = data.prove(pw).unwrap();
        println!("Verifying proof...");
        data.verify(proof).unwrap();
        
        println!("✅ KZG full flow test passed!");
    }
    
    #[test]
    fn test_kzg_batch_verify() {
        let config = CircuitConfig {
            num_wires: 400,
            ..CircuitConfig::wide_ecc_config()
        };
        let mut builder = CircuitBuilder::<F, D>::new(config);
        
        println!("=== KZG Batch Verification Test ===");
        
        // Create multiple polynomials
        let poly1 = vec![
            builder.constant_nonnative(Bn128Scalar::from_canonical_u64(1)),
            builder.constant_nonnative(Bn128Scalar::from_canonical_u64(2)),
        ];
        let poly2 = vec![
            builder.constant_nonnative(Bn128Scalar::from_canonical_u64(3)),
            builder.constant_nonnative(Bn128Scalar::from_canonical_u64(4)),
        ];
        
        // Setup
        let params = KZGSetup::new_test_setup(16);
        let g1_gen = builder.g1_generator();
        let powers_of_tau = vec![g1_gen.clone(), g1_gen.clone()];
        let (_, g2_tau_value) = params.get_g2_powers();
        let g2_tau = builder.constant_affine_point_g2::<G2, Bn128Base>(*g2_tau_value);
        
        // Create commitments
        let commitment1 = builder.kzg_commit(&poly1, &powers_of_tau);
        let commitment2 = builder.kzg_commit(&poly2, &powers_of_tau);
        
        // Create opening proofs at different points
        let point1 = builder.constant_nonnative(Bn128Scalar::from_canonical_u64(1));
        let point2 = builder.constant_nonnative(Bn128Scalar::from_canonical_u64(2));
        
        let (eval1, proof1) = builder.kzg_create_opening_proof(&poly1, &point1, &powers_of_tau);
        let (eval2, proof2) = builder.kzg_create_opening_proof(&poly2, &point2, &powers_of_tau);
        
        // Batch verify
        let is_valid = builder.kzg_batch_verify(
            &[commitment1, commitment2],
            &[point1, point2],
            &[eval1, eval2],
            &[proof1, proof2],
            &g2_tau,
        );
        
        builder.assert_one(is_valid.target);
        
        println!("Building circuit...");
        let data = builder.build::<C>();
        let pw = PartialWitness::new();
        let proof = data.prove(pw).unwrap();
        data.verify(proof).unwrap();
        
        println!("✅ KZG batch verification test passed!");
    }
}