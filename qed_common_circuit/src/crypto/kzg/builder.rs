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
    curve::{g2::G2, G1},
    field::{
        bn128_base::Bn128Base, bn128_scalar::Bn128Scalar, extension::quadratic::QuadraticExtension,
    },
    gadgets::{
        g1::{CircuitBuilderG1, G1AffineTarget},
        g2::{CircuitBuilderG2, G2AffineTarget},
        nonnative_fp::{CircuitBuilderNonNative, NonNativeTarget},
        nonnative_fp12::{CircuitBuilderNonNativeExt12, NonNativeTargetExt12},
        nonnative_fp2::CircuitBuilderNonNativeExt2,
        nonnative_fp6::{CircuitBuilderNonNativeExt6, NonNativeTargetExt6},
        pairing::{AffinePointTargetG2, CircuitBuilderCurveG2, CircuitBuilderPairing},
        windowed_mul::CircuitBuilderWindowedMul,
    },
};

use std::marker::PhantomData;

use crate::crypto::secp256k1::ecdsa::curve::curve_types::{AffinePoint, Curve};
use crate::crypto::secp256k1::ecdsa::gadgets::curve::{AffinePointTarget, CircuitBuilderCurve};

use super::{
    commitment::KZGCommitmentTarget,
    fft::{CircuitBuilderFFT, FFTSettingsTarget},
    proof::KZGProofTarget,
};


pub trait CircuitBuilderKZG<F: RichField + Extendable<D>, const D: usize> {
    /// Commit to polynomial in monomial form
    /// C = Σ(a_i * τ^i * G)
    fn kzg_commit(
        &mut self,
        coefficients: &[NonNativeTarget<Bn128Scalar>],
        powers_of_tau: &[G1AffineTarget<F, D>],
    ) -> KZGCommitmentTarget<F, D>;

    /// Create opening proof for p(z) = y
    /// π = [(p(x) - y)/(x - z)](τ) * G
    fn kzg_create_opening_proof(
        &mut self,
        coefficients: &[NonNativeTarget<Bn128Scalar>],
        point: &NonNativeTarget<Bn128Scalar>,
        powers_of_tau: &[G1AffineTarget<F, D>],
    ) -> (NonNativeTarget<Bn128Scalar>, KZGProofTarget<F, D>);

    /// Verify KZG proof
    /// e(C - yG, H) = e(π, τH - zH)
    fn kzg_verify(
        &mut self,
        commitment: &KZGCommitmentTarget<F, D>,
        point: &NonNativeTarget<Bn128Scalar>,
        evaluation: &NonNativeTarget<Bn128Scalar>,
        proof: &KZGProofTarget<F, D>,
        g2_tau: &AffinePointTargetG2<Bn128Base>,
    ) -> BoolTarget;

    /// Batch verify KZG proofs using random challenge
    /// e(Σ(r^i * (C_i - y_iG)), H) = e(Σ(r^i * π_i), τH - (Σ(r^i * z_i))H)
    fn kzg_batch_verify(
        &mut self,
        commitments: &[KZGCommitmentTarget<F, D>],
        points: &[NonNativeTarget<Bn128Scalar>],
        evaluations: &[NonNativeTarget<Bn128Scalar>],
        proofs: &[KZGProofTarget<F, D>],
        g2_tau: &AffinePointTargetG2<Bn128Base>,
    ) -> BoolTarget;
}

pub trait CircuitBuilderKZGHelpers<F: RichField + Extendable<D>, const D: usize> {
    /// Commit to polynomial in Lagrange form
    /// C = Σ(y_i * L_i(τ) * G)
    fn kzg_commit_lagrange(
        &mut self,
        evaluations: &[NonNativeTarget<Bn128Scalar>],
        lagrange_powers: &[G1AffineTarget<F, D>],
    ) -> KZGCommitmentTarget<F, D>;

    /// Create batch opening proofs
    /// π_i = [(p_i(x) - y_i)/(x - z_i)](τ) * G
    fn kzg_create_batch_opening_proofs(
        &mut self,
        coefficients: &[Vec<NonNativeTarget<Bn128Scalar>>],
        points: &[NonNativeTarget<Bn128Scalar>],
        powers_of_tau: &[G1AffineTarget<F, D>],
    ) -> (Vec<NonNativeTarget<Bn128Scalar>>, Vec<KZGProofTarget<F, D>>);

