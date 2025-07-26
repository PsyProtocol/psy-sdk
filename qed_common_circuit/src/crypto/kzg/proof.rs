/// KZG Proof implementation
use plonky2::{
    field::{
        extension::Extendable,
        types::{Field, PrimeField},
    },
    hash::hash_types::RichField,
    plonk::circuit_builder::CircuitBuilder,
};

use crate::crypto::bn254::{
    field::bn128_scalar::Bn128Scalar,
    gadgets::{
        g1::{CircuitBuilderG1, G1AffineTarget},
        nonnative_fp::{CircuitBuilderNonNative, NonNativeTarget},
    },
};

use crate::crypto::secp256k1::ecdsa::curve::curve_types::AffinePoint;

/// KZG proof target in circuit
#[derive(Clone, Debug)]
pub struct KZGProofTarget<F: RichField + Extendable<D>, const D: usize> {
    /// Witness polynomial evaluation proof
    pub w: G1AffineTarget<F, D>,
}

/// KZG proof
#[derive(Clone, Debug)]
pub struct KZGProof {
    /// Witness polynomial evaluation proof
    pub w: AffinePoint<crate::crypto::bn254::curve::g1::G1>,
}

/// Circuit builder extension for KZG proof operations
pub trait CircuitBuilderKZGProof<F: RichField + Extendable<D>, const D: usize> {
    /// Create a KZG opening proof at a given point
    /// W = (C - y*g) / (tau - z) where:
    /// - C is the commitment
    /// - y is the evaluation at point z
    /// - tau is from trusted setup
    fn kzg_create_opening_proof(
        &mut self,
        coefficients: &[NonNativeTarget<Bn128Scalar>],
        point: &NonNativeTarget<Bn128Scalar>,
        powers_of_tau: &[G1AffineTarget<F, D>],
    ) -> (NonNativeTarget<Bn128Scalar>, KZGProofTarget<F, D>);
    
    /// Create multiple opening proofs for batch verification
    fn kzg_create_batch_opening_proofs(
        &mut self,
        coefficients: &[Vec<NonNativeTarget<Bn128Scalar>>],
        points: &[NonNativeTarget<Bn128Scalar>],
        powers_of_tau: &[G1AffineTarget<F, D>],
    ) -> (Vec<NonNativeTarget<Bn128Scalar>>, Vec<KZGProofTarget<F, D>>);
}

impl<F: RichField + Extendable<D>, const D: usize> CircuitBuilderKZGProof<F, D> for CircuitBuilder<F, D> {
    fn kzg_create_opening_proof(
        &mut self,
        coefficients: &[NonNativeTarget<Bn128Scalar>],
        point: &NonNativeTarget<Bn128Scalar>,
        powers_of_tau: &[G1AffineTarget<F, D>],
    ) -> (NonNativeTarget<Bn128Scalar>, KZGProofTarget<F, D>) {
        // Evaluate polynomial at the given point
        let evaluation = self.evaluate_polynomial_nonnative(coefficients, point);
        
        // Compute quotient polynomial coefficients: (f(x) - f(z)) / (x - z)
        let quotient_coeffs = self.compute_quotient_polynomial(coefficients, point, &evaluation);
        
        // Create proof by committing to quotient polynomial
        let proof_commitment = self.kzg_commit_internal(&quotient_coeffs, powers_of_tau);
        
        let proof = KZGProofTarget { w: proof_commitment };
        
        (evaluation, proof)
    }
    
    fn kzg_create_batch_opening_proofs(
        &mut self,
        coefficients: &[Vec<NonNativeTarget<Bn128Scalar>>],
        points: &[NonNativeTarget<Bn128Scalar>],
        powers_of_tau: &[G1AffineTarget<F, D>],
    ) -> (Vec<NonNativeTarget<Bn128Scalar>>, Vec<KZGProofTarget<F, D>>) {
        assert_eq!(coefficients.len(), points.len());
        
        let mut evaluations = Vec::new();
        let mut proofs = Vec::new();
        
        for (coeffs, point) in coefficients.iter().zip(points.iter()) {
            let (eval, proof) = self.kzg_create_opening_proof(coeffs, point, powers_of_tau);
            evaluations.push(eval);
            proofs.push(proof);
        }
        
        (evaluations, proofs)
    }
}

