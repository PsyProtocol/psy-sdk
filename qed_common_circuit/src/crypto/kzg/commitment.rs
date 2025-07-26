/// KZG Commitment implementation
use plonky2::{
    field::{
        extension::Extendable,
        types::{Field, PrimeField},
    },
    hash::hash_types::RichField,
    iop::target::Target,
    plonk::circuit_builder::CircuitBuilder,
};

use crate::crypto::bn254::{
    field::{
        bn128_base::Bn128Base,
        bn128_scalar::Bn128Scalar,
    },
    gadgets::{
        g1::{CircuitBuilderG1, G1AffineTarget},
        nonnative_fp::{CircuitBuilderNonNative, NonNativeTarget},
    },
};

use crate::crypto::secp256k1::ecdsa::curve::curve_types::AffinePoint;

/// KZG commitment target in circuit
#[derive(Clone, Debug)]
pub struct KZGCommitmentTarget<F: RichField + Extendable<D>, const D: usize> {
    pub commitment: G1AffineTarget<F, D>,
}

/// KZG commitment
#[derive(Clone, Debug)]
pub struct KZGCommitment {
    pub commitment: AffinePoint<crate::crypto::bn254::curve::g1::G1>,
}

/// Circuit builder extension for KZG commitment operations
pub trait CircuitBuilderKZG<F: RichField + Extendable<D>, const D: usize> {
    /// Create a polynomial commitment: C = sum(a_i * g^(tau^i))
    fn kzg_commit(
        &mut self,
        coefficients: &[NonNativeTarget<Bn128Scalar>],
        powers_of_tau: &[G1AffineTarget<F, D>],
    ) -> KZGCommitmentTarget<F, D>;
    
    /// Create a commitment from evaluations using Lagrange interpolation
    fn kzg_commit_lagrange(
        &mut self,
        evaluations: &[NonNativeTarget<Bn128Scalar>],
        lagrange_powers: &[G1AffineTarget<F, D>],
    ) -> KZGCommitmentTarget<F, D>;
}

impl<F: RichField + Extendable<D>, const D: usize> CircuitBuilderKZG<F, D> for CircuitBuilder<F, D> {
    fn kzg_commit(
        &mut self,
        coefficients: &[NonNativeTarget<Bn128Scalar>],
        powers_of_tau: &[G1AffineTarget<F, D>],
    ) -> KZGCommitmentTarget<F, D> {
        assert_eq!(coefficients.len(), powers_of_tau.len(), 
            "Coefficients and powers of tau must have the same length");
        
        // Compute sum(a_i * g^(tau^i)) without using infinity point
        // Start with the first term to avoid infinity
        assert!(!coefficients.is_empty(), "Cannot commit to empty polynomial");
        
        let mut commitment = self.scalar_mul_g1(&powers_of_tau[0], &coefficients[0]);
        
        // Add remaining terms
        for (coeff, tau_power) in coefficients.iter().zip(powers_of_tau.iter()).skip(1) {
            let term = self.scalar_mul_g1(tau_power, coeff);
            commitment = self.add_g1_affine(&commitment, &term);
        }
        
        KZGCommitmentTarget { commitment }
    }
    
    fn kzg_commit_lagrange(
        &mut self,
        evaluations: &[NonNativeTarget<Bn128Scalar>],
        lagrange_powers: &[G1AffineTarget<F, D>],
    ) -> KZGCommitmentTarget<F, D> {
        assert_eq!(evaluations.len(), lagrange_powers.len(), 
            "Evaluations and Lagrange powers must have the same length");
        
        // Commitment in Lagrange form: C = sum(f_i * L_i(tau))
        self.kzg_commit(evaluations, lagrange_powers)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
    fn test_kzg_commitment_structure() {
        let config = CircuitConfig::standard_ecc_config();
        let mut builder = CircuitBuilder::<F, D>::new(config);
        
        // Create dummy coefficients
        let coeff1 = builder.constant_nonnative(Bn128Scalar::from_canonical_u64(5));
        let coeff2 = builder.constant_nonnative(Bn128Scalar::from_canonical_u64(3));
        let coefficients = vec![coeff1, coeff2];
        
        // Create dummy powers of tau
        let g1_gen = builder.g1_generator();
        let powers_of_tau = vec![g1_gen.clone(), g1_gen.clone()];
        
        // Create commitment
        let commitment = builder.kzg_commit(&coefficients, &powers_of_tau);
        
        println!("✅ KZG commitment structure test passed!");
    }
}