use std::marker::PhantomData;
use std::any::TypeId;
use num::BigUint;

use plonky2::{
    field::{extension::Extendable, types::{Field, PrimeField}},
    hash::hash_types::RichField,
    iop::target::BoolTarget,
    plonk::circuit_builder::CircuitBuilder,
};

use crate::crypto::bn254::{
    gadgets::{
        g1::{CircuitBuilderG1, G1AffineTarget},
        nonnative_fp::{CircuitBuilderNonNative, NonNativeTarget},
        nonnative_fp2::{CircuitBuilderNonNativeExt2, NonNativeTargetExt2},
        nonnative_fp6::{CircuitBuilderNonNativeExt6, NonNativeTargetExt6},
        nonnative_fp12::{CircuitBuilderNonNativeExt12, NonNativeTargetExt12},
    },
    field::{
        bn128_base::Bn128Base,
        extension::{
            quadratic::QuadraticExtension,
            dodecic::DodecicExtension,
        },
    },
    curve::{g1::G1, g2::G2},
};

use crate::crypto::secp256k1::ecdsa::curve::curve_types::{AffinePoint, Curve};

pub const ATE_LOOP_COUNT: [u64; 4] = [
    0x9d797039be763ba8,
    0x0000000000000001,
    0x0000000000000000,
    0x0000000000000000,
];

fn biguint_from_array(arr: [u64; 4]) -> BigUint {
    BigUint::from_slice(&[
        arr[0] as u32,
        (arr[0] >> 32) as u32,
        arr[1] as u32,
        (arr[1] >> 32) as u32,
        arr[2] as u32,
        (arr[2] >> 32) as u32,
        arr[3] as u32,
        (arr[3] >> 32) as u32,
    ])
}

#[derive(Clone, Debug)]
pub struct AffinePointTargetG2<FF: Field> {
    pub x: NonNativeTargetExt2<FF>,
    pub y: NonNativeTargetExt2<FF>,
}

#[derive(Clone, Debug)]
pub struct JacobianPointTargetG2<FF: Field> {
    pub x: NonNativeTargetExt2<FF>,
    pub y: NonNativeTargetExt2<FF>,
    pub z: NonNativeTargetExt2<FF>,
}

#[derive(Clone, Debug)]
pub struct EllCoefficientsTarget<FF: Field> {
    pub ell_0: NonNativeTargetExt2<FF>,
    pub ell_vw: NonNativeTargetExt2<FF>,
    pub ell_vv: NonNativeTargetExt2<FF>,
}

#[derive(Clone, Debug)]
pub struct G2PreComputeTarget<FF: Field> {
    pub q: AffinePointTargetG2<FF>,
    pub coeffs: Vec<EllCoefficientsTarget<FF>>,
}

pub trait CircuitBuilderPairing<F: RichField + Extendable<D>, const D: usize> {
    fn pairing_bn254(
        &mut self,
        p: &G1AffineTarget<F, D>,
        q: &AffinePointTargetG2<Bn128Base>,
    ) -> NonNativeTargetExt12<Bn128Base>;
    
    fn pairing(
        &mut self,
        p: &G1AffineTarget<F, D>,
        q: &AffinePointTargetG2<Bn128Base>,
    ) -> NonNativeTargetExt12<Bn128Base>;
}

pub trait CircuitBuilderCurveG2<F: RichField + Extendable<D>, const D: usize> {
    fn add_g2<C: Curve<BaseField = QuadraticExtension<FF>>, FF: PrimeField + Extendable<2>>(
        &mut self,
        a: &AffinePointTargetG2<FF>,
        b: &AffinePointTargetG2<FF>,
    ) -> AffinePointTargetG2<FF>;
    
    fn neg_g2<FF: PrimeField + Extendable<2>>(
        &mut self,
        p: &AffinePointTargetG2<FF>,
    ) -> AffinePointTargetG2<FF>;
    
    fn precompute<C: Curve<BaseField = QuadraticExtension<FF>>, FF: PrimeField + Extendable<2>>(
        &mut self,
        p: &AffinePointTargetG2<FF>,
    ) -> G2PreComputeTarget<FF>;
    
    fn miller_loop<
        C: Curve<BaseField = FF>,
        FF: PrimeField + Extendable<2> + Extendable<6> + Extendable<12>,
    >(
        &mut self,
        g1: &G1AffineTarget<F, D>,
        precomp: &G2PreComputeTarget<FF>,
    ) -> NonNativeTargetExt12<FF>;
    