    /// Compute challenge r = Hash(commitments, points, evaluations, proofs)
    fn kzg_compute_batch_challenge(
        &mut self,
        commitments: &[KZGCommitmentTarget<F, D>],
        points: &[NonNativeTarget<Bn128Scalar>],
        evaluations: &[NonNativeTarget<Bn128Scalar>],
        proofs: &[KZGProofTarget<F, D>],
    ) -> NonNativeTarget<Bn128Scalar>;

    /// Batch inverse: (1/a_0, 1/a_1, ..., 1/a_n)
    fn kzg_batch_inverse(
        &mut self,
        elements: &[NonNativeTarget<Bn128Scalar>],
    ) -> Vec<NonNativeTarget<Bn128Scalar>>;

    /// G1 point at infinity
    fn g1_infinity(&mut self) -> G1AffineTarget<F, D>;

    /// Evaluate polynomial at point
    /// p(z) = Σ(a_i * z^i)
    fn kzg_evaluate_polynomial(
        &mut self,
        coefficients: &[NonNativeTarget<Bn128Scalar>],
        point: &NonNativeTarget<Bn128Scalar>,
    ) -> NonNativeTarget<Bn128Scalar>;

    /// Compute quotient polynomial
    /// q(x) = (p(x) - p(z))/(x - z)
    fn kzg_compute_quotient_polynomial(
        &mut self,
        coefficients: &[NonNativeTarget<Bn128Scalar>],
        point: &NonNativeTarget<Bn128Scalar>,
        evaluation: &NonNativeTarget<Bn128Scalar>,
    ) -> Vec<NonNativeTarget<Bn128Scalar>>;

    /// Create opening proof from Lagrange form
    /// π = [Σ(q_i * L_i(τ))]G where q_i = (y_i - p(z))/(ω^i - z)
    /// p(z) = Σ(y_i * L_i(z)) using barycentric interpolation
    fn kzg_create_opening_proof_lagrange(
        &mut self,
        evaluations: &[NonNativeTarget<Bn128Scalar>],
        point: &NonNativeTarget<Bn128Scalar>,
        lagrange_basis_g1: &[G1AffineTarget<F, D>],
        fft_settings: &FFTSettingsTarget<F, D>,
    ) -> (NonNativeTarget<Bn128Scalar>, KZGProofTarget<F, D>);

    /// Compute quotient in evaluation form
    /// q_i = (y_i - p(z))/(ω^i - z) where p(z) = Σ(y_j * L_j(z))
    /// Special case: q_i = 0 when z = ω^i (using L'Hôpital's rule)
    fn kzg_compute_quotient_polynomial_lagrange(
        &mut self,
        evaluations: &[NonNativeTarget<Bn128Scalar>],
        point: &NonNativeTarget<Bn128Scalar>,
        eval_at_point: &NonNativeTarget<Bn128Scalar>,
        fft_settings: &FFTSettingsTarget<F, D>,
    ) -> Vec<NonNativeTarget<Bn128Scalar>>;
}

