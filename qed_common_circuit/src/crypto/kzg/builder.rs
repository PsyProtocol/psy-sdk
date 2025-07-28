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
        windowed_mul::CircuitBuilderWindowedMul,
    },
};

use std::marker::PhantomData;

use crate::crypto::secp256k1::ecdsa::curve::curve_types::{AffinePoint, Curve};
use crate::crypto::secp256k1::ecdsa::gadgets::curve::{AffinePointTarget, CircuitBuilderCurve};

use super::{
    commitment::KZGCommitmentTarget,
    proof::KZGProofTarget,
    fft::{CircuitBuilderFFT, FFTSettingsTarget},
};

use crate::crypto::bn254::gadgets::biguint::BigUintTarget;

pub trait CircuitBuilderKZG<F: RichField + Extendable<D>, const D: usize> {

    fn kzg_commit(
        &mut self,
        coefficients: &[NonNativeTarget<Bn128Scalar>],
        powers_of_tau: &[G1AffineTarget<F, D>],
    ) -> KZGCommitmentTarget<F, D>;

    fn kzg_commit_lagrange(
        &mut self,
        evaluations: &[NonNativeTarget<Bn128Scalar>],
        lagrange_powers: &[G1AffineTarget<F, D>],
    ) -> KZGCommitmentTarget<F, D>;


    fn kzg_create_opening_proof(
        &mut self,
        coefficients: &[NonNativeTarget<Bn128Scalar>],
        point: &NonNativeTarget<Bn128Scalar>,
        powers_of_tau: &[G1AffineTarget<F, D>],
    ) -> (NonNativeTarget<Bn128Scalar>, KZGProofTarget<F, D>);

    fn kzg_create_batch_opening_proofs(
        &mut self,
        coefficients: &[Vec<NonNativeTarget<Bn128Scalar>>],
        points: &[NonNativeTarget<Bn128Scalar>],
        powers_of_tau: &[G1AffineTarget<F, D>],
    ) -> (Vec<NonNativeTarget<Bn128Scalar>>, Vec<KZGProofTarget<F, D>>);


    fn kzg_verify(
        &mut self,
        commitment: &KZGCommitmentTarget<F, D>,
        point: &NonNativeTarget<Bn128Scalar>,
        evaluation: &NonNativeTarget<Bn128Scalar>,
        proof: &KZGProofTarget<F, D>,
        g2_tau: &AffinePointTargetG2<Bn128Base>,
    ) -> BoolTarget;

    fn kzg_batch_verify(
        &mut self,
        commitments: &[KZGCommitmentTarget<F, D>],
        points: &[NonNativeTarget<Bn128Scalar>],
        evaluations: &[NonNativeTarget<Bn128Scalar>],
        proofs: &[KZGProofTarget<F, D>],
        g2_tau: &AffinePointTargetG2<Bn128Base>,
    ) -> BoolTarget;

    fn kzg_compute_batch_challenge(
        &mut self,
        commitments: &[KZGCommitmentTarget<F, D>],
        points: &[NonNativeTarget<Bn128Scalar>],
        evaluations: &[NonNativeTarget<Bn128Scalar>],
        proofs: &[KZGProofTarget<F, D>],
    ) -> NonNativeTarget<Bn128Scalar>;

    fn kzg_batch_inverse(
        &mut self,
        elements: &[NonNativeTarget<Bn128Scalar>],
    ) -> Vec<NonNativeTarget<Bn128Scalar>>;
    
    fn g1_infinity(&mut self) -> G1AffineTarget<F, D>;


    fn kzg_evaluate_polynomial(
        &mut self,
        coefficients: &[NonNativeTarget<Bn128Scalar>],
        point: &NonNativeTarget<Bn128Scalar>,
    ) -> NonNativeTarget<Bn128Scalar>;