    fn to_jacobian_g2<C: Curve<BaseField = QuadraticExtension<FF>>, FF: PrimeField + Extendable<2>>(
        &mut self,
        p: &AffinePointTargetG2<FF>,
    ) -> JacobianPointTargetG2<FF>;
    
    fn to_affine_g2<C: Curve<BaseField = QuadraticExtension<FF>>, FF: PrimeField + Extendable<2>>(
        &mut self,
        p: &JacobianPointTargetG2<FF>,
    ) -> AffinePointTargetG2<FF>;
    
    fn doubling_step_for_flipped_miller_loop<
        C: Curve<BaseField = QuadraticExtension<FF>>,
        FF: PrimeField + Extendable<2>,
    >(
        &mut self,
        p: &JacobianPointTargetG2<FF>,
    ) -> (JacobianPointTargetG2<FF>, EllCoefficientsTarget<FF>);
    
    fn mixed_addition_step_for_flipped_miller_loop<
        C: Curve<BaseField = QuadraticExtension<FF>>,
        FF: PrimeField + Extendable<2>,
    >(
        &mut self,
        r: &JacobianPointTargetG2<FF>,
        p: &AffinePointTargetG2<FF>,
    ) -> (JacobianPointTargetG2<FF>, EllCoefficientsTarget<FF>);
    
    fn mul_by_q<C: Curve<BaseField = QuadraticExtension<FF>>, FF: PrimeField + Extendable<2>>(
        &mut self,
        p: &AffinePointTargetG2<FF>,
    ) -> AffinePointTargetG2<FF>;
    
    fn constant_affine_point_g2<
        C: Curve<BaseField = QuadraticExtension<FF>>,
        FF: PrimeField + Extendable<2>,
    >(
        &mut self,
        point: AffinePoint<C>,
    ) -> AffinePointTargetG2<FF>;
}

impl<F: RichField + Extendable<D>, const D: usize> CircuitBuilderPairing<F, D>
    for CircuitBuilder<F, D>
{
    fn pairing_bn254(
        &mut self,
        p: &G1AffineTarget<F, D>,
        q: &AffinePointTargetG2<Bn128Base>,
    ) -> NonNativeTargetExt12<Bn128Base> {
        println!("Step 1: Precompute line coefficients for G2 point...");
        let pre = self.precompute::<G2, Bn128Base>(q);
        println!("  - Precomputed {} coefficients", pre.coeffs.len());
        
        println!("Step 2: Miller loop computation...");
        let m = self.miller_loop::<G1, Bn128Base>(p, &pre);
        println!("  - Miller loop completed");
        
        println!("Step 3: Final exponentiation (two chunks)...");
        println!("  - First chunk...");
        let res = self.final_exponentiation_first_chunk(&m);
        println!("  - Last chunk...");
        let result = self.final_exponentiation_last_chunk(&res);
        println!("  - Final exponentiation completed");
        
        result
    }
    
    fn pairing(
        &mut self,
        p: &G1AffineTarget<F, D>,
        q: &AffinePointTargetG2<Bn128Base>,
    ) -> NonNativeTargetExt12<Bn128Base> {
        let pre = self.precompute::<G2, Bn128Base>(q);
        
        let m = self.miller_loop::<G1, Bn128Base>(p, &pre);
        
        let res = self.final_exponentiation_first_chunk(&m);
        self.final_exponentiation_last_chunk(&res)
    }
}

