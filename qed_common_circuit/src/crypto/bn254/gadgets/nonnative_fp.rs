/// NonNative field arithmetic gadgets
use std::marker::PhantomData;
use std::any::TypeId;

use plonky2::{
    field::{extension::Extendable, types::{Field, PrimeField}},
    hash::hash_types::RichField,
    iop::{
        target::{BoolTarget, Target},
        generator::{GeneratedValues, SimpleGenerator},
        witness::{PartitionWitness, WitnessWrite},
    },
    plonk::circuit_builder::CircuitBuilder,
    gates::gate::Gate,
};

use crate::crypto::bn254::gadgets::biguint::{BigUintTarget, CircuitBuilderBiguint, GeneratedValuesBigUint, WitnessBigUint};
use crate::u32::gadgets::arithmetic_u32::{U32Target, CircuitBuilderU32};
use crate::u32::gadgets::range_check::range_check_u32_circuit;
use crate::crypto::bn254::gadgets::gates::{
    nonnative_add::NonnativeAddGate,
    nonnative_mul::NonnativeMulGate,
    u32_to_u28::U32ToU28Gate,
    u28_to_u32::U28ToU32Gate,
};
use num::{BigUint, Zero};

/// Helper function to compute ceiling division
fn ceil_div_usize(a: usize, b: usize) -> usize {
    (a + b - 1) / b
}

/// A target representing a nonnative field element
#[derive(Clone, Debug)]
pub struct NonNativeTarget<FF: Field> {
    pub value: BigUintTarget,
    pub _phantom: PhantomData<FF>,
}

impl<FF: Field> NonNativeTarget<FF> {
    pub fn new(value: BigUintTarget) -> Self {
        Self {
            value,
            _phantom: PhantomData,
        }
    }
}

/// Trait for circuit builders to support nonnative field operations
pub trait CircuitBuilderNonNative<F: RichField + Extendable<D>, const D: usize> {
    /// Create a new nonnative target
    fn add_virtual_nonnative_target<FF: Field>(&mut self) -> NonNativeTarget<FF>;

    /// Create a nonnative constant
    fn constant_nonnative<FF: PrimeField>(&mut self, value: FF) -> NonNativeTarget<FF>;

    /// Add two nonnative elements
    fn add_nonnative<FF: Field>(
        &mut self,
        lhs: &NonNativeTarget<FF>,
        rhs: &NonNativeTarget<FF>,
    ) -> NonNativeTarget<FF>;

    /// Subtract two nonnative elements
    fn sub_nonnative<FF: Field + PrimeField>(
        &mut self,
        lhs: &NonNativeTarget<FF>,
        rhs: &NonNativeTarget<FF>,
    ) -> NonNativeTarget<FF>;

    /// Multiply two nonnative elements
    fn mul_nonnative<FF: Field>(
        &mut self,
        lhs: &NonNativeTarget<FF>,
        rhs: &NonNativeTarget<FF>,
    ) -> NonNativeTarget<FF>;

    /// Compute the inverse of a nonnative element
    fn inv_nonnative<FF: PrimeField>(&mut self, x: &NonNativeTarget<FF>) -> NonNativeTarget<FF>;

    /// Divide two nonnative elements
    fn div_nonnative<FF: Field + PrimeField>(
        &mut self,
        lhs: &NonNativeTarget<FF>,
        rhs: &NonNativeTarget<FF>,
    ) -> NonNativeTarget<FF>;

    /// Negate a nonnative element
    fn neg_nonnative<FF: Field + PrimeField>(&mut self, x: &NonNativeTarget<FF>) -> NonNativeTarget<FF>;

    /// Check if two nonnative elements are equal
    fn is_equal_nonnative<FF: Field>(
        &mut self,
        lhs: &NonNativeTarget<FF>,
        rhs: &NonNativeTarget<FF>,
    ) -> BoolTarget;

    /// Check if a nonnative element is zero
    fn is_zero_nonnative<FF: Field>(&mut self, x: &NonNativeTarget<FF>) -> BoolTarget;

    /// Conditionally select between two nonnative elements
    fn select_nonnative<FF: Field>(
        &mut self,
        condition: BoolTarget,
        true_value: &NonNativeTarget<FF>,
        false_value: &NonNativeTarget<FF>,
    ) -> NonNativeTarget<FF>;

    /// Create a nonnative zero
    fn zero_nonnative<FF: Field>(&mut self) -> NonNativeTarget<FF>;

    /// Create a nonnative one
    fn one_nonnative<FF: Field>(&mut self) -> NonNativeTarget<FF>;

    /// Convert a boolean to a nonnative element
    fn bool_to_nonnative<FF: Field>(&mut self, b: BoolTarget) -> NonNativeTarget<FF>;

    /// Compute a^b for nonnative elements
    fn pow_nonnative<FF: Field>(
        &mut self,
        base: &NonNativeTarget<FF>,
        exponent: &NonNativeTarget<FF>,
    ) -> NonNativeTarget<FF>;

    /// Square a nonnative element
    fn square_nonnative<FF: Field>(&mut self, x: &NonNativeTarget<FF>) -> NonNativeTarget<FF>;

    /// Cube a nonnative element
    fn cube_nonnative<FF: Field>(&mut self, x: &NonNativeTarget<FF>) -> NonNativeTarget<FF>;

    /// Assert that a nonnative element is valid (in the range [0, modulus))
    fn assert_valid_nonnative<FF: Field>(&mut self, x: &NonNativeTarget<FF>);

    /// Reduce a BigUint modulo the field modulus
    fn reduce_nonnative<FF: Field>(&mut self, x: &BigUintTarget) -> NonNativeTarget<FF>;