    fn kzg_compute_quotient_polynomial(
        &mut self,
        coefficients: &[NonNativeTarget<Bn128Scalar>],
        point: &NonNativeTarget<Bn128Scalar>,
        evaluation: &NonNativeTarget<Bn128Scalar>,
    ) -> Vec<NonNativeTarget<Bn128Scalar>>;
    
    
    fn kzg_commit_lagrange_basis(
        &mut self,
        evaluations: &[NonNativeTarget<Bn128Scalar>],
        lagrange_basis_g1: &[G1AffineTarget<F, D>],
    ) -> KZGCommitmentTarget<F, D>;
    
    fn kzg_create_opening_proof_lagrange(
        &mut self,
        evaluations: &[NonNativeTarget<Bn128Scalar>],
        point: &NonNativeTarget<Bn128Scalar>,
        lagrange_basis_g1: &[G1AffineTarget<F, D>],
        fft_settings: &FFTSettingsTarget<F, D>,
    ) -> (NonNativeTarget<Bn128Scalar>, KZGProofTarget<F, D>);
    
    fn kzg_compute_quotient_polynomial_lagrange(
        &mut self,
        evaluations: &[NonNativeTarget<Bn128Scalar>],
        point: &NonNativeTarget<Bn128Scalar>,
        eval_at_point: &NonNativeTarget<Bn128Scalar>,
        fft_settings: &FFTSettingsTarget<F, D>,
    ) -> Vec<NonNativeTarget<Bn128Scalar>>;
}

impl<F: RichField + Extendable<D>, const D: usize> CircuitBuilderKZG<F, D> for CircuitBuilder<F, D> {

    fn kzg_commit(
        &mut self,
        coefficients: &[NonNativeTarget<Bn128Scalar>],
        powers_of_tau: &[G1AffineTarget<F, D>],
    ) -> KZGCommitmentTarget<F, D> {
        assert_eq!(coefficients.len(), powers_of_tau.len(),
            "Coefficients and powers of tau must have the same length");
        assert!(!coefficients.is_empty(), "Cannot commit to empty polynomial");

        
        
        let commitment = self.g1_msm(powers_of_tau, coefficients);

        KZGCommitmentTarget { commitment }
    }

    fn kzg_commit_lagrange(
        &mut self,
        evaluations: &[NonNativeTarget<Bn128Scalar>],
        lagrange_powers: &[G1AffineTarget<F, D>],
    ) -> KZGCommitmentTarget<F, D> {
        assert_eq!(evaluations.len(), lagrange_powers.len(),
            "Evaluations and Lagrange powers must have the same length");

        self.kzg_commit(evaluations, lagrange_powers)
    }