impl<F: RichField + Extendable<D>, const D: usize> CircuitBuilderKZG<F, D>
    for CircuitBuilder<F, D>
{
    fn kzg_commit(
        &mut self,
        coefficients: &[NonNativeTarget<Bn128Scalar>],
        powers_of_tau: &[G1AffineTarget<F, D>],
    ) -> KZGCommitmentTarget<F, D> {
        assert_eq!(
            coefficients.len(),
            powers_of_tau.len(),
            "Coefficients and powers of tau must have the same length"
        );
        assert!(
            !coefficients.is_empty(),
            "Cannot commit to empty polynomial"
        );

        // C = Σ(a_i * τ^i * G) = MSM(powers_of_tau, coefficients)
        let commitment = self.g1_msm(powers_of_tau, coefficients);

        KZGCommitmentTarget { commitment }
    }

    fn kzg_create_opening_proof(
        &mut self,
        coefficients: &[NonNativeTarget<Bn128Scalar>],
        point: &NonNativeTarget<Bn128Scalar>,
        powers_of_tau: &[G1AffineTarget<F, D>],
    ) -> (NonNativeTarget<Bn128Scalar>, KZGProofTarget<F, D>) {
        // y = p(z)
        let evaluation = CircuitBuilderKZGHelpers::kzg_evaluate_polynomial(self, coefficients, point);

        // q(x) = (p(x) - y)/(x - z)
        let quotient_coeffs =
            CircuitBuilderKZGHelpers::kzg_compute_quotient_polynomial(self, coefficients, point, &evaluation);

        let proof_commitment = if quotient_coeffs.is_empty() {
            self.g1_generator()
        } else {
            if quotient_coeffs.len() > powers_of_tau.len() {
                panic!(
                    "Not enough SRS powers: need {}, have {}",
                    quotient_coeffs.len(),
                    powers_of_tau.len()
                );
            }

            // π = q(τ) * G
            let commitment =
                self.kzg_commit(&quotient_coeffs, &powers_of_tau[..quotient_coeffs.len()]);
            commitment.commitment
        };

        (
            evaluation,
            KZGProofTarget {
                w: proof_commitment,
            },
        )
    }

    fn kzg_verify(
        &mut self,
        commitment: &KZGCommitmentTarget<F, D>,
        point: &NonNativeTarget<Bn128Scalar>,
        evaluation: &NonNativeTarget<Bn128Scalar>,
        proof: &KZGProofTarget<F, D>,
        g2_tau: &AffinePointTargetG2<Bn128Base>,
    ) -> BoolTarget {
        // Verify: e(C - yG, H) = e(π, τH - zH)
        let g1_gen = self.g1_generator();
        let g2_gen = self.constant_affine_point_g2::<G2, Bn128Base>(G2::GENERATOR_AFFINE);

        // Compute yG
        let y_g = self.scalar_mul_g1(&g1_gen, evaluation);

        let x_equal = self.is_equal_nonnative(&commitment.commitment.x, &y_g.x);
        let y_equal = self.is_equal_nonnative(&commitment.commitment.y, &y_g.y);
        let points_equal = self.and(x_equal, y_equal);

        // Compute C - yG
        let neg_y_g = self.neg_g1_affine(&y_g);
        let c_minus_yg_normal = self.add_or_double_g1_affine(&commitment.commitment, &neg_y_g);

        let infinity = CircuitBuilderKZGHelpers::g1_infinity(self);
        let c_minus_yg = self.select_g1(points_equal, &infinity, &c_minus_yg_normal);

        let left_g1 = c_minus_yg;

        let g2_gen_windowed = G2AffineTarget {
            x: g2_gen.x.clone(),
            y: g2_gen.y.clone(),
            is_infinity: self._false(),
            _phantom: PhantomData,
        };
        // Compute zH
        let z_h = self.curve_scalar_mul_windowed_g2(&g2_gen_windowed, point);

        let z_h_pairing = AffinePointTargetG2 {
            x: z_h.x.clone(),
            y: z_h.y.clone(),
        };

        // Compute τH - zH
        use crate::crypto::bn254::gadgets::pairing::CircuitBuilderCurveG2;
        let neg_z_h = CircuitBuilderCurveG2::neg_g2(self, &z_h_pairing);
        let tau_minus_z_h = CircuitBuilderCurveG2::add_g2::<G2, Bn128Base>(self, g2_tau, &neg_z_h);

        let right_g2 = tau_minus_z_h;

        // Check e(C - yG, H) = e(π, τH - zH)
        let left_pairing = self.pairing::<Bn128Base, G1, G2>(&left_g1, &g2_gen);
        let right_pairing = self.pairing::<Bn128Base, G1, G2>(&proof.w, &right_g2);

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
        // Verify: e(Σ(r^i * (C_i - y_iG)), H) = e(Σ(r^i * π_i), τH - (Σ(r^i * z_i))H)

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

        // Compute random challenge r
        let challenge = CircuitBuilderKZGHelpers::kzg_compute_batch_challenge(self, commitments, points, evaluations, proofs);

        // Compute powers of r: [1, r, r², ..., r^(n-1)]
        let mut r_powers = vec![self.one_nonnative()];
        for i in 1..commitments.len() {
            let prev = &r_powers[i - 1];
            let next = self.mul_nonnative(prev, &challenge);
            r_powers.push(next);
        }

        // Compute left side: Σ(r^i * (C_i - y_iG))
        let g1_gen = self.g1_generator();
        let mut left_acc = CircuitBuilderKZGHelpers::g1_infinity(self);

        for i in 0..commitments.len() {
            let y_g = self.scalar_mul_g1(&g1_gen, &evaluations[i]);
            let neg_y_g = self.neg_g1_affine(&y_g);
            let c_minus_yg = self.add_or_double_g1_affine(&commitments[i].commitment, &neg_y_g);
            let scaled = self.scalar_mul_g1(&c_minus_yg, &r_powers[i]);
            left_acc = self.add_or_double_g1_affine(&left_acc, &scaled);
        }

        // Compute right side G1 part: Σ(r^i * π_i)
        let mut right_acc = CircuitBuilderKZGHelpers::g1_infinity(self);

        for i in 0..proofs.len() {
            let scaled = self.scalar_mul_g1(&proofs[i].w, &r_powers[i]);
            right_acc = self.add_or_double_g1_affine(&right_acc, &scaled);
        }

        let g2_gen = self.constant_affine_point_g2::<G2, Bn128Base>(G2::GENERATOR_AFFINE);

        // Compute Σ(r^i * z_i) for G2 scalar
        let mut z_acc = self.zero_nonnative();

        for i in 0..points.len() {
            let r_times_z = self.mul_nonnative(&r_powers[i], &points[i]);
            z_acc = self.add_nonnative(&z_acc, &r_times_z);
        }

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

        // Compute τH - (Σ(r^i * z_i))H
        let neg_z_h = CircuitBuilderCurveG2::neg_g2(self, &z_h);
        let right_g2 = CircuitBuilderCurveG2::add_g2::<G2, Bn128Base>(self, g2_tau, &neg_z_h);

        // Check e(Σ(r^i * (C_i - y_iG)), H) = e(Σ(r^i * π_i), τH - (Σ(r^i * z_i))H)
        let left_pairing = self.pairing::<Bn128Base, G1, G2>(&left_acc, &g2_gen);
        let right_pairing = self.pairing::<Bn128Base, G1, G2>(&right_acc, &right_g2);

        self.is_equal_ext12(&left_pairing, &right_pairing)
    }
}

