use core::marker::PhantomData;

use crate::crypto::bn254::field::extension::quadratic::QuadraticExtension;
use crate::crypto::bn254::gadgets::biguint::{BigUintTarget, CircuitBuilderBiguint};
use crate::crypto::bn254::gadgets::g1::{G1AffineTarget, CircuitBuilderG1};
use crate::crypto::bn254::gadgets::g2::{G2AffineTarget, CircuitBuilderG2};
use crate::crypto::bn254::gadgets::nonnative_fp::{CircuitBuilderNonNative, NonNativeTarget};
use crate::crypto::bn254::gadgets::nonnative_fp2::{NonNativeTargetExt2, CircuitBuilderNonNativeExt2};
use crate::crypto::bn254::gadgets::split_nonnative::CircuitBuilderSplit;
use crate::crypto::bn254::field::{bn128_base::Bn128Base, bn128_scalar::Bn128Scalar};
use crate::u32::arithmetic_u32;

use num::BigUint;
use plonky2::field::extension::Extendable;
use plonky2::field::types::{Field, Sample, PrimeField};
use plonky2::hash::hash_types::RichField;
use plonky2::hash::keccak::KeccakHash;
use plonky2::iop::target::{BoolTarget, Target};
use plonky2::plonk::circuit_builder::CircuitBuilder;
use plonky2::plonk::config::{GenericHashOut, Hasher};

use crate::crypto::bn254::curve::{G1, G1Affine};
use crate::crypto::secp256k1::ecdsa::curve::curve_types::Curve;

const WINDOW_SIZE: usize = 4;

pub trait CircuitBuilderWindowedMul<F: RichField + Extendable<D>, const D: usize> {
    fn precompute_window_g1(
        &mut self,
        p: &G1AffineTarget<F, D>,
    ) -> Vec<G1AffineTarget<F, D>>;

    fn random_access_curve_points_g1(
        &mut self,
        access_index: Target,
        v: Vec<G1AffineTarget<F, D>>,
    ) -> G1AffineTarget<F, D>;

    fn curve_scalar_mul_windowed_g1(
        &mut self,
        p: &G1AffineTarget<F, D>,
        n: &NonNativeTarget<Bn128Scalar>,
    ) -> G1AffineTarget<F, D>;

    fn precompute_window_g2(
        &mut self,
        p: &G2AffineTarget<F, D>,
    ) -> Vec<G2AffineTarget<F, D>>;

    fn random_access_curve_points_g2(
        &mut self,
        access_index: Target,
        v: Vec<G2AffineTarget<F, D>>,
    ) -> G2AffineTarget<F, D>;

    fn curve_scalar_mul_windowed_g2(
        &mut self,
        p: &G2AffineTarget<F, D>,
        n: &NonNativeTarget<Bn128Scalar>,
    ) -> G2AffineTarget<F, D>;
}