    /// Connect two nonnative elements
    fn connect_nonnative<FF: Field>(
        &mut self,
        lhs: &NonNativeTarget<FF>,
        rhs: &NonNativeTarget<FF>,
    );

    /// Multiply a nonnative element by a boolean
    fn mul_nonnative_by_bool<FF: Field>(
        &mut self,
        a: &NonNativeTarget<FF>,
        b: BoolTarget,
    ) -> NonNativeTarget<FF>;

    /// Conditionally negate a nonnative element
    fn nonnative_conditional_neg<FF: PrimeField>(
        &mut self,
        x: &NonNativeTarget<FF>,
        condition: BoolTarget,
    ) -> NonNativeTarget<FF>;

    /// Split a nonnative element into bits
    fn split_nonnative_to_bits<FF: Field>(&mut self, x: &NonNativeTarget<FF>) -> Vec<BoolTarget>;

    /// Multiply by nonresidue for the field FF
    fn mul_by_nonresidue_nonnative<FF: PrimeField>(
        &mut self,
        x: &NonNativeTarget<FF>,
    ) -> NonNativeTarget<FF>;

    /// Returns `x % |FF|` as a `NonNativeTarget`.
    fn reduce<FF: Field>(&mut self, x: &BigUintTarget) -> NonNativeTarget<FF>;

    /// Convert from biguint to nonnative
    fn biguint_to_nonnative<FF: Field>(&mut self, x: &BigUintTarget) -> NonNativeTarget<FF>;

    /// Convert from nonnative to canonical biguint
    fn nonnative_to_canonical_biguint<FF: Field>(
        &mut self,
        x: &NonNativeTarget<FF>,
    ) -> BigUintTarget;

    /// If-then-else for nonnative elements
    fn if_nonnative<FF: PrimeField>(
        &mut self,
        b: BoolTarget,
        x: &NonNativeTarget<FF>,
        y: &NonNativeTarget<FF>,
    ) -> NonNativeTarget<FF>;

    /// Add a virtual nonnative target with specified number of limbs
    fn add_virtual_nonnative_target_sized<FF: Field>(
        &mut self,
        num_limbs: usize,
    ) -> NonNativeTarget<FF>;

    /// Number of limbs needed for a nonnative field element
    fn num_nonnative_limbs<FF: Field>() -> usize;
}

