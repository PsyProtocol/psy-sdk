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
        nonnative_fp::{CircuitBuilderNonNative, NonNativeTarget},
        biguint::{CircuitBuilderBiguint, BigUintTarget},
    },
};

use crate::crypto::kzg::builder::CircuitBuilderKZG;

use num::{BigUint, Zero};

use std::marker::PhantomData;

#[derive(Clone, Debug)]
pub struct FFTSettingsTarget<F: RichField + Extendable<D>, const D: usize> {
    pub domain_size: usize,
    pub root_of_unity: NonNativeTarget<Bn128Scalar>,
    pub roots_of_unity: Vec<NonNativeTarget<Bn128Scalar>>,
    pub inv_roots_of_unity: Vec<NonNativeTarget<Bn128Scalar>>,
    pub domain_size_inv: NonNativeTarget<Bn128Scalar>,
    _phantom: PhantomData<F>,
}

pub trait CircuitBuilderFFT<F: RichField + Extendable<D>, const D: usize> {
    fn fft_settings(
        &mut self,
        domain_size: usize,
    ) -> FFTSettingsTarget<F, D>;
    
    fn fft_forward(
        &mut self,
        coeffs: &[NonNativeTarget<Bn128Scalar>],
        settings: &FFTSettingsTarget<F, D>,
    ) -> Vec<NonNativeTarget<Bn128Scalar>>;
    
    fn fft_inverse(
        &mut self,
        evals: &[NonNativeTarget<Bn128Scalar>],
        settings: &FFTSettingsTarget<F, D>,
    ) -> Vec<NonNativeTarget<Bn128Scalar>>;
    
    fn lagrange_interpolate_at_point(
        &mut self,
        evaluations: &[NonNativeTarget<Bn128Scalar>],
        point: &NonNativeTarget<Bn128Scalar>,
        settings: &FFTSettingsTarget<F, D>,
    ) -> NonNativeTarget<Bn128Scalar>;
    
    fn primitive_root_of_unity(
        &mut self,
        n: usize,
    ) -> NonNativeTarget<Bn128Scalar>;
}

