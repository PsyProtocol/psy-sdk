/// G1 elliptic curve gadgets for plonky2 circuits
use std::marker::PhantomData;

use plonky2::{
    field::{extension::Extendable, types::Field},
    hash::hash_types::RichField,
    iop::target::{BoolTarget, Target},
    plonk::circuit_builder::CircuitBuilder,
};

use crate::crypto::bn254::{
    gadgets::{
        nonnative_fp::{CircuitBuilderNonNative, NonNativeTarget},
    },
    field::{
        bn128_base::Bn128Base,
        bn128_scalar::Bn128Scalar,
    },
    curve::{
        g1::G1,
    },
};

use crate::crypto::secp256k1::ecdsa::curve::curve_types::{AffinePoint, Curve, ProjectivePoint};
use crate::crypto::secp256k1::ecdsa::gadgets::curve::CircuitBuilderCurve;

// Type alias for compatibility
type G1Affine = AffinePoint<G1>;

/// G1 affine point in circuit
#[derive(Clone, Debug)]
pub struct G1AffineTarget<F: RichField + Extendable<D>, const D: usize> {
    pub x: NonNativeTarget<Bn128Base>,
    pub y: NonNativeTarget<Bn128Base>,
    pub is_infinity: BoolTarget,
    pub _phantom: PhantomData<F>,
}

/// G1 projective point in circuit
#[derive(Clone, Debug)]
pub struct G1ProjectiveTarget<F: RichField + Extendable<D>, const D: usize> {
    pub x: NonNativeTarget<Bn128Base>,
    pub y: NonNativeTarget<Bn128Base>,
    pub z: NonNativeTarget<Bn128Base>,
    pub _phantom: PhantomData<F>,
}

/// Circuit builder extension for G1 curve operations
pub trait CircuitBuilderG1<F: RichField + Extendable<D>, const D: usize> {
    /// Create G1 affine point target
    fn add_virtual_g1_affine_target(&mut self) -> G1AffineTarget<F, D>;
    
    /// Create G1 projective point target
    fn add_virtual_g1_projective_target(&mut self) -> G1ProjectiveTarget<F, D>;
    
    /// Create constant G1 affine point
    fn constant_g1_affine(&mut self, point: G1Affine) -> G1AffineTarget<F, D>;
    
    /// Add two G1 affine points
    fn add_g1_affine(
        &mut self,
        p1: &G1AffineTarget<F, D>,
        p2: &G1AffineTarget<F, D>,
    ) -> G1AffineTarget<F, D>;
    
    /// Double a G1 affine point
    fn double_g1_affine(&mut self, p: &G1AffineTarget<F, D>) -> G1AffineTarget<F, D>;
    
    /// Negate G1 affine point (negate y-coordinate)
    fn neg_g1_affine(&mut self, p: &G1AffineTarget<F, D>) -> G1AffineTarget<F, D>;
    
    /// Scalar multiplication for G1 point
    fn scalar_mul_g1(
        &mut self,
        point: &G1AffineTarget<F, D>,
        scalar: &NonNativeTarget<Bn128Scalar>,
    ) -> G1AffineTarget<F, D>;
    
    /// Check if G1 point is on curve
    fn assert_g1_on_curve(&mut self, point: &G1AffineTarget<F, D>);
    
    /// Check if two G1 points are equal
    fn is_equal_g1(
        &mut self,
        p1: &G1AffineTarget<F, D>,
        p2: &G1AffineTarget<F, D>,
    ) -> BoolTarget;
    
    /// Conditionally select between two G1 points
    fn select_g1(
        &mut self,
        condition: BoolTarget,
        true_point: &G1AffineTarget<F, D>,
        false_point: &G1AffineTarget<F, D>,
    ) -> G1AffineTarget<F, D>;
    
    /// Convert projective to affine
    fn g1_projective_to_affine(
        &mut self,
        p: &G1ProjectiveTarget<F, D>,
    ) -> G1AffineTarget<F, D>;
    
    /// Convert affine to projective
    fn g1_affine_to_projective(
        &mut self,
        p: &G1AffineTarget<F, D>,
    ) -> G1ProjectiveTarget<F, D>;
    
