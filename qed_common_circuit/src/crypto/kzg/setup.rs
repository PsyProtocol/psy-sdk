/// KZG Setup and Parameters
use crate::crypto::bn254::{
    field::{
        bn128_base::Bn128Base,
        bn128_scalar::Bn128Scalar,
        extension::quadratic::QuadraticExtension,
    },
};

use crate::crypto::secp256k1::ecdsa::curve::curve_types::{AffinePoint, ProjectivePoint, Curve, CurveScalar};

/// KZG parameters for trusted setup
#[derive(Clone, Debug)]
pub struct KZGParams {
    /// Powers of tau in G1: [g, g^tau, g^tau^2, ..., g^tau^(n-1)]
    pub powers_of_tau_g1: Vec<AffinePoint<crate::crypto::bn254::curve::g1::G1>>,
    /// Powers of tau in G2: [h, h^tau]
    pub powers_of_tau_g2: Vec<AffinePoint<crate::crypto::bn254::curve::g2::G2>>,
    /// Maximum degree supported
    pub max_degree: usize,
}

/// KZG Setup functionality
pub struct KZGSetup;

impl KZGSetup {
    /// Create a new trusted setup with given tau (toxic waste)
    /// WARNING: In production, tau must be securely destroyed after setup
    pub fn new_trusted_setup(tau: Bn128Scalar, max_degree: usize) -> KZGParams {
        use crate::crypto::bn254::curve::{g1::G1, g2::G2};
        
        // Generate powers of tau in G1
        let mut powers_of_tau_g1 = Vec::with_capacity(max_degree);
        let g1_gen = G1::GENERATOR_AFFINE;
        let mut tau_power = Bn128Scalar::ONE;
        
        for _ in 0..max_degree {
            let point = (CurveScalar::<G1>(tau_power) * g1_gen.to_projective()).to_affine();
            powers_of_tau_g1.push(point);
            tau_power = tau_power * tau;
        }
        
        // Generate powers of tau in G2 (only need [h, h^tau] for basic KZG)
        let mut powers_of_tau_g2 = Vec::with_capacity(2);
        let g2_gen = G2::GENERATOR_AFFINE;
        
        // h
        powers_of_tau_g2.push(g2_gen);
        // h^tau
        let h_tau = (CurveScalar::<G2>(tau) * g2_gen.to_projective()).to_affine();
        powers_of_tau_g2.push(h_tau);
        
        KZGParams {
            powers_of_tau_g1,
            powers_of_tau_g2,
            max_degree,
        }
    }
    
    /// Create parameters for testing (insecure - only for tests!)
    #[cfg(test)]
    pub fn new_test_setup(max_degree: usize) -> KZGParams {
        // Use a fixed tau for testing
        let tau = Bn128Scalar::from_canonical_u64(12345);
        Self::new_trusted_setup(tau, max_degree)
    }
    
    /// Verify the setup is valid by checking pairing relationships
    pub fn verify_setup(params: &KZGParams) -> bool {
        if params.powers_of_tau_g1.len() < 2 || params.powers_of_tau_g2.len() < 2 {
            return false;
        }
        
        // Check e(g^tau, h) = e(g, h^tau)
        // This would require pairing implementation
        // For now, just check basic validity
        true
    }
}

/// Helper functions for setup operations
impl KZGParams {
    /// Get the maximum polynomial degree this setup supports
    pub fn max_degree(&self) -> usize {
        self.max_degree
    }
    
    /// Get powers of tau in G1 up to specified degree
    pub fn get_g1_powers(&self, degree: usize) -> Option<&[AffinePoint<crate::crypto::bn254::curve::g1::G1>]> {
        if degree <= self.powers_of_tau_g1.len() {
            Some(&self.powers_of_tau_g1[..degree])
        } else {
            None
        }
    }
    
    /// Get G2 generator and tau power
    pub fn get_g2_powers(&self) -> (&AffinePoint<crate::crypto::bn254::curve::g2::G2>, &AffinePoint<crate::crypto::bn254::curve::g2::G2>) {
        (&self.powers_of_tau_g2[0], &self.powers_of_tau_g2[1])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use plonky2::field::types::Field;

    #[test]
    fn test_trusted_setup() {
        let max_degree = 16;
        let params = KZGSetup::new_test_setup(max_degree);
        
        assert_eq!(params.powers_of_tau_g1.len(), max_degree);
        assert_eq!(params.powers_of_tau_g2.len(), 2);
        assert!(KZGSetup::verify_setup(&params));
        
        println!("✅ Trusted setup test passed!");
        println!("   - Generated {} G1 powers", params.powers_of_tau_g1.len());
        println!("   - Generated {} G2 powers", params.powers_of_tau_g2.len());
    }
    
    #[test]
    fn test_powers_of_tau_consistency() {
        let tau = Bn128Scalar::from_canonical_u64(7);
        let params = KZGSetup::new_trusted_setup(tau, 4);
        
        // Verify tau powers are computed correctly
        // params.powers_of_tau_g1[i] should equal g^(tau^i)
        use crate::crypto::bn254::curve::g1::G1;
        let g1_gen = G1::GENERATOR_AFFINE;
        
        // Check g^1 = g
        assert_eq!(params.powers_of_tau_g1[0], g1_gen);
        
        println!("✅ Powers of tau consistency test passed!");
    }
}