impl<F: RichField + Extendable<D>, const D: usize> CircuitBuilderFFT<F, D> 
    for CircuitBuilder<F, D> 
{
    fn fft_settings(
        &mut self,
        domain_size: usize,
    ) -> FFTSettingsTarget<F, D> {
        assert!(domain_size.is_power_of_two(), "Domain size must be power of 2");
        
        let root = self.primitive_root_of_unity(domain_size);
        
        let mut roots_of_unity = Vec::with_capacity(domain_size);
        let mut inv_roots_of_unity = Vec::with_capacity(domain_size);
        
        let one = self.one_nonnative();
        roots_of_unity.push(one.clone());
        inv_roots_of_unity.push(one.clone());
        
        let root_inv = self.inv_nonnative(&root);
        
        for i in 1..domain_size {
            let prev_root = &roots_of_unity[i-1];
            let next_root = self.mul_nonnative(prev_root, &root);
            roots_of_unity.push(next_root);
            
            let prev_inv_root = &inv_roots_of_unity[i-1];
            let next_inv_root = self.mul_nonnative(prev_inv_root, &root_inv);
            inv_roots_of_unity.push(next_inv_root);
        }
        
        let domain_size_scalar = self.constant_nonnative(
            Bn128Scalar::from_canonical_usize(domain_size)
        );
        let domain_size_inv = self.inv_nonnative(&domain_size_scalar);
        
        FFTSettingsTarget {
            domain_size,
            root_of_unity: root,
            roots_of_unity,
            inv_roots_of_unity,
            domain_size_inv,
            _phantom: PhantomData,
        }
    }
    
    fn fft_forward(
        &mut self,
        coeffs: &[NonNativeTarget<Bn128Scalar>],
        settings: &FFTSettingsTarget<F, D>,
    ) -> Vec<NonNativeTarget<Bn128Scalar>> {
        let n = settings.domain_size;
        assert_eq!(coeffs.len(), n, "Input size must match domain size");
        
        let mut values = coeffs.to_vec();
        
        for i in 0..n {
            let j = reverse_bits(i, n.trailing_zeros());
            if i < j {
                values.swap(i, j);
            }
        }
        
        let mut m = 1;
        while m < n {
            let half_m = m;
            m <<= 1;
            
            let round_root_idx = n / m;
            let round_root = &settings.roots_of_unity[round_root_idx];
            
            for k in (0..n).step_by(m) {
                let mut root_power = self.one_nonnative();
                
                for j in 0..half_m {
                    let t = self.mul_nonnative(&root_power, &values[k + j + half_m]);
                    let u = values[k + j].clone();
                    
                    values[k + j] = self.add_nonnative(&u, &t);
                    values[k + j + half_m] = self.sub_nonnative(&u, &t);
                    
                    root_power = self.mul_nonnative(&root_power, round_root);
                }
            }
        }
        
        values
    }
    
    fn fft_inverse(
        &mut self,
        evals: &[NonNativeTarget<Bn128Scalar>],
        settings: &FFTSettingsTarget<F, D>,
    ) -> Vec<NonNativeTarget<Bn128Scalar>> {
        let n = settings.domain_size;
        assert_eq!(evals.len(), n, "Input size must match domain size");
        
        let mut values = evals.to_vec();
        
        for i in 0..n {
            let j = reverse_bits(i, n.trailing_zeros());
            if i < j {
                values.swap(i, j);
            }
        }
        
        let mut m = 1;
        while m < n {
            let half_m = m;
            m <<= 1;
            
            let round_root_idx = n / m;
            let round_root = &settings.inv_roots_of_unity[round_root_idx];
            
            for k in (0..n).step_by(m) {
                let mut root_power = self.one_nonnative();
                
                for j in 0..half_m {
                    let t = self.mul_nonnative(&root_power, &values[k + j + half_m]);
                    let u = values[k + j].clone();
                    
                    values[k + j] = self.add_nonnative(&u, &t);
                    values[k + j + half_m] = self.sub_nonnative(&u, &t);
                    
                    root_power = self.mul_nonnative(&root_power, round_root);
                }
            }
        }
        
        for value in &mut values {
            *value = self.mul_nonnative(value, &settings.domain_size_inv);
        }
        
        values
    }
    
    fn lagrange_interpolate_at_point(
        &mut self,
        evaluations: &[NonNativeTarget<Bn128Scalar>],
        point: &NonNativeTarget<Bn128Scalar>,
        settings: &FFTSettingsTarget<F, D>,
    ) -> NonNativeTarget<Bn128Scalar> {
        let n = settings.domain_size;
        assert_eq!(evaluations.len(), n);
        
        
        let mut numerators = Vec::with_capacity(n);
        let mut denominators = Vec::with_capacity(n);
        
        for i in 0..n {
            let diff = self.sub_nonnative(point, &settings.roots_of_unity[i]);
            denominators.push(diff);
            numerators.push(evaluations[i].clone());
        }
        
        let inverses = self.kzg_batch_inverse(&denominators);
        
        let mut sum = self.zero_nonnative();
        for i in 0..n {
            let term = self.mul_nonnative(&numerators[i], &inverses[i]);
            sum = self.add_nonnative(&sum, &term);
        }
        
        let z_pow_n = CircuitBuilderPowExt::pow_nonnative(self, point, n);
        let one = self.one_nonnative();
        let z_pow_n_minus_1 = self.sub_nonnative(&z_pow_n, &one);
        let scaled = self.mul_nonnative(&z_pow_n_minus_1, &settings.domain_size_inv);
        
        self.mul_nonnative(&scaled, &sum)
    }
    
    fn primitive_root_of_unity(
        &mut self,
        n: usize,
    ) -> NonNativeTarget<Bn128Scalar> {
        
        assert!(n.is_power_of_two(), "n must be power of 2");
        
        let g = self.constant_nonnative(Bn128Scalar::from_canonical_u64(5));
        
        let r = Bn128Scalar::order();
        let r_minus_1 = r - BigUint::from(1u64);
        let n_biguint = BigUint::from(n);
        let exponent = r_minus_1 / n_biguint;
        
        self.pow_nonnative_biguint(&g, &exponent)
    }
}

fn reverse_bits(x: usize, log_n: u32) -> usize {
    let mut result = 0;
    for i in 0..log_n {
        if (x >> i) & 1 == 1 {
            result |= 1 << (log_n - 1 - i);
        }
    }
    result
}

trait CircuitBuilderPowExt<F: RichField + Extendable<D>, const D: usize> {
    fn pow_nonnative_biguint(
        &mut self,
        base: &NonNativeTarget<Bn128Scalar>,
        exponent: &BigUint,
    ) -> NonNativeTarget<Bn128Scalar>;
    
    fn pow_nonnative(
        &mut self,
        base: &NonNativeTarget<Bn128Scalar>,
        exponent: usize,
    ) -> NonNativeTarget<Bn128Scalar>;
}

impl<F: RichField + Extendable<D>, const D: usize> CircuitBuilderPowExt<F, D> 
    for CircuitBuilder<F, D> 
{
    fn pow_nonnative_biguint(
        &mut self,
        base: &NonNativeTarget<Bn128Scalar>,
        exponent: &BigUint,
    ) -> NonNativeTarget<Bn128Scalar> {
        let mut result = self.one_nonnative();
        let mut temp = base.clone();
        
        let bits = exponent.to_bytes_le();
        for byte in bits {
            for i in 0..8 {
                if (byte >> i) & 1 == 1 {
                    result = self.mul_nonnative(&result, &temp);
                }
                temp = self.mul_nonnative(&temp, &temp);
            }
        }
        
        result
    }
    
    fn pow_nonnative(
        &mut self,
        base: &NonNativeTarget<Bn128Scalar>,
        exponent: usize,
    ) -> NonNativeTarget<Bn128Scalar> {
        let mut result = self.one_nonnative();
        let mut temp = base.clone();
        let mut exp = exponent;
        
        while exp > 0 {
            if exp & 1 == 1 {
                result = self.mul_nonnative(&result, &temp);
            }
            temp = self.mul_nonnative(&temp, &temp);
            exp >>= 1;
        }
        
        result
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