    /// G1 generator point
    fn g1_generator(&mut self) -> G1AffineTarget<F, D>;
    
    /// Connect two G1 affine points (equality constraint)
    fn connect_g1(
        &mut self,
        a: &G1AffineTarget<F, D>,
        b: &G1AffineTarget<F, D>,
    );
    
}

impl<F: RichField + Extendable<D>, const D: usize> CircuitBuilderG1<F, D>
    for CircuitBuilder<F, D>
{
    fn add_virtual_g1_affine_target(&mut self) -> G1AffineTarget<F, D> {
        G1AffineTarget {
            x: self.add_virtual_nonnative_target(),
            y: self.add_virtual_nonnative_target(),
            is_infinity: self.add_virtual_bool_target_safe(),
            _phantom: PhantomData,
        }
    }
    
    fn add_virtual_g1_projective_target(&mut self) -> G1ProjectiveTarget<F, D> {
        G1ProjectiveTarget {
            x: self.add_virtual_nonnative_target(),
            y: self.add_virtual_nonnative_target(),
            z: self.add_virtual_nonnative_target(),
            _phantom: PhantomData,
        }
    }
    
    fn constant_g1_affine(&mut self, point: G1Affine) -> G1AffineTarget<F, D> {
        G1AffineTarget {
            x: self.constant_nonnative(point.x),
            y: self.constant_nonnative(point.y),
            is_infinity: self.constant_bool(point.zero),
            _phantom: PhantomData,
        }
    }
    
    fn add_g1_affine(
        &mut self,
        p1: &G1AffineTarget<F, D>,
        p2: &G1AffineTarget<F, D>,
    ) -> G1AffineTarget<F, D> {
        // Simple addition following plonky2-pairing style
        // Does not handle infinity points - assumes all points are valid curve points
        let u = self.sub_nonnative(&p2.y, &p1.y);
        let v = self.sub_nonnative(&p2.x, &p1.x);
        let v_inv = self.inv_nonnative(&v);
        let s = self.mul_nonnative(&u, &v_inv);
        let s_squared = self.square_nonnative(&s);
        let x_sum = self.add_nonnative(&p2.x, &p1.x);
        let x3 = self.sub_nonnative(&s_squared, &x_sum);
        let x_diff = self.sub_nonnative(&p1.x, &x3);
        let prod = self.mul_nonnative(&s, &x_diff);
        let y3 = self.sub_nonnative(&prod, &p1.y);

        G1AffineTarget {
            x: x3,
            y: y3,
            is_infinity: self._false(),
            _phantom: PhantomData,
        }
    }
    
    fn double_g1_affine(&mut self, p: &G1AffineTarget<F, D>) -> G1AffineTarget<F, D> {
        // Point doubling formula for y^2 = x^3 + b (b=3 for BN254)
        // slope = (3*x^2) / (2*y)
        let x_squared = self.square_nonnative(&p.x);
        let two_x_squared = self.add_nonnative(&x_squared, &x_squared);
        let three_x_squared = self.add_nonnative(&x_squared, &two_x_squared);
        let two_y = self.add_nonnative(&p.y, &p.y);
        let slope = self.div_nonnative(&three_x_squared, &two_y);
        
        // x3 = slope^2 - 2*x
        let slope_squared = self.square_nonnative(&slope);
        let two_x = self.add_nonnative(&p.x, &p.x);
        let x3 = self.sub_nonnative(&slope_squared, &two_x);
        
        // y3 = slope * (x - x3) - y
        let x_diff = self.sub_nonnative(&p.x, &x3);
        let y3_temp = self.mul_nonnative(&slope, &x_diff);
        let y3 = self.sub_nonnative(&y3_temp, &p.y);
        
        G1AffineTarget {
            x: x3,
            y: y3,
            is_infinity: self._false(),
            _phantom: PhantomData,
        }
    }
    
    fn neg_g1_affine(&mut self, p: &G1AffineTarget<F, D>) -> G1AffineTarget<F, D> {
        G1AffineTarget {
            x: p.x.clone(),
            y: self.neg_nonnative(&p.y),
            is_infinity: p.is_infinity,
            _phantom: PhantomData,
        }
    }
    
    fn scalar_mul_g1(
        &mut self,
        point: &G1AffineTarget<F, D>,
        scalar: &NonNativeTarget<Bn128Scalar>,
    ) -> G1AffineTarget<F, D> {
        use crate::crypto::bn254::curve::g1::G1;
        use crate::crypto::secp256k1::ecdsa::curve::curve_types::{Curve, CurveScalar};
        
        let bits = self.split_nonnative_to_bits(scalar);

        let rando = (CurveScalar(Bn128Scalar::rand()) * G1::GENERATOR_PROJECTIVE).to_affine();
        let randot = self.constant_g1_affine(rando);
        // Result starts at `rando`, which is later subtracted, because we don't support arithmetic with the zero point.
        let mut result = self.add_virtual_g1_affine_target();
        self.connect_g1(&randot, &result);

        let mut two_i_times_p = self.add_virtual_g1_affine_target();
        self.connect_g1(point, &two_i_times_p);

        for &bit in bits.iter() {
            let not_bit = self.not(bit);

            let result_plus_2_i_p = self.add_g1_affine(&result, &two_i_times_p);

            let new_x_if_bit = self.mul_nonnative_by_bool(&result_plus_2_i_p.x, bit);
            let new_x_if_not_bit = self.mul_nonnative_by_bool(&result.x, not_bit);
            let new_y_if_bit = self.mul_nonnative_by_bool(&result_plus_2_i_p.y, bit);
            let new_y_if_not_bit = self.mul_nonnative_by_bool(&result.y, not_bit);

            let new_x = self.add_nonnative(&new_x_if_bit, &new_x_if_not_bit);
            let new_y = self.add_nonnative(&new_y_if_bit, &new_y_if_not_bit);

            result = G1AffineTarget { 
                x: new_x, 
                y: new_y,
                is_infinity: self._false(),
                _phantom: PhantomData,
            };

            two_i_times_p = self.double_g1_affine(&two_i_times_p);
        }

        // Subtract off result's initial value of `rando`.
        let neg_r = self.neg_g1_affine(&randot);
        result = self.add_g1_affine(&result, &neg_r);

        result
    }
    
    fn assert_g1_on_curve(&mut self, point: &G1AffineTarget<F, D>) {
        // Simple curve check following plonky2-pairing style
        // G1 curve equation: y^2 = x^3 + b, where b = 3
        let y_squared = self.square_nonnative(&point.y);
        let x_squared = self.square_nonnative(&point.x);
        let x_cubed = self.mul_nonnative(&x_squared, &point.x);
        let three = self.constant_nonnative(Bn128Base::from_canonical_u64(3));
        let rhs = self.add_nonnative(&x_cubed, &three);
        
        // Assert equality: y^2 = x^3 + 3
        self.connect_nonnative(&y_squared, &rhs);
    }
    
    fn is_equal_g1(
        &mut self,
        p1: &G1AffineTarget<F, D>,
        p2: &G1AffineTarget<F, D>,
    ) -> BoolTarget {
        let x_equal = self.is_equal_nonnative(&p1.x, &p2.x);
        let y_equal = self.is_equal_nonnative(&p1.y, &p2.y);
        let infinity_equal = self.is_equal(p1.is_infinity.target, p2.is_infinity.target);
        
        let coords_equal = self.and(x_equal, y_equal);
        self.and(coords_equal, infinity_equal)
    }
    
    fn select_g1(
        &mut self,
        condition: BoolTarget,
        true_point: &G1AffineTarget<F, D>,
        false_point: &G1AffineTarget<F, D>,
    ) -> G1AffineTarget<F, D> {
        G1AffineTarget {
            x: self.select_nonnative(condition, &true_point.x, &false_point.x),
            y: self.select_nonnative(condition, &true_point.y, &false_point.y),
            is_infinity: BoolTarget::new_unsafe(self.select(condition, true_point.is_infinity.target, false_point.is_infinity.target)),
            _phantom: PhantomData,
        }
    }
    
    fn g1_projective_to_affine(
        &mut self,
        p: &G1ProjectiveTarget<F, D>,
    ) -> G1AffineTarget<F, D> {
        // Convert (X, Y, Z) to (X/Z, Y/Z)
        let z_inv = self.inv_nonnative(&p.z);
        let x_affine = self.mul_nonnative(&p.x, &z_inv);
        let y_affine = self.mul_nonnative(&p.y, &z_inv);
        
        // Check if Z is zero (infinity point)
        let z_is_zero = self.is_zero_nonnative(&p.z);
        
        G1AffineTarget {
            x: x_affine,
            y: y_affine,
            is_infinity: z_is_zero,
            _phantom: PhantomData,
        }
    }
    
    fn g1_affine_to_projective(
        &mut self,
        p: &G1AffineTarget<F, D>,
    ) -> G1ProjectiveTarget<F, D> {
        let one = self.one_nonnative();
        let zero = self.zero_nonnative();
        
        G1ProjectiveTarget {
            x: self.select_nonnative(p.is_infinity, &zero, &p.x),
            y: self.select_nonnative(p.is_infinity, &one, &p.y),
            z: self.select_nonnative(p.is_infinity, &zero, &one),
            _phantom: PhantomData,
        }
    }
    
    fn g1_generator(&mut self) -> G1AffineTarget<F, D> {
        self.constant_g1_affine(G1::GENERATOR_AFFINE)
    }
    
    fn connect_g1(
        &mut self,
        a: &G1AffineTarget<F, D>,
        b: &G1AffineTarget<F, D>,
    ) {
        self.connect_nonnative(&a.x, &b.x);
        self.connect_nonnative(&a.y, &b.y);
        self.connect(a.is_infinity.target, b.is_infinity.target);
    }
}