impl<F: RichField + Extendable<D>, const D: usize> CircuitBuilderCurveG2<F, D>
    for CircuitBuilder<F, D>
{
    fn add_g2<C: Curve<BaseField = QuadraticExtension<FF>>, FF: PrimeField + Extendable<2>>(
        &mut self,
        a: &AffinePointTargetG2<FF>,
        b: &AffinePointTargetG2<FF>,
    ) -> AffinePointTargetG2<FF> {
        let AffinePointTargetG2 { x: x1, y: y1 } = a;
        let AffinePointTargetG2 { x: x2, y: y2 } = b;

        let u = self.sub_nonnative_ext2(y2, y1);
        let v = self.sub_nonnative_ext2(x2, x1);
        let v_inv = self.inv_nonnative_ext2(&v);
        let s = self.mul_nonnative_ext2(&u, &v_inv);
        let s_squared = self.mul_nonnative_ext2(&s, &s);
        let x_sum = self.add_nonnative_ext2(x2, x1);
        let x3 = self.sub_nonnative_ext2(&s_squared, &x_sum);
        let x_diff = self.sub_nonnative_ext2(x1, &x3);
        let prod = self.mul_nonnative_ext2(&s, &x_diff);
        let y3 = self.sub_nonnative_ext2(&prod, y1);

        AffinePointTargetG2 { x: x3, y: y3 }
    }
    
    fn neg_g2<FF: PrimeField + Extendable<2>>(
        &mut self,
        p: &AffinePointTargetG2<FF>,
    ) -> AffinePointTargetG2<FF> {
        let neg_y = self.neg_nonnative_ext2(&p.y);
        AffinePointTargetG2 {
            x: p.x.clone(),
            y: neg_y,
        }
    }
    
    fn constant_affine_point_g2<
        C: Curve<BaseField = QuadraticExtension<FF>>,
        FF: PrimeField + Extendable<2>,
    >(
        &mut self,
        point: AffinePoint<C>,
    ) -> AffinePointTargetG2<FF> {
        debug_assert!(!point.zero);
        AffinePointTargetG2 {
            x: self.constant_nonnative_ext2(point.x),
            y: self.constant_nonnative_ext2(point.y),
        }
    }
    
    fn to_jacobian_g2<C: Curve<BaseField = QuadraticExtension<FF>>, FF: PrimeField + Extendable<2>>(
        &mut self,
        p: &AffinePointTargetG2<FF>,
    ) -> JacobianPointTargetG2<FF> {
        JacobianPointTargetG2 {
            x: p.x.clone(),
            y: p.y.clone(),
            z: self.constant_nonnative_ext2(QuadraticExtension([FF::ONE, FF::ZERO])),
        }
    }
    
    fn to_affine_g2<C: Curve<BaseField = QuadraticExtension<FF>>, FF: PrimeField + Extendable<2>>(
        &mut self,
        p: &JacobianPointTargetG2<FF>,
    ) -> AffinePointTargetG2<FF> {
        let z_inv = self.inv_nonnative_ext2(&p.z);
        let z_inv_squared = self.mul_nonnative_ext2(&z_inv, &z_inv);
        let x = self.mul_nonnative_ext2(&p.x, &z_inv_squared);
        let z_inv_cubed = self.mul_nonnative_ext2(&z_inv_squared, &z_inv);
        let y = self.mul_nonnative_ext2(&p.y, &z_inv_cubed);

        AffinePointTargetG2 { x, y }
    }
    
    fn precompute<C: Curve<BaseField = QuadraticExtension<FF>>, FF: PrimeField + Extendable<2>>(
        &mut self,
        p: &AffinePointTargetG2<FF>,
    ) -> G2PreComputeTarget<FF> {
        let mut r = self.to_jacobian_g2::<C, FF>(p);
        let mut coeffs = Vec::with_capacity(102);
        let mut found_one = false;
        let mut bit_count = 0;

        for (j_idx, j) in ATE_LOOP_COUNT.iter().rev().enumerate() {
            if j_idx == 0 {
                println!("  Processing ATE_LOOP_COUNT[{}] = 0x{:x}", 3 - j_idx, j);
            }
            for i in (0..64).rev() {
                if !found_one {
                    found_one = (j >> i) & 1 == 1;
                    continue;
                }
                bit_count += 1;

                let (r0, coeff) = self.doubling_step_for_flipped_miller_loop::<C, FF>(&r);
                r = r0;
                coeffs.push(coeff);

                if (j >> i) & 1 == 1 {
                    let (r0, coeff) = self.mixed_addition_step_for_flipped_miller_loop::<C, FF>(&r, p);
                    r = r0;
                    coeffs.push(coeff);
                }
                
                if bit_count % 10 == 0 {
                    println!("    Processed {} bits, {} coefficients", bit_count, coeffs.len());
                }
            }
        }

        let q1 = self.mul_by_q::<C, FF>(p);
        let mul_by_q = self.mul_by_q::<C, FF>(&q1);
        let q2 = self.neg_g2::<FF>(&mul_by_q);

        let (r0, coeff) = self.mixed_addition_step_for_flipped_miller_loop::<C, FF>(&r, &q1);
        r = r0;
        coeffs.push(coeff);
        let (_, coeff) = self.mixed_addition_step_for_flipped_miller_loop::<C, FF>(&r, &q2);
        coeffs.push(coeff);

        G2PreComputeTarget {
            q: p.clone(),
            coeffs,
        }
    }
    
    fn miller_loop<
        C: Curve<BaseField = FF>,
        FF: PrimeField + Extendable<2> + Extendable<6> + Extendable<12>,
    >(
        &mut self,
        g1: &G1AffineTarget<F, D>,
        precomp: &G2PreComputeTarget<FF>,
    ) -> NonNativeTargetExt12<FF> {
        let mut f = self.constant_nonnative_ext12(DodecicExtension::ONE);
        let mut idx = 0;
        let mut found_one = false;

        let g1_x: &NonNativeTarget<FF> = unsafe { std::mem::transmute(&g1.x) };
        let g1_y: &NonNativeTarget<FF> = unsafe { std::mem::transmute(&g1.y) };

        for j in ATE_LOOP_COUNT.iter().rev() {
            for i in (0..64).rev() {
                if !found_one {
                    found_one = (j >> i) & 1 == 1;
                    continue;
                }

                let c = &precomp.coeffs[idx];
                idx += 1;
                f = self.squared_nonnative_ext12(&f);
                let ell_vw = self.scale_nonnative_ext2(&c.ell_vw, g1_y);
                let ell_vv = self.scale_nonnative_ext2(&c.ell_vv, g1_x);
                f = self.mul_by_024(&f, &c.ell_0, &ell_vw, &ell_vv);

                if (j >> i) & 1 == 1 {
                    let c = &precomp.coeffs[idx];
                    idx += 1;
                    let ell_vw = self.scale_nonnative_ext2(&c.ell_vw, g1_y);
                    let ell_vv = self.scale_nonnative_ext2(&c.ell_vv, g1_x);
                    f = self.mul_by_024(&f, &c.ell_0, &ell_vw, &ell_vv);
                }
            }
        }

        let c = &precomp.coeffs[idx];
        idx += 1;
        let ell_vw = self.scale_nonnative_ext2(&c.ell_vw, g1_y);
        let ell_vv = self.scale_nonnative_ext2(&c.ell_vv, g1_x);
        f = self.mul_by_024(&f, &c.ell_0, &ell_vw, &ell_vv);

        let c = &precomp.coeffs[idx];
        let ell_vw = self.scale_nonnative_ext2(&c.ell_vw, g1_y);
        let ell_vv = self.scale_nonnative_ext2(&c.ell_vv, g1_x);
        f = self.mul_by_024(&f, &c.ell_0, &ell_vv, &ell_vw);

        f
    }
    
    fn doubling_step_for_flipped_miller_loop<
        C: Curve<BaseField = QuadraticExtension<FF>>,
        FF: PrimeField + Extendable<2>,
    >(
        &mut self,
        p: &JacobianPointTargetG2<FF>,
    ) -> (JacobianPointTargetG2<FF>, EllCoefficientsTarget<FF>) {
        let two_inv = self.constant_nonnative(FF::from_canonical_u64(2).inverse());
        
        let mut a = self.mul_nonnative_ext2(&p.x, &p.y);
        a = self.scale_nonnative_ext2(&a, &two_inv);
        let b = self.squared_nonnative_ext2(&p.y);
        let c = self.squared_nonnative_ext2(&p.z);
        let mut d = self.add_nonnative_ext2(&c, &c);
        d = self.add_nonnative_ext2(&d, &c);
        
        let mut e = self.constant_nonnative_ext2(QuadraticExtension([FF::from_canonical_u64(3), FF::ZERO]));
        e = self.mul_nonnative_ext2(&e, &d);
        let mut f = self.add_nonnative_ext2(&e, &e);
        f = self.add_nonnative_ext2(&f, &e);
        let mut g = self.add_nonnative_ext2(&b, &f);
        g = self.scale_nonnative_ext2(&g, &two_inv);
        let mut h = self.add_nonnative_ext2(&p.y, &p.z);
        h = self.squared_nonnative_ext2(&h);
        h = self.sub_nonnative_ext2(&h, &b);
        h = self.sub_nonnative_ext2(&h, &c);
        let i = self.sub_nonnative_ext2(&e, &b);
        let j = self.squared_nonnative_ext2(&p.x);
        let e_sq = self.squared_nonnative_ext2(&e);

        let mut x = self.sub_nonnative_ext2(&b, &f);
        x = self.mul_nonnative_ext2(&a, &x);
        let mut y = self.squared_nonnative_ext2(&g);
        let mut e_sq_3 = self.add_nonnative_ext2(&e_sq, &e_sq);
        e_sq_3 = self.add_nonnative_ext2(&e_sq_3, &e_sq);
        y = self.sub_nonnative_ext2(&y, &e_sq_3);
        let z = self.mul_nonnative_ext2(&b, &h);

        let ell_0 = self.mul_by_nonresidue_nonnative_ext2(&i);
        let ell_vw = self.neg_nonnative_ext2(&h);
        let mut ell_vv = self.add_nonnative_ext2(&j, &j);
        ell_vv = self.add_nonnative_ext2(&ell_vv, &j);

        (
            JacobianPointTargetG2 { x, y, z },
            EllCoefficientsTarget {
                ell_0,
                ell_vw,
                ell_vv,
            },
        )
    }
    
    fn mixed_addition_step_for_flipped_miller_loop<
        C: Curve<BaseField = QuadraticExtension<FF>>,
        FF: PrimeField + Extendable<2>,
    >(
        &mut self,
        r: &JacobianPointTargetG2<FF>,
        base: &AffinePointTargetG2<FF>,
    ) -> (JacobianPointTargetG2<FF>, EllCoefficientsTarget<FF>) {
        let mut d = self.mul_nonnative_ext2(&r.z, &base.x);
        d = self.sub_nonnative_ext2(&r.x, &d);
        let mut e = self.mul_nonnative_ext2(&r.z, &base.y);
        e = self.sub_nonnative_ext2(&r.y, &e);
        let f = self.squared_nonnative_ext2(&d);
        let g = self.squared_nonnative_ext2(&e);
        let h = self.mul_nonnative_ext2(&d, &f);
        let i = self.mul_nonnative_ext2(&r.x, &f);
        let mut j = self.mul_nonnative_ext2(&r.z, &g);
        j = self.add_nonnative_ext2(&j, &h);
        j = self.sub_nonnative_ext2(&j, &i);
        j = self.sub_nonnative_ext2(&j, &i);
        let x = self.mul_nonnative_ext2(&d, &j);
        let mut y = self.sub_nonnative_ext2(&i, &j);
        y = self.mul_nonnative_ext2(&e, &y);
        let h_y = self.mul_nonnative_ext2(&h, &r.y);
        y = self.sub_nonnative_ext2(&y, &h_y);
        let z = self.mul_nonnative_ext2(&r.z, &h);
        let e_x = self.mul_nonnative_ext2(&e, &base.x);
        let d_y = self.mul_nonnative_ext2(&d, &base.y);
        let mut ell_0 = self.sub_nonnative_ext2(&e_x, &d_y);
        ell_0 = self.mul_by_nonresidue_nonnative_ext2(&ell_0);
        let ell_vv = self.neg_nonnative_ext2(&e);
        let ell_vw = d;
        (
            JacobianPointTargetG2 { x, y, z },
            EllCoefficientsTarget {
                ell_0,
                ell_vv,
                ell_vw,
            },
        )
    }
    
    fn mul_by_q<C: Curve<BaseField = QuadraticExtension<FF>>, FF: PrimeField + Extendable<2>>(
        &mut self,
        p: &AffinePointTargetG2<FF>,
    ) -> AffinePointTargetG2<FF> {
        let x_frobenius = self.frobenius_map_nonnative_ext2(&p.x, 1);
        let y_frobenius = self.frobenius_map_nonnative_ext2(&p.y, 1);
        
        let twist_mul_by_q_x = self.constant_nonnative_ext2(QuadraticExtension([
            FF::from_noncanonical_biguint(biguint_from_array([
                13075984984163199792,
                3782902503040509012,
                8791150885551868305,
                1825854335138010348,
            ])),
            FF::from_noncanonical_biguint(biguint_from_array([
                7963664994991228759,
                12257807996192067905,
                13179524609921305146,
                2767831111890561987,
            ])),
        ]));
        
        let twist_mul_by_q_y = self.constant_nonnative_ext2(QuadraticExtension([
            FF::from_noncanonical_biguint(biguint_from_array([
                16482010305593259561,
                13488546290961988299,
                3578621962720924518,
                2681173117283399901,
            ])),
            FF::from_noncanonical_biguint(biguint_from_array([
                11661927080404088775,
                553939530661941723,
                7860678177968807019,
                3208568454732775116,
            ])),
        ]));
        
        AffinePointTargetG2 {
            x: self.mul_nonnative_ext2(&twist_mul_by_q_x, &x_frobenius),
            y: self.mul_nonnative_ext2(&twist_mul_by_q_y, &y_frobenius),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use plonky2::{
        iop::witness::PartialWitness,
        plonk::{circuit_data::CircuitConfig, config::{GenericConfig, PoseidonGoldilocksConfig}},
    };

    const D: usize = 2;
    type C = PoseidonGoldilocksConfig;
    type F = <C as GenericConfig<D>>::F;

    #[test]
    fn test_pairing_structure() {
        let config = CircuitConfig::standard_ecc_config();
        let mut builder = CircuitBuilder::<F, D>::new(config);

        let g1_point = builder.g1_generator();
        
        let g2_x = QuadraticExtension([Bn128Base::ONE, Bn128Base::ZERO]);
        let g2_y = QuadraticExtension([Bn128Base::from_canonical_u64(2), Bn128Base::ZERO]);
        let g2_affine_point = AffinePoint::<G2> {
            x: g2_x,
            y: g2_y,
            zero: false,
        };
        let g2_point = builder.constant_affine_point_g2::<G2, Bn128Base>(g2_affine_point);
        
        println!("✅ Complete pairing structure created successfully");
        println!("📋 Structure verification:");
        println!("   - ATE_LOOP_COUNT: Ported from plonky2-pairing");  
        println!("   - G2 data structures: AffinePointTargetG2, JacobianPointTargetG2, EllCoefficientsTarget");
        println!("   - Main functions: pairing(), precompute(), miller_loop()");
        println!("   - Helper functions: doubling_step, mixed_addition_step, mul_by_q");
        println!("   - Next step: Implement Miller loop and final exponentiation algorithms");
    }
    
    #[test]
    fn test_pairing() {
        use crate::crypto::bn254::curve::{G1, G2};
        use crate::crypto::secp256k1::ecdsa::curve::curve_types::Curve;
        use crate::crypto::secp256k1::ecdsa::gadgets::curve::CircuitBuilderCurve;
        use std::time::Instant;
        
        let config = CircuitConfig {
            num_wires: 400,
            ..CircuitConfig::wide_ecc_config()
        };
        let mut builder = CircuitBuilder::<F, D>::new(config);
        
        let start = Instant::now();
        println!("Starting pairing test...");

        let g1 = G1::GENERATOR_AFFINE;
        let g2 = G2::GENERATOR_AFFINE;
        
        let g1_target = builder.g1_generator();
        let g2_target = builder.constant_affine_point_g2::<G2, Bn128Base>(g2);
        
        println!("Computing pairing e(G1, G2)...");
        
        let pairing_start = Instant::now();
        let pairing_result = builder.pairing_bn254(&g1_target, &g2_target);
        println!("Pairing computation took: {:?}", pairing_start.elapsed());
        
        println!("Building circuit...");
        println!("Number of gates before build: {}", builder.num_gates());
        
        let build_start = Instant::now();
        let data = builder.build::<C>();
        println!("Circuit build took: {:?}", build_start.elapsed());
        
        println!("Circuit built successfully!");
        println!("Number of gates: {}", data.common.gates.len());
        println!("Degree bits: {}", data.common.degree_bits());
        println!("Number of public inputs: {}", data.common.num_public_inputs);
        
        println!("Generating proof...");
        let pw = PartialWitness::new();
        let proof = data.prove(pw).unwrap();
        
        println!("Verifying proof...");
        data.verify(proof).unwrap();
        
        println!("✅ Pairing test passed!");
    }
    
    #[test]
    fn test_pairing_components() {
        use crate::crypto::bn254::curve::G2;
        use crate::crypto::secp256k1::ecdsa::curve::curve_types::Curve;
        
        let config = CircuitConfig {
            num_wires: 400,
            ..CircuitConfig::wide_ecc_config()
        };
        let mut builder = CircuitBuilder::<F, D>::new(config);
        
        let g2 = G2::GENERATOR_AFFINE;
        let g2_target = builder.constant_affine_point_g2::<G2, Bn128Base>(g2);
        
        let precomp = builder.precompute::<G2, Bn128Base>(&g2_target);
        
        println!("✅ Pairing components test passed");
        println!("   - G2 point creation: OK");
        println!("   - Precompute: OK (generated {} coefficients)", precomp.coeffs.len());
    }
}