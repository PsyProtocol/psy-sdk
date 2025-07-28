use crate::crypto::bn254::{
    field::{
        bn128_base::Bn128Base,
        bn128_scalar::Bn128Scalar,
        extension::quadratic::QuadraticExtension,
    },
};

use crate::crypto::secp256k1::ecdsa::curve::curve_types::{AffinePoint, ProjectivePoint, Curve, CurveScalar};

#[derive(Clone, Debug)]
pub struct KZGParams {
    pub powers_of_tau_g1: Vec<AffinePoint<crate::crypto::bn254::curve::g1::G1>>,
    pub powers_of_tau_g2: Vec<AffinePoint<crate::crypto::bn254::curve::g2::G2>>,
    pub max_degree: usize,
    
    
    pub lagrange_g1: Option<Vec<AffinePoint<crate::crypto::bn254::curve::g1::G1>>>,
    pub roots_of_unity: Option<Vec<Bn128Scalar>>,
    pub is_lagrange_form: bool,
}

pub struct KZGSetup;

impl KZGSetup {
    pub fn new_trusted_setup(tau: Bn128Scalar, max_degree: usize) -> KZGParams {
        use crate::crypto::bn254::curve::{g1::G1, g2::G2};
        
        
        let mut powers_of_tau_g1 = Vec::with_capacity(max_degree);
        let g1_gen = G1::GENERATOR_AFFINE;
        let mut tau_power = Bn128Scalar::ONE;
        
        for _ in 0..max_degree {
            let point = (CurveScalar::<G1>(tau_power) * g1_gen.to_projective()).to_affine();
            powers_of_tau_g1.push(point);
            tau_power = tau_power * tau;
        }
        
        
        let mut powers_of_tau_g2 = Vec::with_capacity(2);
        let g2_gen = G2::GENERATOR_AFFINE;
        
        
        powers_of_tau_g2.push(g2_gen);
        
        let h_tau = (CurveScalar::<G2>(tau) * g2_gen.to_projective()).to_affine();
        powers_of_tau_g2.push(h_tau);
        
        KZGParams {
            powers_of_tau_g1,
            powers_of_tau_g2,
            max_degree,
            lagrange_g1: None,
            roots_of_unity: None,
            is_lagrange_form: false,
        }
    }
    
    #[cfg(test)]
    pub fn new_test_setup(max_degree: usize) -> KZGParams {
        
        let tau = Bn128Scalar::from_canonical_u64(12345);
        Self::new_trusted_setup(tau, max_degree)
    }
    
    pub fn new_lagrange_setup(tau: Bn128Scalar, domain_size: usize) -> KZGParams {
        use crate::crypto::bn254::curve::{g1::G1, g2::G2};
        use plonky2::field::types::Field;
        
        assert!(domain_size.is_power_of_two(), "Domain size must be power of 2");
        
        
        let mut params = Self::new_trusted_setup(tau, domain_size);
        
        
        let omega = Self::compute_primitive_root_of_unity(domain_size);
        
        
        let mut roots_of_unity = Vec::with_capacity(domain_size);
        let mut omega_power = Bn128Scalar::ONE;
        for _ in 0..domain_size {
            roots_of_unity.push(omega_power);
            omega_power = omega_power * omega;
        }
        
        
        
        let mut lagrange_g1 = Vec::with_capacity(domain_size);
        
        
        let mut tau_power = tau;
        for _ in 1..domain_size {
            tau_power = tau_power * tau;
        }
        let tau_n_minus_1 = tau_power - Bn128Scalar::ONE;
        
        
        let n_inv = Bn128Scalar::from_canonical_usize(domain_size).inverse();
        
        
        for i in 0..domain_size {
            let denominator = tau - roots_of_unity[i];
            if denominator == Bn128Scalar::ZERO {
                
                panic!("tau cannot be a root of unity for Lagrange setup");
            }
            let l_i_tau = tau_n_minus_1 * denominator.inverse() * n_inv;
            
            
            let g1_gen = G1::GENERATOR_AFFINE;
            let point = (CurveScalar::<G1>(l_i_tau) * g1_gen.to_projective()).to_affine();
            lagrange_g1.push(point);
        }
        
        params.lagrange_g1 = Some(lagrange_g1);
        params.roots_of_unity = Some(roots_of_unity);
        params.is_lagrange_form = true;
        
        params
    }
    
    fn compute_primitive_root_of_unity(n: usize) -> Bn128Scalar {
        use num::{BigUint, One};
        use plonky2::field::types::Field;
        
        assert!(n.is_power_of_two(), "n must be power of 2");
        
        
        let g = Bn128Scalar::from_canonical_u64(5);
        
        
        let r = Bn128Scalar::order();
        let r_minus_1 = r - BigUint::one();
        let n_biguint = BigUint::from(n);
        let exponent = r_minus_1 / n_biguint;
        
        
        g.exp_biguint(&exponent)
    }
    
    pub fn verify_setup(params: &KZGParams) -> bool {
        if params.powers_of_tau_g1.len() < 2 || params.powers_of_tau_g2.len() < 2 {
            return false;
        }
        
        
        
        true
    }
}

impl KZGParams {
    pub fn max_degree(&self) -> usize {
        self.max_degree
    }
    
    pub fn get_g1_powers(&self, degree: usize) -> Option<&[AffinePoint<crate::crypto::bn254::curve::g1::G1>]> {
        if degree <= self.powers_of_tau_g1.len() {
            Some(&self.powers_of_tau_g1[..degree])
        } else {
            None
        }
    }
    
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
    }
    
    #[test]
    fn test_powers_of_tau_consistency() {
        let tau = Bn128Scalar::from_canonical_u64(7);
        let params = KZGSetup::new_trusted_setup(tau, 4);
        
        use crate::crypto::bn254::curve::g1::G1;
        let g1_gen = G1::GENERATOR_AFFINE;
        
        assert_eq!(params.powers_of_tau_g1[0], g1_gen);
    }
}