use std::marker::PhantomData;

use plonky2::{
    field::{extension::Extendable, types::Field},
    hash::hash_types::RichField,
    iop::target::{BoolTarget, Target},
    plonk::circuit_builder::CircuitBuilder,
};

use crate::crypto::bn254::{
    gadgets::{
        nonnative_fp::CircuitBuilderNonNative,
        nonnative_fp2::{CircuitBuilderNonNativeExt2, NonNativeTargetExt2},
    },
    curve::{G2Affine, G2Projective},
    field::{
        bn128_base::Bn128Base,
        extension::quadratic::QuadraticExtension,
    },
};

type Fp2 = QuadraticExtension<Bn128Base>;

#[derive(Clone, Debug)]
pub struct G2AffineTarget<F: RichField + Extendable<D>, const D: usize> {
    pub x: NonNativeTargetExt2<Bn128Base>,
    pub y: NonNativeTargetExt2<Bn128Base>,
    pub is_infinity: BoolTarget,
    pub _phantom: PhantomData<F>,
}

pub trait CircuitBuilderG2<F: RichField + Extendable<D>, const D: usize> {
    fn add_virtual_g2_affine_target(&mut self) -> G2AffineTarget<F, D>;
    
    fn constant_g2_affine(&mut self, point: G2Affine) -> G2AffineTarget<F, D>;
    
    fn is_equal_g2(
        &mut self,
        p1: &G2AffineTarget<F, D>,
        p2: &G2AffineTarget<F, D>,
    ) -> BoolTarget;
    
    fn select_g2(
        &mut self,
        condition: BoolTarget,
        true_point: &G2AffineTarget<F, D>,
        false_point: &G2AffineTarget<F, D>,
    ) -> G2AffineTarget<F, D>;
    
    fn add_g2(
        &mut self,
        a: &G2AffineTarget<F, D>,
        b: &G2AffineTarget<F, D>,
    ) -> G2AffineTarget<F, D>;
    
    fn neg_g2(
        &mut self,
        p: &G2AffineTarget<F, D>,
    ) -> G2AffineTarget<F, D>;
}