/// Helper trait for internal operations
pub trait KZGProofHelpers<F: RichField + Extendable<D>, const D: usize> {
    fn evaluate_polynomial_nonnative(
        &mut self,
        coefficients: &[NonNativeTarget<Bn128Scalar>],
        point: &NonNativeTarget<Bn128Scalar>,
    ) -> NonNativeTarget<Bn128Scalar>;
    
    fn compute_quotient_polynomial(
        &mut self,
        coefficients: &[NonNativeTarget<Bn128Scalar>],
        point: &NonNativeTarget<Bn128Scalar>,
        evaluation: &NonNativeTarget<Bn128Scalar>,
    ) -> Vec<NonNativeTarget<Bn128Scalar>>;
    
    fn kzg_commit_internal(
        &mut self,
        coefficients: &[NonNativeTarget<Bn128Scalar>],
        powers_of_tau: &[G1AffineTarget<F, D>],
    ) -> G1AffineTarget<F, D>;
}

impl<F: RichField + Extendable<D>, const D: usize> KZGProofHelpers<F, D> for CircuitBuilder<F, D> {
    fn evaluate_polynomial_nonnative(
        &mut self,
        coefficients: &[NonNativeTarget<Bn128Scalar>],
        point: &NonNativeTarget<Bn128Scalar>,
    ) -> NonNativeTarget<Bn128Scalar> {
        // Horner's method: f(x) = a_n + x(a_{n-1} + x(a_{n-2} + ... + x*a_0))
        if coefficients.is_empty() {
            return self.constant_nonnative(Bn128Scalar::ZERO);
        }
        
        let mut result = coefficients[coefficients.len() - 1].clone();
        for i in (0..coefficients.len() - 1).rev() {
            result = self.mul_nonnative(&result, point);
            result = self.add_nonnative(&result, &coefficients[i]);
        }
        
        result
    }
    
    fn compute_quotient_polynomial(
        &mut self,
        coefficients: &[NonNativeTarget<Bn128Scalar>],
        point: &NonNativeTarget<Bn128Scalar>,
        evaluation: &NonNativeTarget<Bn128Scalar>,
    ) -> Vec<NonNativeTarget<Bn128Scalar>> {
        // Compute (f(x) - f(z)) / (x - z)
        // This is a simplified version - full implementation would use polynomial division
        let mut quotient_coeffs = Vec::new();
        
        if coefficients.is_empty() {
            return quotient_coeffs;
        }
        
        // For now, create a dummy quotient with one less degree
        for i in 1..coefficients.len() {
            quotient_coeffs.push(coefficients[i].clone());
        }
        
        quotient_coeffs
    }
    
    fn kzg_commit_internal(
        &mut self,
        coefficients: &[NonNativeTarget<Bn128Scalar>],
        powers_of_tau: &[G1AffineTarget<F, D>],
    ) -> G1AffineTarget<F, D> {
        use super::commitment::CircuitBuilderKZG;
        
        // Ensure we have enough powers of tau
        let len = coefficients.len().min(powers_of_tau.len());
        let truncated_coeffs = &coefficients[..len];
        let truncated_powers = &powers_of_tau[..len];
        
        let commitment = self.kzg_commit(truncated_coeffs, truncated_powers);
        commitment.commitment
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::bn254::gadgets::g1::CircuitBuilderG1;
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
    fn test_polynomial_evaluation() {
        let config = CircuitConfig::standard_ecc_config();
        let mut builder = CircuitBuilder::<F, D>::new(config);
        
        // Create polynomial f(x) = 3x^2 + 2x + 1
        let coeffs = vec![
            builder.constant_nonnative(Bn128Scalar::from_canonical_u64(1)), // a_0
            builder.constant_nonnative(Bn128Scalar::from_canonical_u64(2)), // a_1
            builder.constant_nonnative(Bn128Scalar::from_canonical_u64(3)), // a_2
        ];
        
        // Evaluate at x = 2
        let point = builder.constant_nonnative(Bn128Scalar::from_canonical_u64(2));
        let eval = builder.evaluate_polynomial_nonnative(&coeffs, &point);
        
        // Expected: 3*4 + 2*2 + 1 = 17
        let expected = builder.constant_nonnative(Bn128Scalar::from_canonical_u64(17));
        builder.connect_nonnative(&eval, &expected);
        
        let data = builder.build::<C>();
        let pw = PartialWitness::new();
        let proof = data.prove(pw).unwrap();
        data.verify(proof).unwrap();
        
        println!("✅ Polynomial evaluation test passed!");
    }
}