impl<F: RichField + Extendable<D>, const D: usize> CircuitBuilderKZGHelpers<F, D>
    for CircuitBuilder<F, D>
{
    fn kzg_commit_lagrange(
        &mut self,
        evaluations: &[NonNativeTarget<Bn128Scalar>],
        lagrange_powers: &[G1AffineTarget<F, D>],
    ) -> KZGCommitmentTarget<F, D> {
        assert_eq!(
            evaluations.len(),
            lagrange_powers.len(),
            "Evaluations and Lagrange powers must have the same length"
        );

        // C = Σ(y_i * L_i(τ) * G)
        // Reuse kzg_commit since it's the same operation
        CircuitBuilderKZG::kzg_commit(self, evaluations, lagrange_powers)
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
            let (eval, proof) = CircuitBuilderKZG::kzg_create_opening_proof(self, coeffs, point, powers_of_tau);
            evaluations.push(eval);
            proofs.push(proof);
        }

        (evaluations, proofs)
    }

    fn kzg_evaluate_polynomial(
        &mut self,
        coefficients: &[NonNativeTarget<Bn128Scalar>],
        point: &NonNativeTarget<Bn128Scalar>,
    ) -> NonNativeTarget<Bn128Scalar> {
        if coefficients.is_empty() {
            return self.constant_nonnative(Bn128Scalar::ZERO);
        }

        if coefficients.len() == 1 {
            return coefficients[0].clone();
        }

        // Horner's method: p(z) = a_0 + z*(a_1 + z*(a_2 + ... + z*a_n))
        let mut result = coefficients[coefficients.len() - 1].clone();

        for i in (0..coefficients.len() - 1).rev() {
            result = self.mul_nonnative(&result, point);
            result = self.add_nonnative(&result, &coefficients[i]);
        }

        result
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

        // a'_0 = a_0 - p(z)
        adjusted_coeffs[0] = self.sub_nonnative(&adjusted_coeffs[0], evaluation);

        let mut quotient = Vec::with_capacity(n - 1);

        // Synthetic division: q(x) = (p(x) - p(z))/(x - z)
        quotient.push(adjusted_coeffs[n - 1].clone());

        for i in (1..n - 1).rev() {
            let z_times_prev = self.mul_nonnative(point, &quotient[quotient.len() - 1]);
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
        use plonky2::hash::poseidon::PoseidonHash;

        let mut inputs = Vec::new();

        // r = Hash(len || C_0 || z_0 || y_0 || π_0 || ... || C_n || z_n || y_n || π_n) mod p
        inputs.push(self.constant(F::from_canonical_usize(commitments.len())));

        for i in 0..commitments.len() {
            // Commitment C_i (x, y coordinates)
            for limb in &commitments[i].commitment.x.value.limbs {
                inputs.push(limb.0);
            }
            for limb in &commitments[i].commitment.y.value.limbs {
                inputs.push(limb.0);
            }

            // Point z_i
            for limb in &points[i].value.limbs {
                inputs.push(limb.0);
            }

            // Evaluation y_i
            for limb in &evaluations[i].value.limbs {
                inputs.push(limb.0);
            }

            // Proof π_i (x, y coordinates)
            for limb in &proofs[i].w.x.value.limbs {
                inputs.push(limb.0);
            }
            for limb in &proofs[i].w.y.value.limbs {
                inputs.push(limb.0);
            }
        }

        let hash = self.hash_n_to_hash_no_pad::<PoseidonHash>(inputs);

        // Convert hash to nonnative scalar
        // Use hash elements as U32 limbs for BigUint, then convert to nonnative
        use crate::u32::gadgets::arithmetic_u32::{CircuitBuilderU32, U32Target};
        use crate::crypto::secp256k1::ecdsa::gadgets::biguint::{BigUintTarget, CircuitBuilderBiguint};
        
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

        // Montgomery batch inversion
        // acc[i] = Π(elements[0..=i])
        let mut acc = vec![elements[0].clone()];
        for i in 1..n {
            let product = self.mul_nonnative(&acc[i - 1], &elements[i]);
            acc.push(product);
        }

        // 1/Π(all elements)
        let inv_acc = self.inv_nonnative(&acc[n - 1]);

        let mut inverses = vec![self.zero_nonnative(); n];

        if n > 1 {
            inverses[n - 1] = self.mul_nonnative(&acc[n - 2], &inv_acc);
        } else {
            inverses[0] = inv_acc.clone();
        }

        let mut running_inv = inv_acc;
        for i in (0..n - 1).rev() {
            if i > 0 {
                inverses[i] = self.mul_nonnative(&acc[i - 1], &running_inv);
            } else {
                inverses[i] = running_inv.clone();
            }
            running_inv = self.mul_nonnative(&running_inv, &elements[i + 1]);
        }

        inverses
    }

    fn g1_infinity(&mut self) -> G1AffineTarget<F, D> {
        let zero = self.zero_nonnative();
        let one = self.one_nonnative();
        G1AffineTarget {
            x: zero,
            y: one, // (0, 1) represents point at infinity in affine coordinates
            is_infinity: self._true(),
            _phantom: PhantomData,
        }
    }


    fn kzg_create_opening_proof_lagrange(
        &mut self,
        evaluations: &[NonNativeTarget<Bn128Scalar>],
        point: &NonNativeTarget<Bn128Scalar>,
        lagrange_basis_g1: &[G1AffineTarget<F, D>],
        fft_settings: &FFTSettingsTarget<F, D>,
    ) -> (NonNativeTarget<Bn128Scalar>, KZGProofTarget<F, D>) {
        // p(z) = Σ(y_i * L_i(z)) using barycentric interpolation
        let eval_at_point = self.lagrange_interpolate_at_point(evaluations, point, fft_settings);

        // q_i = (y_i - p(z))/(ω^i - z)
        let quotient_evals = self.kzg_compute_quotient_polynomial_lagrange(
            evaluations,
            point,
            &eval_at_point,
            fft_settings,
        );

        // π = Σ(q_i * L_i(τ) * G)
        let proof_commitment = self.kzg_commit_lagrange(&quotient_evals, lagrange_basis_g1);

        (
            eval_at_point,
            KZGProofTarget {
                w: proof_commitment.commitment,
            },
        )
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

        // For each i: compute q_i = (y_i - p(z))/(ω^i - z)
        for i in 0..n {
            let is_equal = self.is_equal_nonnative(point, &fft_settings.roots_of_unity[i]);
            is_special.push(is_equal);

            let numerator = self.sub_nonnative(&evaluations[i], eval_at_point);
            let denominator = self.sub_nonnative(&fft_settings.roots_of_unity[i], point);

            denominators.push(denominator);
            quotient_evals.push(numerator);
        }

        // Batch invert all denominators
        let inverses = self.kzg_batch_inverse(&denominators);

        let mut final_quotient_evals = Vec::with_capacity(n);
        for i in 0..n {
            // q_i = (y_i - p(z))/(ω^i - z)
            let q_i = self.mul_nonnative(&quotient_evals[i], &inverses[i]);

            // Special case: q_i = 0 when z = ω^i
            let zero = self.zero_nonnative();
            let final_q_i = self.select_nonnative(is_special[i], &zero, &q_i);

            final_quotient_evals.push(final_q_i);
        }

        final_quotient_evals
    }
}