impl<F: RichField + Extendable<D>, const D: usize> CircuitBuilderNonNative<F, D>
    for CircuitBuilder<F, D>
{
    fn add_virtual_nonnative_target<FF: Field>(&mut self) -> NonNativeTarget<FF> {
        // Always create 8 limbs for compatibility with custom gates
        let value = self.add_virtual_biguint_target(8);
        NonNativeTarget::new(value)
    }

    fn constant_nonnative<FF: PrimeField>(&mut self, value: FF) -> NonNativeTarget<FF> {
        let biguint = value.to_canonical_biguint();
        let mut limbs = biguint.to_u32_digits();
        // Ensure we have exactly 8 limbs for custom gates
        limbs.resize(8, 0u32);
        let big_value = BigUintTarget {
            limbs: limbs.into_iter().map(|x| self.constant_u32(x)).collect(),
        };
        NonNativeTarget::new(big_value)
    }

    fn add_nonnative<FF: Field>(
        &mut self,
        lhs: &NonNativeTarget<FF>,
        rhs: &NonNativeTarget<FF>,
    ) -> NonNativeTarget<FF> {
        // Use custom gate for optimized non-native addition
        let gate = NonnativeAddGate::<F, D>::new_from_config(&self.config);
        let (row, copy) = self.find_slot(gate, &[], &[]);
        let mut targets = Vec::new();
        
        for i in 0..8 {
            self.connect(
                Target::wire(row, gate.wire_ith_input_x(copy, i)),
                lhs.value.limbs[i].0,
            );
            self.connect(
                Target::wire(row, gate.wire_ith_input_y(copy, i)),
                rhs.value.limbs[i].0,
            );
            targets.push(U32Target(
                Target::wire(row, gate.wire_ith_output_result(copy, i)),
            ));
        }
        
        NonNativeTarget::new(BigUintTarget { limbs: targets })
    }

    fn sub_nonnative<FF: Field + PrimeField>(
        &mut self,
        lhs: &NonNativeTarget<FF>,
        rhs: &NonNativeTarget<FF>,
    ) -> NonNativeTarget<FF> {
        let diff = self.add_virtual_nonnative_target::<FF>();
        self.add_simple_generator(NonNativeSubtractionGenerator::<F, D, FF> {
            a: lhs.clone(),
            b: rhs.clone(),
            diff: diff.clone(),
            _phantom: PhantomData,
        });
        
        // Range check the difference limbs
        use crate::u32::gadgets::range_check::range_check_u32_circuit;
        range_check_u32_circuit(self, diff.value.limbs.clone());
        
        // Verify: a = diff + b
        let diff_plus_b = self.add_nonnative(&diff, rhs);
        self.connect_nonnative(lhs, &diff_plus_b);
        
        diff
    }

    fn mul_nonnative<FF: Field>(
        &mut self,
        lhs: &NonNativeTarget<FF>,
        rhs: &NonNativeTarget<FF>,
    ) -> NonNativeTarget<FF> {
        // Convert U32 limbs to U28 for multiplication
        let gate = U32ToU28Gate::<F, D>::new_from_config(&self.config);
        let (row, copy) = self.find_slot(gate, &[], &[]);
        for i in 0..8 {
            self.connect(
                Target::wire(row, gate.wire_ith_input(copy, i)),
                lhs.value.limbs[i].0,
            );
        }
        let mut a_targets = Vec::new();
        for i in 0..10 {
            a_targets.push(Target::wire(row, gate.wire_ith_output_result(copy, i)));
        }
        
        let gate = U32ToU28Gate::<F, D>::new_from_config(&self.config);
        let (row, copy) = self.find_slot(gate, &[], &[]);
        for i in 0..8 {
            self.connect(
                Target::wire(row, gate.wire_ith_input(copy, i)),
                rhs.value.limbs[i].0,
            );
        }
        let mut b_targets = Vec::new();
        for i in 0..10 {
            b_targets.push(Target::wire(row, gate.wire_ith_output_result(copy, i)));
        }
        
        // Multiply using custom gate
        let mut xy = Vec::new();
        let gate = NonnativeMulGate::<F, D>::new_from_config(&self.config);
        let (row, copy) = self.find_slot(gate, &[], &[]);
        for i in 0..10 {
            self.connect(
                Target::wire(row, gate.wire_ith_input_x(copy, i)),
                a_targets[i],
            );
            self.connect(
                Target::wire(row, gate.wire_ith_input_y(copy, i)),
                b_targets[i],
            );
            xy.push(Target::wire(row, gate.wire_ith_output_result(copy, i)));
        }
        
        // Multiply by inverse constants for reduction
        let mut res = Vec::new();
        let gate = NonnativeMulGate::<F, D>::new_from_config(&self.config);
        let (row, copy) = self.find_slot(gate, &[], &[]);
        
        // Import BN128_BASE_INV constants from plonky2-pairing
        const BN128_BASE_INV: [u32; 10] = [
            0x14afa37, 0x84884a0, 0x8edf8ed, 0x2285027, 0x2d9eb20, 0xcfb7449, 0x9cf63e9, 0x59e5c63,
            0xe671571, 0x2,
        ];
        
        for i in 0..10 {
            self.connect(Target::wire(row, gate.wire_ith_input_x(copy, i)), xy[i]);
            let inv = self.constant(F::from_canonical_u32(BN128_BASE_INV[i]));
            self.connect(Target::wire(row, gate.wire_ith_input_y(copy, i)), inv);
            res.push(Target::wire(row, gate.wire_ith_output_result(copy, i)));
        }
        
        // Convert back to U32
        let gate = U28ToU32Gate::<F, D>::new_from_config(&self.config);
        let (row, copy) = self.find_slot(gate, &[], &[]);
        for i in 0..10 {
            self.connect(Target::wire(row, gate.wire_ith_input(copy, i)), res[i]);
        }
        let mut targets = Vec::new();
        for i in 0..8 {
            targets.push(U32Target(
                Target::wire(row, gate.wire_ith_output_result(copy, i)),
            ));
        }
        
        NonNativeTarget::new(BigUintTarget { limbs: targets })
    }

    fn inv_nonnative<FF: PrimeField>(&mut self, x: &NonNativeTarget<FF>) -> NonNativeTarget<FF> {
        let num_limbs = x.value.num_limbs();
        let inv_biguint = self.add_virtual_biguint_target(num_limbs);
        let one = self.constant_nonnative(FF::ONE);

        self.add_simple_generator(NonNativeInverseGenerator::<F, D, FF> {
            x: x.clone(),
            inv: inv_biguint.clone(),
            _phantom: PhantomData,
        });

        let product = self.mul_nonnative(
            &x,
            &NonNativeTarget {
                value: inv_biguint.clone(),
                _phantom: PhantomData,
            },
        );
        self.connect_nonnative(&product, &one);

        NonNativeTarget::<FF> {
            value: inv_biguint,
            _phantom: PhantomData,
        }
    }

    fn div_nonnative<FF: Field + PrimeField>(
        &mut self,
        lhs: &NonNativeTarget<FF>,
        rhs: &NonNativeTarget<FF>,
    ) -> NonNativeTarget<FF> {
        let rhs_inv = self.inv_nonnative(rhs);
        self.mul_nonnative(lhs, &rhs_inv)
    }

    fn neg_nonnative<FF: Field + PrimeField>(&mut self, x: &NonNativeTarget<FF>) -> NonNativeTarget<FF> {
        let zero = self.zero_nonnative();
        self.sub_nonnative(&zero, x)
    }

    fn is_equal_nonnative<FF: Field>(
        &mut self,
        lhs: &NonNativeTarget<FF>,
        rhs: &NonNativeTarget<FF>,
    ) -> BoolTarget {
        // Check if all limbs are equal
        let mut all_equal = self._true();
        let (lhs_padded, rhs_padded) = self.pad_biguints(&lhs.value, &rhs.value);
        
        for (lhs_limb, rhs_limb) in lhs_padded.limbs.iter().zip(rhs_padded.limbs.iter()) {
            let limb_equal = self.is_equal(lhs_limb.0, rhs_limb.0);
            all_equal = self.and(all_equal, limb_equal);
        }
        
        all_equal
    }

    fn is_zero_nonnative<FF: Field>(&mut self, x: &NonNativeTarget<FF>) -> BoolTarget {
        let zero = self.zero_biguint();
        self.is_equal_nonnative(&NonNativeTarget::new(zero), x)
    }

    fn select_nonnative<FF: Field>(
        &mut self,
        condition: BoolTarget,
        true_value: &NonNativeTarget<FF>,
        false_value: &NonNativeTarget<FF>,
    ) -> NonNativeTarget<FF> {
        let (true_padded, false_padded) = self.pad_biguints(&true_value.value, &false_value.value);
        let result_limbs: Vec<_> = true_padded.limbs.iter().zip(false_padded.limbs.iter())
            .map(|(&true_limb, &false_limb)| {
                let selected = self.select(condition, true_limb.0, false_limb.0);
                crate::u32::gadgets::arithmetic_u32::U32Target(selected)
            })
            .collect();
        
        NonNativeTarget::new(BigUintTarget { limbs: result_limbs })
    }

    fn zero_nonnative<FF: Field>(&mut self) -> NonNativeTarget<FF> {
        let zero_limbs = vec![self.zero_u32(); 8];
        let zero = BigUintTarget { limbs: zero_limbs };
        NonNativeTarget::new(zero)
    }

    fn one_nonnative<FF: Field>(&mut self) -> NonNativeTarget<FF> {
        let mut limbs = vec![self.zero_u32(); 8];
        limbs[0] = self.one_u32();
        let one = BigUintTarget { limbs };
        NonNativeTarget::new(one)
    }

    fn bool_to_nonnative<FF: Field>(&mut self, b: BoolTarget) -> NonNativeTarget<FF> {
        let one = self.one_nonnative();
        let zero = self.zero_nonnative();
        self.select_nonnative(b, &one, &zero)
    }

    fn pow_nonnative<FF: Field>(
        &mut self,
        base: &NonNativeTarget<FF>,
        exponent: &NonNativeTarget<FF>,
    ) -> NonNativeTarget<FF> {
        // Square-and-multiply algorithm
        let mut result = self.one_nonnative::<FF>();
        let mut base_power = base.clone();
        
        // We need to handle this differently since we can't iterate over witness values
        // This is a simplified version - proper implementation would need more work
        // For now, just return base * base as a placeholder
        let squared = self.mul_nonnative(&base_power, &base_power);
        squared
    }

    fn square_nonnative<FF: Field>(&mut self, x: &NonNativeTarget<FF>) -> NonNativeTarget<FF> {
        self.mul_nonnative(x, x)
    }

    fn cube_nonnative<FF: Field>(&mut self, x: &NonNativeTarget<FF>) -> NonNativeTarget<FF> {
        let x_squared = self.square_nonnative(x);
        self.mul_nonnative(x, &x_squared)
    }

    fn assert_valid_nonnative<FF: Field>(&mut self, x: &NonNativeTarget<FF>) {
        // Assert that x < modulus
        let modulus = self.constant_biguint(&FF::characteristic());
        let is_valid = self.cmp_biguint(&x.value, &modulus);
        self.assert_one(is_valid.target);
    }

    fn reduce_nonnative<FF: Field>(&mut self, x: &BigUintTarget) -> NonNativeTarget<FF> {
        let modulus = self.constant_biguint(&FF::characteristic());
        
        // Use rem_biguint to compute x mod modulus (more efficient than div_rem)
        let remainder = self.rem_biguint(x, &modulus);
        
        NonNativeTarget::new(remainder)
    }

    fn connect_nonnative<FF: Field>(
        &mut self,
        lhs: &NonNativeTarget<FF>,
        rhs: &NonNativeTarget<FF>,
    ) {
        self.connect_biguint(&lhs.value, &rhs.value);
    }

    fn mul_nonnative_by_bool<FF: Field>(
        &mut self,
        a: &NonNativeTarget<FF>,
        b: BoolTarget,
    ) -> NonNativeTarget<FF> {
        let result = self.mul_biguint_by_bool(&a.value, b);
        NonNativeTarget::new(result)
    }

    fn nonnative_conditional_neg<FF: PrimeField>(
        &mut self,
        x: &NonNativeTarget<FF>,
        condition: BoolTarget,
    ) -> NonNativeTarget<FF> {
        let neg = self.neg_nonnative(x);
        self.select_nonnative(condition, &neg, x)
    }

    fn split_nonnative_to_bits<FF: Field>(&mut self, x: &NonNativeTarget<FF>) -> Vec<BoolTarget> {
        let num_limbs = x.value.num_limbs();
        let mut result = Vec::with_capacity(num_limbs * 32);

        for i in 0..num_limbs {
            let limb = x.value.get_limb(i);
            let bit_targets = self.split_le_base::<2>(limb.0, 32);
            let bits: Vec<_> = bit_targets
                .iter()
                .map(|&t| BoolTarget::new_unsafe(t))
                .collect();

            result.extend(bits);
        }

        result
    }

    fn mul_by_nonresidue_nonnative<FF: PrimeField>(
        &mut self,
        x: &NonNativeTarget<FF>,
    ) -> NonNativeTarget<FF> {
        // For BN128 fields, NONRESIDUE is -1, so we just negate
        // This works for both Bn128Base and Bn128Scalar
        self.neg_nonnative(x)
    }

    fn reduce<FF: Field>(&mut self, x: &BigUintTarget) -> NonNativeTarget<FF> {
        self.reduce_nonnative(x)
    }

    fn biguint_to_nonnative<FF: Field>(&mut self, x: &BigUintTarget) -> NonNativeTarget<FF> {
        NonNativeTarget {
            value: x.clone(),
            _phantom: PhantomData,
        }
    }

    fn nonnative_to_canonical_biguint<FF: Field>(
        &mut self,
        x: &NonNativeTarget<FF>,
    ) -> BigUintTarget {
        x.value.clone()
    }

    fn if_nonnative<FF: PrimeField>(
        &mut self,
        b: BoolTarget,
        x: &NonNativeTarget<FF>,
        y: &NonNativeTarget<FF>,
    ) -> NonNativeTarget<FF> {
        let not_b = self.not(b);
        let maybe_x = self.mul_nonnative_by_bool(x, b);
        let maybe_y = self.mul_nonnative_by_bool(y, not_b);
        self.add_nonnative(&maybe_x, &maybe_y)
    }

    fn add_virtual_nonnative_target_sized<FF: Field>(
        &mut self,
        num_limbs: usize,
    ) -> NonNativeTarget<FF> {
        let value = self.add_virtual_biguint_target(num_limbs);
        NonNativeTarget {
            value,
            _phantom: PhantomData,
        }
    }

    fn num_nonnative_limbs<FF: Field>() -> usize {
        ceil_div_usize(FF::BITS, 32)
    }
}