impl<F: RichField + Extendable<D>, const D: usize> CircuitBuilderWindowedMul<F, D>
    for CircuitBuilder<F, D>
{
    fn precompute_window_g1(
        &mut self,
        p: &G1AffineTarget<F, D>,
    ) -> Vec<G1AffineTarget<F, D>> {
        let g = G1::GENERATOR_AFFINE;
        let starting = self.constant_g1_affine(g);
        
        let mut multiples = vec![starting.clone()];
        
        for i in 1..1 << WINDOW_SIZE {
            multiples.push(self.add_or_double_g1_affine(p, &multiples[i - 1]));
        }
        
        let neg_starting = self.neg_g1_affine(&starting);
        for i in 1..1 << WINDOW_SIZE {
            multiples[i] = self.add_or_double_g1_affine(&multiples[i], &neg_starting);
        }
        
        multiples
    }

    fn random_access_curve_points_g1(
        &mut self,
        access_index: Target,
        v: Vec<G1AffineTarget<F, D>>,
    ) -> G1AffineTarget<F, D> {
        let num_limbs = 8; // BN128 base field has 256 bits = 8 * 32-bit limbs
        let zero = arithmetic_u32::U32Target(self.zero());
        
        let x_limbs: Vec<Vec<_>> = (0..num_limbs)
            .map(|i| {
                v.iter()
                    .map(|p| p.x.value.limbs.get(i).unwrap_or(&zero).0)
                    .collect()
            })
            .collect();
            
        let y_limbs: Vec<Vec<_>> = (0..num_limbs)
            .map(|i| {
                v.iter()
                    .map(|p| p.y.value.limbs.get(i).unwrap_or(&zero).0)
                    .collect()
            })
            .collect();
            
        let is_infinity_targets: Vec<_> = v.iter().map(|p| p.is_infinity.target).collect();

        let selected_x_limbs: Vec<_> = x_limbs
            .iter()
            .map(|limbs| arithmetic_u32::U32Target(self.random_access(access_index, limbs.clone())))
            .collect();
            
        let selected_y_limbs: Vec<_> = y_limbs
            .iter()
            .map(|limbs| arithmetic_u32::U32Target(self.random_access(access_index, limbs.clone())))
            .collect();
            
        let selected_is_infinity = BoolTarget::new_unsafe(
            self.random_access(access_index, is_infinity_targets)
        );

        G1AffineTarget {
            x: NonNativeTarget {
                value: BigUintTarget {
                    limbs: selected_x_limbs,
                },
                _phantom: PhantomData,
            },
            y: NonNativeTarget {
                value: BigUintTarget {
                    limbs: selected_y_limbs,
                },
                _phantom: PhantomData,
            },
            is_infinity: selected_is_infinity,
            _phantom: PhantomData,
        }
    }

    fn curve_scalar_mul_windowed_g1(
        &mut self,
        p: &G1AffineTarget<F, D>,
        n: &NonNativeTarget<Bn128Scalar>,
    ) -> G1AffineTarget<F, D> {
        let hash_0 = KeccakHash::<25>::hash_no_pad(&[F::ZERO]);
        let hash_0_scalar = Bn128Scalar::from_noncanonical_biguint(BigUint::from_bytes_le(
            &GenericHashOut::<F>::to_bytes(&hash_0),
        ));
        
        let starting_scalar = hash_0_scalar;
        let starting_point_x = Bn128Base::from_canonical_u64(0x123456789abcdef0u64);
        let starting_point_y = Bn128Base::from_canonical_u64(0xfedcba9876543210u64);
        
        let mut result = self.constant_g1_affine(G1::GENERATOR_AFFINE);
        
        let precomputation = self.precompute_window_g1(p);
        let zero = self.zero();
        
        let windows = self.split_nonnative_to_4_bit_limbs(n);
        
        for i in (0..windows.len()).rev() {
            for _ in 0..WINDOW_SIZE {
                result = self.double_g1_affine(&result);
            }
            
            let window = windows[i];
            let to_add = self.random_access_curve_points_g1(window, precomputation.clone());
            
            let is_zero = self.is_equal(window, zero);
            let should_add = self.not(is_zero);
            
            let new_result = self.add_or_double_g1_affine(&result, &to_add);
            result = G1AffineTarget {
                x: self.select_nonnative(should_add, &new_result.x, &result.x),
                y: self.select_nonnative(should_add, &new_result.y, &result.y),
                is_infinity: {
                    let not_should_add = self.not(should_add);
                    let case_true = self.and(should_add, new_result.is_infinity);
                    let case_false = self.and(not_should_add, result.is_infinity);
                    self.or(case_true, case_false)
                },
                _phantom: PhantomData,
            };
        }
        
        result
    }

    fn precompute_window_g2(
        &mut self,
        p: &G2AffineTarget<F, D>,
    ) -> Vec<G2AffineTarget<F, D>> {
        let starting = G2AffineTarget {
            x: self.constant_nonnative_ext2(QuadraticExtension([
                Bn128Base::from_canonical_u64(1),
                Bn128Base::from_canonical_u64(2),
            ])),
            y: self.constant_nonnative_ext2(QuadraticExtension([
                Bn128Base::from_canonical_u64(3),
                Bn128Base::from_canonical_u64(4),
            ])),
            is_infinity: self._false(),
            _phantom: PhantomData,
        };
        
        let mut multiples = vec![starting.clone()];
        
        for i in 1..1 << WINDOW_SIZE {
            multiples.push(self.add_g2(p, &multiples[i - 1]));
        }
        
        let neg_starting = self.neg_g2(&starting);
        for i in 1..1 << WINDOW_SIZE {
            multiples[i] = self.add_g2(&multiples[i], &neg_starting);
        }
        
        multiples
    }

    fn random_access_curve_points_g2(
        &mut self,
        access_index: Target,
        v: Vec<G2AffineTarget<F, D>>,
    ) -> G2AffineTarget<F, D> {
        let num_limbs = 8; // BN128 base field has 256 bits = 8 * 32-bit limbs
        let zero = arithmetic_u32::U32Target(self.zero());
        
        let x_c0_limbs: Vec<Vec<_>> = (0..num_limbs)
            .map(|i| {
                v.iter()
                    .map(|p| p.x.c0.value.limbs.get(i).unwrap_or(&zero).0)
                    .collect()
            })
            .collect();
            
        let x_c1_limbs: Vec<Vec<_>> = (0..num_limbs)
            .map(|i| {
                v.iter()
                    .map(|p| p.x.c1.value.limbs.get(i).unwrap_or(&zero).0)
                    .collect()
            })
            .collect();
            
        let y_c0_limbs: Vec<Vec<_>> = (0..num_limbs)
            .map(|i| {
                v.iter()
                    .map(|p| p.y.c0.value.limbs.get(i).unwrap_or(&zero).0)
                    .collect()
            })
            .collect();
            
        let y_c1_limbs: Vec<Vec<_>> = (0..num_limbs)
            .map(|i| {
                v.iter()
                    .map(|p| p.y.c1.value.limbs.get(i).unwrap_or(&zero).0)
                    .collect()
            })
            .collect();

        let selected_x_c0_limbs: Vec<_> = x_c0_limbs
            .iter()
            .map(|limbs| arithmetic_u32::U32Target(self.random_access(access_index, limbs.clone())))
            .collect();
            
        let selected_x_c1_limbs: Vec<_> = x_c1_limbs
            .iter()
            .map(|limbs| arithmetic_u32::U32Target(self.random_access(access_index, limbs.clone())))
            .collect();
            
        let selected_y_c0_limbs: Vec<_> = y_c0_limbs
            .iter()
            .map(|limbs| arithmetic_u32::U32Target(self.random_access(access_index, limbs.clone())))
            .collect();
            
        let selected_y_c1_limbs: Vec<_> = y_c1_limbs
            .iter()
            .map(|limbs| arithmetic_u32::U32Target(self.random_access(access_index, limbs.clone())))
            .collect();

        let is_infinity_targets: Vec<_> = v.iter().map(|p| p.is_infinity.target).collect();
        let selected_is_infinity = BoolTarget::new_unsafe(
            self.random_access(access_index, is_infinity_targets)
        );

        G2AffineTarget {
            x: NonNativeTargetExt2 {
                c0: NonNativeTarget {
                    value: BigUintTarget {
                        limbs: selected_x_c0_limbs,
                    },
                    _phantom: PhantomData,
                },
                c1: NonNativeTarget {
                    value: BigUintTarget {
                        limbs: selected_x_c1_limbs,
                    },
                    _phantom: PhantomData,
                },
                _phantom: PhantomData,
            },
            y: NonNativeTargetExt2 {
                c0: NonNativeTarget {
                    value: BigUintTarget {
                        limbs: selected_y_c0_limbs,
                    },
                    _phantom: PhantomData,
                },
                c1: NonNativeTarget {
                    value: BigUintTarget {
                        limbs: selected_y_c1_limbs,
                    },
                    _phantom: PhantomData,
                },
                _phantom: PhantomData,
            },
            is_infinity: selected_is_infinity,
            _phantom: PhantomData,
        }
    }

    fn curve_scalar_mul_windowed_g2(
        &mut self,
        p: &G2AffineTarget<F, D>,
        n: &NonNativeTarget<Bn128Scalar>,
    ) -> G2AffineTarget<F, D> {
        let mut result = G2AffineTarget {
            x: self.constant_nonnative_ext2(QuadraticExtension([
                Bn128Base::ZERO,
                Bn128Base::ZERO,
            ])),
            y: self.constant_nonnative_ext2(QuadraticExtension([
                Bn128Base::ONE,
                Bn128Base::ZERO,
            ])),
            is_infinity: self._true(),
            _phantom: PhantomData,
        };
        
        let precomputation = self.precompute_window_g2(p);
        let zero = self.zero();
        
        let windows = self.split_nonnative_to_4_bit_limbs(n);
        
        for i in (0..windows.len()).rev() {
            for _ in 0..WINDOW_SIZE {
                result = self.add_g2(&result, &result); // double
            }
            
            let window = windows[i];
            let to_add = self.random_access_curve_points_g2(window, precomputation.clone());
            
            let is_zero = self.is_equal(window, zero);
            let should_add = self.not(is_zero);
            
            let new_result = self.add_g2(&result, &to_add);
            result = G2AffineTarget {
                x: NonNativeTargetExt2 {
                    c0: self.select_nonnative(should_add, &new_result.x.c0, &result.x.c0),
                    c1: self.select_nonnative(should_add, &new_result.x.c1, &result.x.c1),
                    _phantom: PhantomData,
                },
                y: NonNativeTargetExt2 {
                    c0: self.select_nonnative(should_add, &new_result.y.c0, &result.y.c0),
                    c1: self.select_nonnative(should_add, &new_result.y.c1, &result.y.c1),
                    _phantom: PhantomData,
                },
                is_infinity: {
                    let not_should_add = self.not(should_add);
                    let case_true = self.and(should_add, new_result.is_infinity);
                    let case_false = self.and(not_should_add, result.is_infinity);
                    self.or(case_true, case_false)
                },
                _phantom: PhantomData,
            };
        }
        
        result
    }
}