    fn kzg_create_opening_proof(
        &mut self,
        coefficients: &[NonNativeTarget<Bn128Scalar>],
        point: &NonNativeTarget<Bn128Scalar>,
        powers_of_tau: &[G1AffineTarget<F, D>],
    ) -> (NonNativeTarget<Bn128Scalar>, KZGProofTarget<F, D>) {
        
        let evaluation = self.kzg_evaluate_polynomial(coefficients, point);

        
        let quotient_coeffs = self.kzg_compute_quotient_polynomial(coefficients, point, &evaluation);

        
        let proof_commitment = if quotient_coeffs.is_empty() {
            self.g1_generator()
        } else {
            
            if quotient_coeffs.len() > powers_of_tau.len() {
                panic!("Not enough SRS powers: need {}, have {}", quotient_coeffs.len(), powers_of_tau.len());
            }

            
            let commitment = self.kzg_commit(&quotient_coeffs, &powers_of_tau[..quotient_coeffs.len()]);
            commitment.commitment
        };

        (evaluation, KZGProofTarget { w: proof_commitment })
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


    fn kzg_verify(
        &mut self,
        commitment: &KZGCommitmentTarget<F, D>,
        point: &NonNativeTarget<Bn128Scalar>,
        evaluation: &NonNativeTarget<Bn128Scalar>,
        proof: &KZGProofTarget<F, D>,
        g2_tau: &AffinePointTargetG2<Bn128Base>,
    ) -> BoolTarget {
        
        let g1_gen = self.g1_generator();
        let g2_gen = self.constant_affine_point_g2::<G2, Bn128Base>(G2::GENERATOR_AFFINE);

        
        let y_g = self.scalar_mul_g1(&g1_gen, evaluation);
        
        
        let x_equal = self.is_equal_nonnative(&commitment.commitment.x, &y_g.x);
        let y_equal = self.is_equal_nonnative(&commitment.commitment.y, &y_g.y);
        let points_equal = self.and(x_equal, y_equal);
        
        
        let neg_y_g = self.neg_g1_affine(&y_g);
        let c_minus_yg_normal = self.add_or_double_g1_affine(&commitment.commitment, &neg_y_g);
        
        
        let infinity = self.g1_infinity();
        let c_minus_yg = self.select_g1(points_equal, &infinity, &c_minus_yg_normal);
        
        
        let left_g1 = c_minus_yg;

        let g2_gen_windowed = G2AffineTarget {
            x: g2_gen.x.clone(),
            y: g2_gen.y.clone(),
            is_infinity: self._false(),
            _phantom: PhantomData,
        };
        let z_h = self.curve_scalar_mul_windowed_g2(&g2_gen_windowed, point);

        let z_h_pairing = AffinePointTargetG2 {
            x: z_h.x.clone(),
            y: z_h.y.clone(),
        };

        use crate::crypto::bn254::gadgets::pairing::CircuitBuilderCurveG2;
        let neg_z_h = CircuitBuilderCurveG2::neg_g2(self, &z_h_pairing);
        let tau_minus_z_h = CircuitBuilderCurveG2::add_g2::<G2, Bn128Base>(self, g2_tau, &neg_z_h);
        
        
        let right_g2 = tau_minus_z_h;

        
        let left_pairing = self.pairing_bn254(&left_g1, &g2_gen);
        let right_pairing = self.pairing_bn254(&proof.w, &right_g2);
        
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
        use crate::crypto::bn254::gadgets::biguint::CircuitBuilderBiguint;
        use num::Zero;
        
        assert_eq!(commitments.len(), points.len());
        assert_eq!(commitments.len(), evaluations.len());
        assert_eq!(commitments.len(), proofs.len());

        if commitments.is_empty() {
            return self._true();
        }

        
        if commitments.len() == 1 {
            return self.kzg_verify(
                &commitments[0],
                &points[0],
                &evaluations[0],
                &proofs[0],
                g2_tau,
            );
        }

        
        let challenge = self.kzg_compute_batch_challenge(commitments, points, evaluations, proofs);
        
        
        let mut r_powers = vec![self.one_nonnative()];
        for i in 1..commitments.len() {
            let prev = &r_powers[i-1];
            let next = self.mul_nonnative(prev, &challenge);
            r_powers.push(next);
        }

        
        let g1_gen = self.g1_generator();
        let mut left_acc = self.g1_infinity();

        for i in 0..commitments.len() {
            let y_g = self.scalar_mul_g1(&g1_gen, &evaluations[i]);
            let neg_y_g = self.neg_g1_affine(&y_g);
            let c_minus_yg = self.add_or_double_g1_affine(&commitments[i].commitment, &neg_y_g);
            let scaled = self.scalar_mul_g1(&c_minus_yg, &r_powers[i]);
            left_acc = self.add_or_double_g1_affine(&left_acc, &scaled);
        }

        
        let mut right_acc = self.g1_infinity();
        
        for i in 0..proofs.len() {
            let scaled = self.scalar_mul_g1(&proofs[i].w, &r_powers[i]);
            right_acc = self.add_or_double_g1_affine(&right_acc, &scaled);
        }

        
        let g2_gen = self.constant_affine_point_g2::<G2, Bn128Base>(G2::GENERATOR_AFFINE);
        
        let modulus = self.constant_biguint(&Bn128Scalar::order());
        let mut z_acc_biguint = self.constant_biguint(&num::BigUint::zero());
        
        for i in 0..points.len() {
            let r_biguint = self.nonnative_to_canonical_biguint(&r_powers[i]);
            let z_biguint = self.nonnative_to_canonical_biguint(&points[i]);
            let product = self.mul_biguint(&r_biguint, &z_biguint);
            let reduced = self.rem_biguint(&product, &modulus);
            
            let sum = self.add_biguint(&z_acc_biguint, &reduced);
            z_acc_biguint = self.rem_biguint(&sum, &modulus);
        }
        
        let z_acc = self.biguint_to_nonnative(&z_acc_biguint);
        
        let g2_gen_windowed = G2AffineTarget {
            x: g2_gen.x.clone(),
            y: g2_gen.y.clone(),
            is_infinity: self._false(),
            _phantom: PhantomData,
        };
        let z_h_windowed = self.curve_scalar_mul_windowed_g2(&g2_gen_windowed, &z_acc);
        
        let z_h = AffinePointTargetG2 {
            x: z_h_windowed.x.clone(),
            y: z_h_windowed.y.clone(),
        };
        
        let neg_z_h = CircuitBuilderCurveG2::neg_g2(self, &z_h);
        let right_g2 = CircuitBuilderCurveG2::add_g2::<G2, Bn128Base>(self, g2_tau, &neg_z_h);

        let left_pairing = self.pairing_bn254(&left_acc, &g2_gen);
        let right_pairing = self.pairing_bn254(&right_acc, &right_g2);

        self.is_equal_ext12(&left_pairing, &right_pairing)
    }


    fn kzg_evaluate_polynomial(
        &mut self,
        coefficients: &[NonNativeTarget<Bn128Scalar>],
        point: &NonNativeTarget<Bn128Scalar>,
    ) -> NonNativeTarget<Bn128Scalar> {
        use crate::crypto::bn254::gadgets::biguint::CircuitBuilderBiguint;

        if coefficients.is_empty() {
            return self.constant_nonnative(Bn128Scalar::ZERO);
        }

        if coefficients.len() == 1 {
            return coefficients[0].clone();
        }


        let modulus = self.constant_biguint(&Bn128Scalar::order());
        let point_biguint = self.nonnative_to_canonical_biguint(point);

        let mut result_biguint = self.nonnative_to_canonical_biguint(&coefficients[coefficients.len() - 1]);

        for i in (0..coefficients.len() - 1).rev() {
            let product = self.mul_biguint(&result_biguint, &point_biguint);
            let reduced = self.rem_biguint(&product, &modulus);

            let coeff_biguint = self.nonnative_to_canonical_biguint(&coefficients[i]);
            let sum = self.add_biguint(&reduced, &coeff_biguint);
            result_biguint = self.rem_biguint(&sum, &modulus);
        }

        self.biguint_to_nonnative(&result_biguint)
    }

    fn kzg_compute_quotient_polynomial(
        &mut self,
        coefficients: &[NonNativeTarget<Bn128Scalar>],
        point: &NonNativeTarget<Bn128Scalar>,
        evaluation: &NonNativeTarget<Bn128Scalar>,
    ) -> Vec<NonNativeTarget<Bn128Scalar>> {
        
        if coefficients.is_empty() {
            return vec![];
        }
        
        if coefficients.len() == 1 {
            return vec![];
        }
        
        let n = coefficients.len();
        
        let mut adjusted_coeffs = coefficients.to_vec();
        
        use crate::crypto::bn254::gadgets::biguint::CircuitBuilderBiguint;
        let modulus = self.constant_biguint(&Bn128Scalar::order());
        let a0_biguint = self.nonnative_to_canonical_biguint(&adjusted_coeffs[0]);
        let eval_biguint = self.nonnative_to_canonical_biguint(evaluation);
        
        let a0_plus_p = self.add_biguint(&a0_biguint, &modulus);
        let diff_plus_p = self.sub_biguint(&a0_plus_p, &eval_biguint);
        let diff_biguint = self.rem_biguint(&diff_plus_p, &modulus);
        
        adjusted_coeffs[0] = self.biguint_to_nonnative(&diff_biguint);
        
        let mut quotient = Vec::with_capacity(n - 1);
        
        quotient.push(adjusted_coeffs[n-1].clone());
        
        for i in (1..n-1).rev() {
            let z_biguint = self.nonnative_to_canonical_biguint(point);
            let prev_biguint = self.nonnative_to_canonical_biguint(&quotient[quotient.len()-1]);
            let product = self.mul_biguint(&z_biguint, &prev_biguint);
            let z_times_prev_reduced = self.rem_biguint(&product, &modulus);
            let z_times_prev = self.biguint_to_nonnative(&z_times_prev_reduced);
            
            let next_coeff = self.add_nonnative(&adjusted_coeffs[i], &z_times_prev);
            quotient.push(next_coeff);
        }
        
        quotient.reverse();
        
        
        quotient
    }

    fn kzg_compute_batch_challenge(
        &mut self,
        commitments: &[KZGCommitmentTarget<F, D>],
        points: &[NonNativeTarget<Bn128Scalar>],
        evaluations: &[NonNativeTarget<Bn128Scalar>],
        proofs: &[KZGProofTarget<F, D>],
    ) -> NonNativeTarget<Bn128Scalar> {
        use crate::crypto::bn254::gadgets::biguint::{CircuitBuilderBiguint, BigUintTarget};
        use crate::u32::gadgets::arithmetic_u32::{CircuitBuilderU32, U32Target};
        use plonky2::hash::poseidon::PoseidonHash;
        
        let mut inputs = Vec::new();
        
        inputs.push(self.constant(F::from_canonical_usize(commitments.len())));
        
        for i in 0..commitments.len() {
            for limb in &commitments[i].commitment.x.value.limbs {
                inputs.push(limb.0);
            }
            for limb in &commitments[i].commitment.y.value.limbs {
                inputs.push(limb.0);
            }
            
            for limb in &points[i].value.limbs {
                inputs.push(limb.0);
            }
            
            for limb in &evaluations[i].value.limbs {
                inputs.push(limb.0);
            }
            
            for limb in &proofs[i].w.x.value.limbs {
                inputs.push(limb.0);
            }
            for limb in &proofs[i].w.y.value.limbs {
                inputs.push(limb.0);
            }
        }
        
        let hash = self.hash_n_to_hash_no_pad::<PoseidonHash>(inputs);
        
        let mut u32_limbs = Vec::new();
        for i in 0..hash.elements.len().min(8) {
            u32_limbs.push(U32Target(hash.elements[i]));
        }
        
        while u32_limbs.len() < 8 {
            u32_limbs.push(self.zero_u32());
        }
        
        let biguint = BigUintTarget { limbs: u32_limbs };
        let modulus = self.constant_biguint(&Bn128Scalar::order());
        let reduced = self.rem_biguint(&biguint, &modulus);
        
        self.biguint_to_nonnative(&reduced)
    }

    fn kzg_batch_inverse(
        &mut self,
        elements: &[NonNativeTarget<Bn128Scalar>],
    ) -> Vec<NonNativeTarget<Bn128Scalar>> {
        if elements.is_empty() {
            return vec![];
        }
        
        let n = elements.len();
        
        let mut acc = vec![elements[0].clone()];
        for i in 1..n {
            let product = self.mul_nonnative(&acc[i-1], &elements[i]);
            acc.push(product);
        }
        
        let inv_acc = self.inv_nonnative(&acc[n-1]);
        
        let mut inverses = vec![self.zero_nonnative(); n];
        
        if n > 1 {
            inverses[n-1] = self.mul_nonnative(&acc[n-2], &inv_acc);
        } else {
            inverses[0] = inv_acc.clone();
        }
        
        let mut running_inv = inv_acc;
        for i in (0..n-1).rev() {
            if i > 0 {
                inverses[i] = self.mul_nonnative(&acc[i-1], &running_inv);
            } else {
                inverses[i] = running_inv.clone();
            }
            running_inv = self.mul_nonnative(&running_inv, &elements[i+1]);
        }
        
        inverses
    }
    
    fn g1_infinity(&mut self) -> G1AffineTarget<F, D> {
        let zero = self.zero_nonnative();
        let one = self.one_nonnative();
        G1AffineTarget {
            x: zero,
            y: one,  // (0, 1) represents point at infinity in affine coordinates
            is_infinity: self._true(),
            _phantom: PhantomData,
        }
    }

    
    fn kzg_commit_lagrange_basis(
        &mut self,
        evaluations: &[NonNativeTarget<Bn128Scalar>],
        lagrange_basis_g1: &[G1AffineTarget<F, D>],
    ) -> KZGCommitmentTarget<F, D> {
        assert_eq!(evaluations.len(), lagrange_basis_g1.len(),
            "Evaluations and Lagrange basis must have the same length");
        
        let commitment = self.g1_msm(lagrange_basis_g1, evaluations);
        KZGCommitmentTarget { commitment }
    }
    
    fn kzg_create_opening_proof_lagrange(
        &mut self,
        evaluations: &[NonNativeTarget<Bn128Scalar>],
        point: &NonNativeTarget<Bn128Scalar>,
        lagrange_basis_g1: &[G1AffineTarget<F, D>],
        fft_settings: &FFTSettingsTarget<F, D>,
    ) -> (NonNativeTarget<Bn128Scalar>, KZGProofTarget<F, D>) {
        let eval_at_point = self.lagrange_interpolate_at_point(
            evaluations, 
            point, 
            fft_settings
        );
        
        let quotient_evals = self.kzg_compute_quotient_polynomial_lagrange(
            evaluations,
            point,
            &eval_at_point,
            fft_settings
        );
        
        let proof_commitment = self.kzg_commit_lagrange_basis(
            &quotient_evals,
            lagrange_basis_g1
        );
        
        (eval_at_point, KZGProofTarget { w: proof_commitment.commitment })
    }
    
    fn kzg_compute_quotient_polynomial_lagrange(
        &mut self,
        evaluations: &[NonNativeTarget<Bn128Scalar>],
        point: &NonNativeTarget<Bn128Scalar>,
        eval_at_point: &NonNativeTarget<Bn128Scalar>,
        fft_settings: &FFTSettingsTarget<F, D>,
    ) -> Vec<NonNativeTarget<Bn128Scalar>> {
        let n = evaluations.len();
        assert_eq!(n, fft_settings.domain_size);
        
        let mut quotient_evals = Vec::with_capacity(n);
        let mut denominators = Vec::with_capacity(n);
        
        let mut special_indices: Vec<usize> = Vec::new();
        let mut is_special = Vec::with_capacity(n);
        
        for i in 0..n {
            let is_equal = self.is_equal_nonnative(point, &fft_settings.roots_of_unity[i]);
            is_special.push(is_equal);
            
            let numerator = self.sub_nonnative(&evaluations[i], eval_at_point);
            let denominator = self.sub_nonnative(&fft_settings.roots_of_unity[i], point);
            
            denominators.push(denominator);
            quotient_evals.push(numerator);
        }
        
        let inverses = self.kzg_batch_inverse(&denominators);
        
        let mut final_quotient_evals = Vec::with_capacity(n);
        for i in 0..n {
            let q_i = self.mul_nonnative(&quotient_evals[i], &inverses[i]);
            
            let zero = self.zero_nonnative();
            let final_q_i = self.select_nonnative(is_special[i], &zero, &q_i);
            
            final_quotient_evals.push(final_q_i);
        }
        
        
        final_quotient_evals
    }

}