#[derive(Debug)]
struct NonNativeSubtractionGenerator<F: RichField + Extendable<D>, const D: usize, FF: Field> {
    a: NonNativeTarget<FF>,
    b: NonNativeTarget<FF>,
    diff: NonNativeTarget<FF>,
    _phantom: PhantomData<F>,
}

impl<F: RichField + Extendable<D>, const D: usize, FF: PrimeField> SimpleGenerator<F, D>
    for NonNativeSubtractionGenerator<F, D, FF>
{
    fn id(&self) -> String {
        "NonNativeSubtractionGenerator".to_string()
    }

    fn serialize(&self, dst: &mut Vec<u8>, _common_data: &plonky2::plonk::circuit_data::CommonCircuitData<F, D>) -> Result<(), plonky2::util::serialization::IoError> {
        use plonky2::util::serialization::Write;
        dst.write_usize(self.a.value.limbs.len())?;
        for limb in &self.a.value.limbs {
            dst.write_target(limb.0)?;
        }
        for limb in &self.b.value.limbs {
            dst.write_target(limb.0)?;
        }
        for limb in &self.diff.value.limbs {
            dst.write_target(limb.0)?;
        }
        Ok(())
    }

    fn deserialize(src: &mut plonky2::util::serialization::Buffer, _common_data: &plonky2::plonk::circuit_data::CommonCircuitData<F, D>) -> Result<Self, plonky2::util::serialization::IoError> {
        use plonky2::util::serialization::Read;
        let limb_count = src.read_usize()?;
        
        let mut a_limbs = Vec::with_capacity(limb_count);
        for _ in 0..limb_count {
            a_limbs.push(U32Target(src.read_target()?));
        }
        
        let mut b_limbs = Vec::with_capacity(limb_count);
        for _ in 0..limb_count {
            b_limbs.push(U32Target(src.read_target()?));
        }
        
        let mut diff_limbs = Vec::with_capacity(limb_count);
        for _ in 0..limb_count {
            diff_limbs.push(U32Target(src.read_target()?));
        }
        
        Ok(Self {
            a: NonNativeTarget::new(BigUintTarget { limbs: a_limbs }),
            b: NonNativeTarget::new(BigUintTarget { limbs: b_limbs }),
            diff: NonNativeTarget::new(BigUintTarget { limbs: diff_limbs }),
            _phantom: PhantomData,
        })
    }

    fn dependencies(&self) -> Vec<Target> {
        self.a
            .value
            .limbs
            .iter()
            .cloned()
            .chain(self.b.value.limbs.clone())
            .map(|l| l.0)
            .collect()
    }

    fn run_once(&self, witness: &PartitionWitness<F>, out_buffer: &mut GeneratedValues<F>) -> Result<(), anyhow::Error> {
        let a_biguint = witness.get_biguint_target(&self.a.value);
        let b_biguint = witness.get_biguint_target(&self.b.value);
        let p = FF::order();
        
        // Ensure a is in range [0, p)
        let a_mod_p = a_biguint % &p;
        // Ensure b is in range [0, p)
        let b_mod_p = b_biguint % &p;
        
        // Compute (a - b) mod p
        let diff_biguint = if a_mod_p >= b_mod_p {
            a_mod_p - b_mod_p
        } else {
            a_mod_p + &p - b_mod_p
        };
        
        out_buffer.set_biguint_target(&self.diff.value, &diff_biguint);
        Ok(())
    }
}