// Temporarily disabled until dependencies are fixed
/*
#[cfg(test)]
mod tests {
    use super::*;
    use plonky2::{
        iop::witness::{PartialWitness, WitnessWrite},
        plonk::{circuit_data::CircuitConfig, config::{GenericConfig, PoseidonGoldilocksConfig}},
    };

    const D: usize = 2;
    type C = PoseidonGoldilocksConfig;
    type F = <C as GenericConfig<D>>::F;

    #[test]
    fn test_g1_generator() {
        let config = CircuitConfig::standard_ecc_config();
        let mut builder = CircuitBuilder::<F, D>::new(config);

        let gen = builder.g1_generator();
        builder.assert_g1_on_curve(&gen);

        let data = builder.build::<C>();
        let pw = PartialWitness::new();
        let proof = data.prove(pw).unwrap();
        data.verify(proof).unwrap();
    }

    #[test]
    fn test_g1_point_doubling() {
        let config = CircuitConfig::standard_ecc_config();
        let mut builder = CircuitBuilder::<F, D>::new(config);

        let gen = builder.g1_generator();
        let doubled = builder.double_g1_affine(&gen);
        
        builder.assert_g1_on_curve(&doubled);

        let data = builder.build::<C>();
        let pw = PartialWitness::new();
        let proof = data.prove(pw).unwrap();
        data.verify(proof).unwrap();
    }

    #[test]
    fn test_g1_point_addition() {
        let config = CircuitConfig::standard_ecc_config();
        let mut builder = CircuitBuilder::<F, D>::new(config);

        let gen = builder.g1_generator();
        let doubled = builder.double_g1_affine(&gen);
        let result = builder.add_g1_affine(&gen, &doubled);
        
        builder.assert_g1_on_curve(&result);

        let data = builder.build::<C>();
        let pw = PartialWitness::new();
        let proof = data.prove(pw).unwrap();
        data.verify(proof).unwrap();
    }

    #[test]
    fn test_g1_infinity_handling() {
        let config = CircuitConfig::standard_ecc_config();
        let mut builder = CircuitBuilder::<F, D>::new(config);

        let gen = builder.g1_generator();
        let infinity = G1AffineTarget {
            x: builder.zero_nonnative(),
            y: builder.zero_nonnative(), 
            is_infinity: builder._true(),
            _phantom: PhantomData,
        };
        
        // Gen + Infinity = Gen
        let result = builder.add_g1_affine(&gen, &infinity);
        let is_equal = builder.is_equal_g1(&result, &gen);
        builder.assert_one(is_equal.target);

        let data = builder.build::<C>();
        let pw = PartialWitness::new();
        let proof = data.prove(pw).unwrap();
        data.verify(proof).unwrap();
    }

    // Tests from plonky2-pairing
    #[test]
    fn test_curve_point_is_valid() -> anyhow::Result<()> {
        use crate::crypto::bn254::curve::g1::G1;
        use crate::crypto::secp256k1::ecdsa::gadgets::curve::CircuitBuilderCurve;
        use crate::crypto::secp256k1::ecdsa::curve::curve_types::Curve;

        let config = CircuitConfig::wide_ecc_config();

        let pw = PartialWitness::new();
        let mut builder = CircuitBuilder::<F, D>::new(config);

        let g = G1::GENERATOR_AFFINE;
        let g_target = builder.constant_affine_point(g);
        let neg_g_target = builder.curve_neg(&g_target);

        builder.curve_assert_valid(&g_target);
        builder.curve_assert_valid(&neg_g_target);

        let data = builder.build::<C>();
        let proof = data.prove(pw).unwrap();

        data.verify(proof)
    }

    #[test]
    #[should_panic]
    fn test_curve_point_is_not_valid() {
        use crate::crypto::bn254::field::bn128_base::Bn128Base;
        use crate::crypto::secp256k1::ecdsa::gadgets::curve::CircuitBuilderCurve;
        use crate::crypto::bn254::gadgets::nonnative_fp::CircuitBuilderNonNative;
        use crate::crypto::secp256k1::ecdsa::curve::curve_types::AffinePoint;

        let config = CircuitConfig::wide_ecc_config();

        let pw = PartialWitness::new();
        let mut builder = CircuitBuilder::<F, D>::new(config);

        let x = Bn128Base([17, 22, 22, 22]);
        let y = Bn128Base([17, 22, 22, 22]);
        let not_g = AffinePoint::<G1> { x, y, zero: false };
        let not_g_target = builder.constant_affine_point(not_g);

        builder.curve_assert_valid(&not_g_target);

        let data = builder.build::<C>();
        let proof = data.prove(pw).unwrap();

        data.verify(proof).unwrap();
    }

    #[test]
    fn test_curve_add() -> anyhow::Result<()> {
        use crate::crypto::bn254::curve::g1::G1;
        use crate::crypto::secp256k1::ecdsa::gadgets::curve::CircuitBuilderCurve;
        use crate::crypto::secp256k1::ecdsa::curve::curve_types::Curve;

        let config = CircuitConfig::wide_ecc_config();

        let pw = PartialWitness::new();
        let mut builder = CircuitBuilder::<F, D>::new(config);

        let g = G1::GENERATOR_AFFINE;
        let double_g = g.double();
        let g_plus_2g = (g + double_g).to_affine();
        let g_plus_2g_expected = builder.constant_affine_point(g_plus_2g);
        builder.curve_assert_valid(&g_plus_2g_expected);

        let g_target = builder.constant_affine_point(g);
        let double_g_target = builder.curve_double(&g_target);
        let g_plus_2g_actual = builder.curve_add(&g_target, &double_g_target);
        builder.curve_assert_valid(&g_plus_2g_actual);

        builder.connect_affine_point(&g_plus_2g_expected, &g_plus_2g_actual);

        let data = builder.build::<C>();
        let proof = data.prove(pw).unwrap();

        data.verify(proof)
    }

    #[test]
    fn test_curve_conditional_add() -> anyhow::Result<()> {
        use crate::crypto::bn254::curve::g1::G1;
        use crate::crypto::secp256k1::ecdsa::gadgets::curve::CircuitBuilderCurve;
        use crate::crypto::secp256k1::ecdsa::curve::curve_types::Curve;

        let config = CircuitConfig::wide_ecc_config();

        let pw = PartialWitness::new();
        let mut builder = CircuitBuilder::<F, D>::new(config);

        let g = G1::GENERATOR_AFFINE;
        let double_g = g.double();
        let g_plus_2g = (g + double_g).to_affine();
        let g_plus_2g_expected = builder.constant_affine_point(g_plus_2g);

        let g_expected = builder.constant_affine_point(g);
        let double_g_target = builder.curve_double(&g_expected);
        let t = builder._true();
        let f = builder._false();
        let g_plus_2g_actual = builder.curve_conditional_add(&g_expected, &double_g_target, t);
        let g_actual = builder.curve_conditional_add(&g_expected, &double_g_target, f);

        builder.connect_affine_point(&g_plus_2g_expected, &g_plus_2g_actual);
        builder.connect_affine_point(&g_expected, &g_actual);

        let data = builder.build::<C>();
        let proof = data.prove(pw).unwrap();

        data.verify(proof)
    }

    #[test]
    fn test_curve_mul() -> anyhow::Result<()> {
        use crate::crypto::bn254::curve::g1::G1;
        use crate::crypto::bn254::field::bn128_scalar::Bn128Scalar;
        use crate::crypto::secp256k1::ecdsa::gadgets::curve::CircuitBuilderCurve;
        use crate::crypto::bn254::gadgets::nonnative_fp::CircuitBuilderNonNative;
        use core::ops::Neg;
        use plonky2::field::types::Field;
        use crate::crypto::secp256k1::ecdsa::curve::curve_types::{Curve, CurveScalar};

        let config = CircuitConfig::wide_ecc_config();

        let pw = PartialWitness::new();
        let mut builder = CircuitBuilder::<F, D>::new(config);

        let g = G1::GENERATOR_PROJECTIVE.to_affine();
        let five = Bn128Scalar::from_canonical_usize(5);
        let neg_five = five.neg();
        let neg_five_scalar = CurveScalar::<G1>(neg_five);
        let neg_five_g = (neg_five_scalar * g.to_projective()).to_affine();
        let neg_five_g_expected = builder.constant_affine_point(neg_five_g);
        builder.curve_assert_valid(&neg_five_g_expected);

        let g_target = builder.constant_affine_point(g);
        let neg_five_target = builder.constant_nonnative(neg_five);
        let neg_five_g_actual = builder.curve_scalar_mul(&g_target, &neg_five_target);
        builder.curve_assert_valid(&neg_five_g_actual);

        builder.connect_affine_point(&neg_five_g_expected, &neg_five_g_actual);

        let data = builder.build::<C>();
        let proof = data.prove(pw).unwrap();

        data.verify(proof)
    }

    #[test]
    fn test_curve_random() -> anyhow::Result<()> {
        use crate::crypto::bn254::curve::g1::G1;
        use crate::crypto::bn254::field::bn128_scalar::Bn128Scalar;
        use crate::crypto::secp256k1::ecdsa::gadgets::curve::CircuitBuilderCurve;
        use crate::crypto::bn254::gadgets::nonnative_fp::CircuitBuilderNonNative;
        use plonky2::field::types::{Field, Sample};
        use crate::crypto::secp256k1::ecdsa::curve::curve_types::{Curve, CurveScalar};

        let config = CircuitConfig::wide_ecc_config();

        let pw = PartialWitness::new();
        let mut builder = CircuitBuilder::<F, D>::new(config);

        let rando = (CurveScalar(Bn128Scalar::rand()) * G1::GENERATOR_PROJECTIVE).to_affine();
        let randot = builder.constant_affine_point(rando);

        let two_target = builder.constant_nonnative(Bn128Scalar::TWO);
        let randot_doubled = builder.curve_double(&randot);
        let randot_times_two = builder.curve_scalar_mul(&randot, &two_target);
        builder.connect_affine_point(&randot_doubled, &randot_times_two);

        let data = builder.build::<C>();
        let proof = data.prove(pw).unwrap();

        data.verify(proof)
    }
}
*/