impl<F: RichField + Extendable<D>, const D: usize> CircuitBuilderG2<F, D>
    for CircuitBuilder<F, D>
{
    fn add_virtual_g2_affine_target(&mut self) -> G2AffineTarget<F, D> {
        G2AffineTarget {
            x: self.add_virtual_nonnative_ext2_target(),
            y: self.add_virtual_nonnative_ext2_target(),
            is_infinity: self.add_virtual_bool_target_safe(),
            _phantom: PhantomData,
        }
    }
    
    fn constant_g2_affine(&mut self, point: G2Affine) -> G2AffineTarget<F, D> {
        G2AffineTarget {
            x: self.constant_nonnative_ext2(point.x),
            y: self.constant_nonnative_ext2(point.y),
            is_infinity: self.constant_bool(point.zero),
            _phantom: PhantomData,
        }
    }
    
    fn is_equal_g2(
        &mut self,
        p1: &G2AffineTarget<F, D>,
        p2: &G2AffineTarget<F, D>,
    ) -> BoolTarget {
        let x_equal = self.is_equal_ext2(&p1.x, &p2.x);
        let y_equal = self.is_equal_ext2(&p1.y, &p2.y);
        let infinity_equal = self.is_equal(p1.is_infinity.target, p2.is_infinity.target);
        
        let coords_equal = self.and(x_equal, y_equal);
        self.and(coords_equal, infinity_equal)
    }
    
    fn select_g2(
        &mut self,
        condition: BoolTarget,
        true_point: &G2AffineTarget<F, D>,
        false_point: &G2AffineTarget<F, D>,
    ) -> G2AffineTarget<F, D> {
        G2AffineTarget {
            x: self.select_ext2(condition, &true_point.x, &false_point.x),
            y: self.select_ext2(condition, &true_point.y, &false_point.y),
            is_infinity: BoolTarget::new_unsafe(self.select(condition, true_point.is_infinity.target, false_point.is_infinity.target)),
            _phantom: PhantomData,
        }
    }
    
    fn add_g2(
        &mut self,
        a: &G2AffineTarget<F, D>,
        b: &G2AffineTarget<F, D>,
    ) -> G2AffineTarget<F, D> {
        let a_is_inf = a.is_infinity;
        let b_is_inf = b.is_infinity;
        
        let result_if_a_inf = b.clone();
        
        let result_if_b_inf = a.clone();
        
        let y_diff = self.sub_nonnative_ext2(&b.y, &a.y);
        let x_diff = self.sub_nonnative_ext2(&b.x, &a.x);
        let x_diff_inv = self.inv_nonnative_ext2(&x_diff);
        let slope = self.mul_nonnative_ext2(&y_diff, &x_diff_inv);
        
        let slope_squared = self.squared_nonnative_ext2(&slope);
        let x3_temp = self.sub_nonnative_ext2(&slope_squared, &a.x);
        let x3 = self.sub_nonnative_ext2(&x3_temp, &b.x);
        
        let x1_minus_x3 = self.sub_nonnative_ext2(&a.x, &x3);
        let y3_temp = self.mul_nonnative_ext2(&slope, &x1_minus_x3);
        let y3 = self.sub_nonnative_ext2(&y3_temp, &a.y);
        
        let regular_result = G2AffineTarget {
            x: x3,
            y: y3,
            is_infinity: self._false(),
            _phantom: PhantomData,
        };
        
        let result_if_not_a_inf = self.select_g2(b_is_inf, &result_if_b_inf, &regular_result);
        self.select_g2(a_is_inf, &result_if_a_inf, &result_if_not_a_inf)
    }
    
    fn neg_g2(
        &mut self,
        p: &G2AffineTarget<F, D>,
    ) -> G2AffineTarget<F, D> {
        G2AffineTarget {
            x: p.x.clone(),
            y: self.neg_nonnative_ext2(&p.y),
            is_infinity: p.is_infinity,
            _phantom: PhantomData,
        }
    }
}


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
    fn test_g2_basic() {
        let config = crate::crypto::bn254::pairing_config();
        let mut builder = CircuitBuilder::<F, D>::new(config);

        let point = G2Affine {
            x: QuadraticExtension([Bn128Base::ONE, Bn128Base::ZERO]),
            y: QuadraticExtension([Bn128Base::from_canonical_u64(2), Bn128Base::ZERO]),
            zero: false,
        };
        
        let g2_target = builder.constant_g2_affine(point);
        
        let same_point = builder.constant_g2_affine(point);
        let is_equal = builder.is_equal_g2(&g2_target, &same_point);
        builder.assert_one(is_equal.target);

        let data = builder.build::<C>();
        let pw = PartialWitness::new();
        let proof = data.prove(pw).unwrap();
        data.verify(proof).unwrap();
    }

    #[test]
    fn test_g2_selection() {
        let config = crate::crypto::bn254::pairing_config();
        let mut builder = CircuitBuilder::<F, D>::new(config);

        let point1 = G2Affine {
            x: QuadraticExtension([Bn128Base::ONE, Bn128Base::ZERO]),
            y: QuadraticExtension([Bn128Base::from_canonical_u64(2), Bn128Base::ZERO]),
            zero: false,
        };
        
        let point2 = G2Affine {
            x: QuadraticExtension([Bn128Base::from_canonical_u64(3), Bn128Base::ZERO]),
            y: QuadraticExtension([Bn128Base::from_canonical_u64(4), Bn128Base::ZERO]),
            zero: false,
        };
        
        let g2_target1 = builder.constant_g2_affine(point1);
        let g2_target2 = builder.constant_g2_affine(point2);
        
        let true_condition = builder._true();
        let selected = builder.select_g2(true_condition, &g2_target1, &g2_target2);
        
        let is_first = builder.is_equal_g2(&selected, &g2_target1);
        builder.assert_one(is_first.target);

        let data = builder.build::<C>();
        let pw = PartialWitness::new();
        let proof = data.prove(pw).unwrap();
        data.verify(proof).unwrap();
    }
}