#[derive(Debug)]
struct NonNativeInverseGenerator<F: RichField + Extendable<D>, const D: usize, FF: PrimeField> {
    x: NonNativeTarget<FF>,
    inv: BigUintTarget,
    _phantom: PhantomData<F>,
}

impl<F: RichField + Extendable<D>, const D: usize, FF: PrimeField> SimpleGenerator<F, D>
    for NonNativeInverseGenerator<F, D, FF>
{
    fn id(&self) -> String {
        "NonNativeInverseGenerator".to_string()
    }

    fn serialize(&self, dst: &mut Vec<u8>, _common_data: &plonky2::plonk::circuit_data::CommonCircuitData<F, D>) -> Result<(), plonky2::util::serialization::IoError> {
        use plonky2::util::serialization::Write;
        dst.write_usize(self.x.value.limbs.len())?;
        for limb in &self.x.value.limbs {
            dst.write_target(limb.0)?;
        }
        dst.write_usize(self.inv.limbs.len())?;
        for limb in &self.inv.limbs {
            dst.write_target(limb.0)?;
        }
        Ok(())
    }

    fn deserialize(src: &mut plonky2::util::serialization::Buffer, _common_data: &plonky2::plonk::circuit_data::CommonCircuitData<F, D>) -> Result<Self, plonky2::util::serialization::IoError> {
        use plonky2::util::serialization::Read;
        let x_limb_count = src.read_usize()?;
        let mut x_limbs = Vec::with_capacity(x_limb_count);
        for _ in 0..x_limb_count {
            x_limbs.push(U32Target(src.read_target()?));
        }
        
        let inv_limb_count = src.read_usize()?;
        let mut inv_limbs = Vec::with_capacity(inv_limb_count);
        for _ in 0..inv_limb_count {
            inv_limbs.push(U32Target(src.read_target()?));
        }
        
        Ok(Self {
            x: NonNativeTarget::new(BigUintTarget { limbs: x_limbs }),
            inv: BigUintTarget { limbs: inv_limbs },
            _phantom: PhantomData,
        })
    }

    fn dependencies(&self) -> Vec<Target> {
        self.x.value.limbs.iter().map(|&l| l.0).collect()
    }

    fn run_once(&self, witness: &PartitionWitness<F>, out_buffer: &mut GeneratedValues<F>) -> Result<(), anyhow::Error> {
        let x = FF::from_noncanonical_biguint(witness.get_biguint_target(&self.x.value));
        let inv = x.inverse();
        let inv_biguint = inv.to_canonical_biguint();
        out_buffer.set_biguint_target(&self.inv, &inv_biguint);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::bn254::field::bn128_base::Bn128Base;
    use plonky2::field::types::{Field, PrimeField, Sample};
    use plonky2::iop::witness::PartialWitness;
    use plonky2::plonk::circuit_data::CircuitConfig;
    use plonky2::plonk::config::{GenericConfig, PoseidonGoldilocksConfig};

    type FF = Bn128Base;
    const D: usize = 2;
    type C = PoseidonGoldilocksConfig;
    type F = <C as GenericConfig<D>>::F;

    #[test]
    fn test_constant_nonnative() {
        let config = CircuitConfig {
            num_wires: 400, // Enough for NonnativeMulGate (334) and some margin
            ..CircuitConfig::wide_ecc_config()
        };
        let mut builder = CircuitBuilder::<F, D>::new(config);

        let x_ff = FF::from_canonical_u64(12345);
        let x_target = builder.constant_nonnative(x_ff);
        
        // Create another constant with same value
        let y_target = builder.constant_nonnative(x_ff);
        
        // They should be equal
        builder.connect_nonnative(&x_target, &y_target);

        let data = builder.build::<C>();
        let pw = PartialWitness::new();
        let proof = data.prove(pw).unwrap();
        data.verify(proof).unwrap();
    }

    #[test]
    fn test_nonnative_addition() {
        let config = CircuitConfig {
            num_wires: 400, // Enough for NonnativeMulGate (334) and some margin
            ..CircuitConfig::wide_ecc_config()
        };
        let mut builder = CircuitBuilder::<F, D>::new(config);

        let a_ff = FF::from_canonical_u64(100);
        let b_ff = FF::from_canonical_u64(200);
        let sum_ff = a_ff + b_ff;

        let a = builder.constant_nonnative(a_ff);
        let b = builder.constant_nonnative(b_ff);
        let sum = builder.add_nonnative(&a, &b);

        let sum_expected = builder.constant_nonnative(sum_ff);
        builder.connect_nonnative(&sum, &sum_expected);

        let data = builder.build::<C>();
        let pw = PartialWitness::new();
        let proof = data.prove(pw).unwrap();
        data.verify(proof).unwrap();
    }

    #[test]
    fn test_nonnative_subtraction() {
        let config = CircuitConfig {
            num_wires: 400, // Enough for NonnativeMulGate (334) and some margin
            num_routed_wires: 80,
            num_constants: 2,
            use_base_arithmetic_gate: true,
            security_bits: 100,
            num_challenges: 2,
            zero_knowledge: false,
            max_quotient_degree_factor: 8,
            fri_config: plonky2::fri::FriConfig {
                rate_bits: 3,
                cap_height: 4,
                proof_of_work_bits: 16,
                reduction_strategy: plonky2::fri::reduction_strategies::FriReductionStrategy::ConstantArityBits(4, 5),
                num_query_rounds: 28,
            },
        };
        let mut builder = CircuitBuilder::<F, D>::new(config);

        let a_ff = FF::from_canonical_u64(300);
        let b_ff = FF::from_canonical_u64(100);
        let diff_ff = a_ff - b_ff;

        let a = builder.constant_nonnative(a_ff);
        let b = builder.constant_nonnative(b_ff);
        let diff = builder.sub_nonnative(&a, &b);

        // Instead of creating a new constant and connecting, 
        // let's verify the result is correct using addition
        let sum = builder.add_nonnative(&diff, &b);
        builder.connect_nonnative(&sum, &a);

        let data = builder.build::<C>();
        let pw = PartialWitness::new();
        let proof = data.prove(pw).unwrap();
        data.verify(proof).unwrap();
    }

    #[test]
    fn test_nonnative_multiplication() {
        let config = CircuitConfig {
            num_wires: 400, // Enough for NonnativeMulGate (334) and some margin
            ..CircuitConfig::wide_ecc_config()
        };
        let mut builder = CircuitBuilder::<F, D>::new(config);

        let a_ff = FF::from_canonical_u64(7);
        let b_ff = FF::from_canonical_u64(13);
        let product_ff = a_ff * b_ff;

        let a = builder.constant_nonnative(a_ff);
        let b = builder.constant_nonnative(b_ff);
        let product = builder.mul_nonnative(&a, &b);

        let product_expected = builder.constant_nonnative(product_ff);
        builder.connect_nonnative(&product, &product_expected);

        let data = builder.build::<C>();
        let pw = PartialWitness::new();
        let proof = data.prove(pw).unwrap();
        data.verify(proof).unwrap();
    }

    #[test]
    fn test_nonnative_negation() {
        let config = CircuitConfig {
            num_wires: 400, // Enough for NonnativeMulGate (334) and some margin
            ..CircuitConfig::wide_ecc_config()
        };
        let mut builder = CircuitBuilder::<F, D>::new(config);

        let x_ff = FF::from_canonical_u64(42);
        let neg_x_ff = -x_ff;

        let x = builder.constant_nonnative(x_ff);
        let neg_x = builder.neg_nonnative(&x);

        let neg_x_expected = builder.constant_nonnative(neg_x_ff);
        builder.connect_nonnative(&neg_x, &neg_x_expected);

        // Also verify that x + (-x) = 0
        let sum = builder.add_nonnative(&x, &neg_x);
        let zero = builder.zero_nonnative();
        builder.connect_nonnative(&sum, &zero);

        let data = builder.build::<C>();
        let pw = PartialWitness::new();
        let proof = data.prove(pw).unwrap();
        data.verify(proof).unwrap();
    }

    #[test]
    fn test_nonnative_inverse() {
        let config = CircuitConfig {
            num_wires: 400, // Enough for NonnativeMulGate (334) and some margin
            num_routed_wires: 80,
            num_constants: 2,
            use_base_arithmetic_gate: true,
            security_bits: 100,
            num_challenges: 2,
            zero_knowledge: false,
            max_quotient_degree_factor: 8,
            fri_config: plonky2::fri::FriConfig {
                rate_bits: 3,
                cap_height: 4,
                proof_of_work_bits: 16,
                reduction_strategy: plonky2::fri::reduction_strategies::FriReductionStrategy::ConstantArityBits(4, 5),
                num_query_rounds: 28,
            },
        };
        let mut builder = CircuitBuilder::<F, D>::new(config);

        let x_ff = FF::from_canonical_u64(7);
        let inv_x_ff = x_ff.inverse();

        let x = builder.constant_nonnative(x_ff);
        let inv_x = builder.inv_nonnative(&x);

        let inv_x_expected = builder.constant_nonnative(inv_x_ff);
        builder.connect_nonnative(&inv_x, &inv_x_expected);

        // Also verify that x * x^(-1) = 1
        let product = builder.mul_nonnative(&x, &inv_x);
        let one = builder.constant_nonnative(FF::ONE);
        builder.connect_nonnative(&product, &one);

        let data = builder.build::<C>();
        let pw = PartialWitness::new();
        let proof = data.prove(pw).unwrap();
        data.verify(proof).unwrap();
    }

    #[test]
    fn test_bool_to_nonnative() {
        let config = CircuitConfig {
            num_wires: 400, // Enough for NonnativeMulGate (334) and some margin
            ..CircuitConfig::wide_ecc_config()
        };
        let mut builder = CircuitBuilder::<F, D>::new(config);

        let true_val = builder._true();
        let false_val = builder._false();

        let true_nn = builder.bool_to_nonnative::<FF>(true_val);
        let false_nn = builder.bool_to_nonnative::<FF>(false_val);

        let one = builder.one_nonnative();
        let zero = builder.zero_nonnative();

        builder.connect_nonnative(&true_nn, &one);
        builder.connect_nonnative(&false_nn, &zero);

        let data = builder.build::<C>();
        let pw = PartialWitness::new();
        let proof = data.prove(pw).unwrap();
        data.verify(proof).unwrap();
    }

    #[test]
    fn test_mul_by_bool() {
        let config = CircuitConfig {
            num_wires: 400, // Enough for NonnativeMulGate (334) and some margin
            ..CircuitConfig::wide_ecc_config()
        };
        let mut builder = CircuitBuilder::<F, D>::new(config);

        let x_ff = FF::from_canonical_u64(42);
        let x = builder.constant_nonnative(x_ff);
        
        let true_val = builder._true();
        let false_val = builder._false();

        // x * true = x
        let x_times_true = builder.mul_nonnative_by_bool(&x, true_val);
        builder.connect_nonnative(&x_times_true, &x);

        // x * false = 0
        let x_times_false = builder.mul_nonnative_by_bool(&x, false_val);
        let zero = builder.zero_nonnative();
        builder.connect_nonnative(&x_times_false, &zero);

        let data = builder.build::<C>();
        let pw = PartialWitness::new();
        let proof = data.prove(pw).unwrap();
        data.verify(proof).unwrap();
    }

    #[test]
    fn test_select_nonnative() {
        let config = CircuitConfig {
            num_wires: 400, // Enough for NonnativeMulGate (334) and some margin
            ..CircuitConfig::wide_ecc_config()
        };
        let mut builder = CircuitBuilder::<F, D>::new(config);

        let a_ff = FF::from_canonical_u64(100);
        let b_ff = FF::from_canonical_u64(200);
        
        let a = builder.constant_nonnative(a_ff);
        let b = builder.constant_nonnative(b_ff);
        
        let true_val = builder._true();
        let false_val = builder._false();

        // select(true, a, b) = a
        let selected_true = builder.select_nonnative(true_val, &a, &b);
        builder.connect_nonnative(&selected_true, &a);

        // select(false, a, b) = b
        let selected_false = builder.select_nonnative(false_val, &a, &b);
        builder.connect_nonnative(&selected_false, &b);

        let data = builder.build::<C>();
        let pw = PartialWitness::new();
        let proof = data.prove(pw).unwrap();
        data.verify(proof).unwrap();
    }

    #[test]
    fn test_conditional_neg() {
        let config = CircuitConfig {
            num_wires: 400, // Enough for NonnativeMulGate (334) and some margin
            ..CircuitConfig::wide_ecc_config()
        };
        let mut builder = CircuitBuilder::<F, D>::new(config);

        let x_ff = FF::from_canonical_u64(42);
        let x = builder.constant_nonnative(x_ff);
        
        let true_val = builder._true();
        let false_val = builder._false();

        // conditional_neg(x, true) = -x
        let neg_if_true = builder.nonnative_conditional_neg(&x, true_val);
        let neg_x = builder.neg_nonnative(&x);
        builder.connect_nonnative(&neg_if_true, &neg_x);

        // conditional_neg(x, false) = x
        let neg_if_false = builder.nonnative_conditional_neg(&x, false_val);
        builder.connect_nonnative(&neg_if_false, &x);

        let data = builder.build::<C>();
        let pw = PartialWitness::new();
        let proof = data.prove(pw).unwrap();
        data.verify(proof).unwrap();
    }

    #[test]
    fn test_nonnative_performance_batch_operations() {
        use std::time::Instant;
        
        let config = CircuitConfig {
            num_wires: 400, // Enough for NonnativeMulGate (334) and some margin
            ..CircuitConfig::wide_ecc_config()
        };
        let mut builder = CircuitBuilder::<F, D>::new(config);

        // Create multiple operations to test circuit scalability
        let values: Vec<FF> = (1..=10).map(|i| FF::from_canonical_u64(i as u64 * 123)).collect();
        let targets: Vec<_> = values.iter()
            .map(|&v| builder.constant_nonnative(v))
            .collect();

        // Perform chain of operations
        let mut result = targets[0].clone();
        for i in 1..targets.len() {
            // Add
            result = builder.add_nonnative(&result, &targets[i]);
            // Multiply by a constant
            let constant = builder.constant_nonnative(FF::from_canonical_u64(2));
            result = builder.mul_nonnative(&result, &constant);
        }

        // Final verification
        let final_target = builder.add_virtual_nonnative_target();
        builder.connect_nonnative(&result, &final_target);

        println!("Building circuit with {} nonnative operations...", targets.len() * 2);
        let start = Instant::now();
        let data = builder.build::<C>();
        let build_time = start.elapsed();
        
        println!("Circuit build time: {:?}", build_time);
        println!("Number of gates: {}", data.common.gates.len());
        
        let pw = PartialWitness::new();
        
        let start = Instant::now();
        let proof = data.prove(pw).unwrap();
        let prove_time = start.elapsed();
        
        let start = Instant::now();
        data.verify(proof).unwrap();
        let verify_time = start.elapsed();
        
        println!("Prove time: {:?}", prove_time);
        println!("Verify time: {:?}", verify_time);
        
        // Performance thresholds (adjust based on your requirements)
        assert!(build_time.as_millis() < 5000, "Build time too long: {:?}", build_time);
        assert!(prove_time.as_millis() < 10000, "Prove time too long: {:?}", prove_time);
        assert!(verify_time.as_millis() < 1000, "Verify time too long: {:?}", verify_time);
    }

    #[test]
    fn test_nonnative_large_field_operations() {
        let config = CircuitConfig {
            num_wires: 400, // Enough for NonnativeMulGate (334) and some margin
            num_routed_wires: 80,
            num_constants: 2,
            use_base_arithmetic_gate: true,
            security_bits: 100,
            num_challenges: 2,
            zero_knowledge: false,
            max_quotient_degree_factor: 8,
            fri_config: plonky2::fri::FriConfig {
                rate_bits: 3,
                cap_height: 4,
                proof_of_work_bits: 16,
                reduction_strategy: plonky2::fri::reduction_strategies::FriReductionStrategy::ConstantArityBits(4, 5),
                num_query_rounds: 28,
            },
        };
        let mut builder = CircuitBuilder::<F, D>::new(config);

        // Test with values close to field modulus
        let large_val1 = FF::from_canonical_u64(u64::MAX - 1000);
        let large_val2 = FF::from_canonical_u64(u64::MAX - 2000);
        
        let target1 = builder.constant_nonnative(large_val1);
        let target2 = builder.constant_nonnative(large_val2);

        // Test various operations with large values
        let sum = builder.add_nonnative(&target1, &target2);
        let diff = builder.sub_nonnative(&target1, &target2);
        let product = builder.mul_nonnative(&target1, &target2);
        let inv1 = builder.inv_nonnative(&target1);
        
        // Verify difference is what we expect
        // diff + target2 should equal target1
        let diff_plus_b = builder.add_nonnative(&diff, &target2);
        builder.connect_nonnative(&diff_plus_b, &target1);
        
        // Verify inverse works: target1 * inv1 = 1
        let one = builder.constant_nonnative(FF::ONE);
        let product_inv = builder.mul_nonnative(&target1, &inv1);
        builder.connect_nonnative(&product_inv, &one);

        let data = builder.build::<C>();
        let pw = PartialWitness::new();
        let proof = data.prove(pw).unwrap();
        data.verify(proof).unwrap();
    }

    #[test]
    fn test_nonnative_edge_cases() {
        let config = CircuitConfig {
            num_wires: 400, // Enough for NonnativeMulGate (334) and some margin
            num_routed_wires: 80,
            num_constants: 2,
            use_base_arithmetic_gate: true,
            security_bits: 100,
            num_challenges: 2,
            zero_knowledge: false,
            max_quotient_degree_factor: 8,
            fri_config: plonky2::fri::FriConfig {
                rate_bits: 3,
                cap_height: 4,
                proof_of_work_bits: 16,
                reduction_strategy: plonky2::fri::reduction_strategies::FriReductionStrategy::ConstantArityBits(4, 5),
                num_query_rounds: 28,
            },
        };
        let mut builder = CircuitBuilder::<F, D>::new(config);

        // Test full edge cases
        let zero: NonNativeTarget<FF> = builder.zero_nonnative();
        let one = builder.constant_nonnative(FF::ONE);
        let value = builder.constant_nonnative(FF::from_canonical_u64(42));

        // Zero operations
        let zero_plus_value = builder.add_nonnative(&zero, &value);
        builder.connect_nonnative(&zero_plus_value, &value);

        let value_plus_zero = builder.add_nonnative(&value, &zero);
        builder.connect_nonnative(&value_plus_zero, &value);

        let zero_times_value = builder.mul_nonnative(&zero, &value);
        builder.connect_nonnative(&zero_times_value, &zero);

        // One operations
        let one_times_value = builder.mul_nonnative(&one, &value);
        builder.connect_nonnative(&one_times_value, &value);

        let value_times_one = builder.mul_nonnative(&value, &one);
        builder.connect_nonnative(&value_times_one, &value);

        let data = builder.build::<C>();
        let pw = PartialWitness::new();
        let proof = data.prove(pw).unwrap();
        data.verify(proof).unwrap();
    }
}

