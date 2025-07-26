/// BigUint gadget implementation
use core::marker::PhantomData;

use num::{BigUint, Integer, Zero};
use plonky2::field::extension::Extendable;
use plonky2::field::types::{PrimeField, PrimeField64};
use plonky2::hash::hash_types::RichField;
use plonky2::iop::generator::{GeneratedValues, SimpleGenerator};
use plonky2::iop::target::{BoolTarget, Target};
use plonky2::iop::witness::{PartitionWitness, Witness};
use plonky2::plonk::circuit_builder::CircuitBuilder;

use crate::u32::gadgets::arithmetic_u32::{CircuitBuilderU32, U32Target};
use crate::u32::gadgets::multiple_comparison::list_lte_u32_circuit;
use crate::u32::witness::{GeneratedValuesU32, WitnessU32};

#[derive(Clone, Debug)]
pub struct BigUintTarget {
    pub limbs: Vec<U32Target>,
}

impl BigUintTarget {
    pub fn num_limbs(&self) -> usize {
        self.limbs.len()
    }

    pub fn get_limb(&self, i: usize) -> U32Target {
        self.limbs[i]
    }
}

pub trait CircuitBuilderBiguint<F: RichField + Extendable<D>, const D: usize> {
    fn constant_biguint(&mut self, value: &BigUint) -> BigUintTarget;

    fn zero_biguint(&mut self) -> BigUintTarget;

    fn connect_biguint(&mut self, lhs: &BigUintTarget, rhs: &BigUintTarget);

    fn pad_biguints(
        &mut self,
        a: &BigUintTarget,
        b: &BigUintTarget,
    ) -> (BigUintTarget, BigUintTarget);

    fn cmp_biguint(&mut self, a: &BigUintTarget, b: &BigUintTarget) -> BoolTarget;

    fn add_virtual_biguint_target(&mut self, num_limbs: usize) -> BigUintTarget;

    /// Add two `BigUintTarget`s.
    fn add_biguint(&mut self, a: &BigUintTarget, b: &BigUintTarget) -> BigUintTarget;

    /// Subtract two `BigUintTarget`s. We assume that the first is larger than the second.
    fn sub_biguint(&mut self, a: &BigUintTarget, b: &BigUintTarget) -> BigUintTarget;

    /// Multiply two `BigUintTarget`s.
    fn mul_biguint(&mut self, a: &BigUintTarget, b: &BigUintTarget) -> BigUintTarget;

    /// Multiply two `BigUintTarget`s and returns the low part.
    fn mul_biguint_by_bool(&mut self, a: &BigUintTarget, b: BoolTarget) -> BigUintTarget;

    fn div_rem_biguint(
        &mut self,
        a: &BigUintTarget,
        b: &BigUintTarget,
    ) -> (BigUintTarget, BigUintTarget);

    fn div_biguint(&mut self, a: &BigUintTarget, b: &BigUintTarget) -> BigUintTarget;

    fn rem_biguint(&mut self, a: &BigUintTarget, b: &BigUintTarget) -> BigUintTarget;
}

impl<F: RichField + Extendable<D>, const D: usize> CircuitBuilderBiguint<F, D>
    for CircuitBuilder<F, D>
{
    fn constant_biguint(&mut self, value: &BigUint) -> BigUintTarget {
        let limbs = value
            .to_u32_digits()
            .into_iter()
            .map(|x| self.constant_u32(x))
            .collect();

        BigUintTarget { limbs }
    }

    fn zero_biguint(&mut self) -> BigUintTarget {
        self.constant_biguint(&BigUint::zero())
    }

    fn connect_biguint(&mut self, lhs: &BigUintTarget, rhs: &BigUintTarget) {
        let (lhs_padded, rhs_padded) = self.pad_biguints(lhs, rhs);
        for (lhs_limb, rhs_limb) in lhs_padded.limbs.iter().zip(rhs_padded.limbs.iter()) {
            self.connect_u32(*lhs_limb, *rhs_limb);
        }
    }

    fn pad_biguints(
        &mut self,
        a: &BigUintTarget,
        b: &BigUintTarget,
    ) -> (BigUintTarget, BigUintTarget) {
        let num_limbs = a.num_limbs().max(b.num_limbs());

        let zero_u32 = self.zero_u32();
        let mut a_limbs = a.limbs.clone();
        let mut b_limbs = b.limbs.clone();

        a_limbs.resize(num_limbs, zero_u32);
        b_limbs.resize(num_limbs, zero_u32);

        (
            BigUintTarget { limbs: a_limbs },
            BigUintTarget { limbs: b_limbs },
        )
    }

    fn cmp_biguint(&mut self, a: &BigUintTarget, b: &BigUintTarget) -> BoolTarget {
        let (a_padded, b_padded) = self.pad_biguints(a, b);
        list_lte_u32_circuit(self, a_padded.limbs, b_padded.limbs)
    }

    fn add_virtual_biguint_target(&mut self, num_limbs: usize) -> BigUintTarget {
        let limbs = (0..num_limbs)
            .map(|_| self.add_virtual_u32_target())
            .collect();

        BigUintTarget { limbs }
    }

    fn add_biguint(&mut self, a: &BigUintTarget, b: &BigUintTarget) -> BigUintTarget {
        let num_limbs = a.num_limbs().max(b.num_limbs());
        let (a_padded, b_padded) = self.pad_biguints(a, b);

        let mut result_limbs = Vec::new();
        let mut carry = self.zero_u32();
        for i in 0..num_limbs {
            let (new_limb, new_carry) = self.add_many_u32(
                &[a_padded.limbs[i], b_padded.limbs[i], carry],
            );
            result_limbs.push(new_limb);
            carry = new_carry;
        }

        // Handle final carry
        result_limbs.push(carry);

        BigUintTarget {
            limbs: result_limbs,
        }
    }

    fn sub_biguint(&mut self, a: &BigUintTarget, b: &BigUintTarget) -> BigUintTarget {
        let (a_padded, b_padded) = self.pad_biguints(a, b);
        let num_limbs = a_padded.num_limbs();

        let mut result_limbs = Vec::new();
        let mut borrow = self.zero_u32();
        for i in 0..num_limbs {
            let (new_limb, new_borrow) = self.sub_u32(
                a_padded.limbs[i],
                b_padded.limbs[i],
                borrow,
            );
            result_limbs.push(new_limb);
            borrow = new_borrow;
        }

        BigUintTarget {
            limbs: result_limbs,
        }
    }

    fn mul_biguint(&mut self, a: &BigUintTarget, b: &BigUintTarget) -> BigUintTarget {
        let num_limbs = a.num_limbs() + b.num_limbs();
        let mut result = self.zero_biguint();
        result.limbs.resize(num_limbs, self.zero_u32());

        for i in 0..a.num_limbs() {
            let mut carry = self.zero_u32();
            for j in 0..b.num_limbs() {
                let prod = self.mul_u32(a.limbs[i], b.limbs[j]);
                let sum_with_result = self.add_u32(result.limbs[i + j], prod.0);
                let (new_val, new_carry1) = self.add_many_u32(&[sum_with_result.0, carry]);
                let (final_val, new_carry2) = self.add_many_u32(&[new_val, sum_with_result.1]);
                let total_carry = self.add_u32(new_carry1, new_carry2);
                let final_carry = self.add_u32(total_carry.0, prod.1);

                result.limbs[i + j] = final_val;
                carry = final_carry.0;
                if i + j + 1 < result.limbs.len() {
                    let temp_sum = self.add_u32(result.limbs[i + j + 1], final_carry.1);
                    result.limbs[i + j + 1] = temp_sum.0;
                    // Note: In circuits, we can't do runtime branching based on witness values
                    // This is a simplified implementation
                    if i + j + 2 < result.limbs.len() {
                        let temp_sum2 = self.add_u32(result.limbs[i + j + 2], temp_sum.1);
                        result.limbs[i + j + 2] = temp_sum2.0;
                    }
                }
            }
            if i + b.num_limbs() < result.limbs.len() {
                result.limbs[i + b.num_limbs()] = self.add_u32(result.limbs[i + b.num_limbs()], carry).0;
            }
        }

        result
    }

    fn mul_biguint_by_bool(&mut self, a: &BigUintTarget, b: BoolTarget) -> BigUintTarget {
        let limbs = a
            .limbs
            .iter()
            .map(|&limb| {
                let prod = self.mul(limb.0, b.target);
                U32Target(prod)
            })
            .collect();

        BigUintTarget { limbs }
    }

    fn div_rem_biguint(
        &mut self,
        a: &BigUintTarget,
        b: &BigUintTarget,
    ) -> (BigUintTarget, BigUintTarget) {
        let a_len = a.num_limbs();
        let b_len = b.num_limbs();
        
        let quotient_len = if a_len >= b_len { a_len - b_len + 1 } else { 1 };
        let quotient = self.add_virtual_biguint_target(quotient_len);
        let remainder = self.add_virtual_biguint_target(b_len);

        self.add_simple_generator(BigUintDivRemGenerator::<F, D> {
            a: a.clone(),
            b: b.clone(),
            quotient: quotient.clone(),
            remainder: remainder.clone(),
            _phantom: PhantomData,
        });

        // Constraint: a = b * quotient + remainder
        let prod = self.mul_biguint(b, &quotient);
        let sum = self.add_biguint(&prod, &remainder);
        self.connect_biguint(a, &sum);

        // Constraint: remainder < b
        let remainder_less_than_b = self.cmp_biguint(&remainder, b);
        self.assert_one(remainder_less_than_b.target);

        (quotient, remainder)
    }

    fn div_biguint(&mut self, a: &BigUintTarget, b: &BigUintTarget) -> BigUintTarget {
        let (quotient, _remainder) = self.div_rem_biguint(a, b);
        quotient
    }

    fn rem_biguint(&mut self, a: &BigUintTarget, b: &BigUintTarget) -> BigUintTarget {
        let (_quotient, remainder) = self.div_rem_biguint(a, b);
        remainder
    }
}

#[derive(Debug, Clone)]
struct BigUintDivRemGenerator<F: RichField + Extendable<D>, const D: usize> {
    a: BigUintTarget,
    b: BigUintTarget,
    quotient: BigUintTarget,
    remainder: BigUintTarget,
    _phantom: PhantomData<F>,
}

impl<F: RichField + Extendable<D>, const D: usize> SimpleGenerator<F, D>
    for BigUintDivRemGenerator<F, D>
{
    fn id(&self) -> String {
        "BigUintDivRemGenerator".to_string()
    }

    fn dependencies(&self) -> Vec<Target> {
        let mut deps = Vec::new();
        deps.extend(self.a.limbs.iter().map(|&l| l.0));
        deps.extend(self.b.limbs.iter().map(|&l| l.0));
        deps
    }

    fn run_once(&self, witness: &PartitionWitness<F>, out_buffer: &mut GeneratedValues<F>) -> Result<(), anyhow::Error> {
        let a_value = witness.get_biguint_target(&self.a);
        let b_value = witness.get_biguint_target(&self.b);

        let (quotient_value, remainder_value) = a_value.div_rem(&b_value);

        out_buffer.set_biguint_target(&self.quotient, &quotient_value);
        out_buffer.set_biguint_target(&self.remainder, &remainder_value);
        Ok(())
    }

    fn serialize(
        &self,
        _dst: &mut Vec<u8>,
        _common_data: &plonky2::plonk::circuit_data::CommonCircuitData<F, D>,
    ) -> plonky2::util::serialization::IoResult<()> {
        todo!()
    }

    fn deserialize(
        _src: &mut plonky2::util::serialization::Buffer,
        _common_data: &plonky2::plonk::circuit_data::CommonCircuitData<F, D>,
    ) -> plonky2::util::serialization::IoResult<Self> {
        todo!()
    }
}

pub trait WitnessBigUint<F: PrimeField>: Witness<F> {
    fn get_biguint_target(&self, target: &BigUintTarget) -> BigUint;
    fn set_biguint_target(&mut self, target: &BigUintTarget, value: &BigUint);
}

impl<T: Witness<F>, F: PrimeField64> WitnessBigUint<F> for T {
    fn get_biguint_target(&self, target: &BigUintTarget) -> BigUint {
        target
            .limbs
            .iter()
            .enumerate()
            .fold(BigUint::zero(), |acc, (i, &limb)| {
                let limb_value = self.get_target(limb.0).to_canonical_u64() as u32;
                acc + (BigUint::from(limb_value) << (32 * i))
            })
    }

    fn set_biguint_target(&mut self, target: &BigUintTarget, value: &BigUint) {
        let limbs = value.to_u32_digits();
        for (i, &limb_target) in target.limbs.iter().enumerate() {
            let limb_value = limbs.get(i).copied().unwrap_or(0u32);
            self.set_u32_target(limb_target, limb_value);
        }
    }
}

pub trait GeneratedValuesBigUint<F: PrimeField64> {
    fn set_biguint_target(&mut self, target: &BigUintTarget, value: &BigUint);
}

impl<F: PrimeField64> GeneratedValuesBigUint<F> for GeneratedValues<F> {
    fn set_biguint_target(&mut self, target: &BigUintTarget, value: &BigUint) {
        let limbs = value.to_u32_digits();
        for (i, &limb_target) in target.limbs.iter().enumerate() {
            let limb_value = limbs.get(i).copied().unwrap_or(0u32);
            GeneratedValuesU32::set_u32_target(self, limb_target, limb_value);
        }
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use num::{BigUint, FromPrimitive, Integer};
    use plonky2::iop::witness::PartialWitness;
    use plonky2::plonk::circuit_builder::CircuitBuilder;
    use plonky2::plonk::circuit_data::CircuitConfig;
    use plonky2::plonk::config::{GenericConfig, PoseidonGoldilocksConfig};
    use rand::rngs::OsRng;
    use rand::Rng;

    use super::{CircuitBuilderBiguint, WitnessBigUint};

    #[test]
    fn test_biguint_add() -> Result<()> {
        const D: usize = 2;
        type C = PoseidonGoldilocksConfig;
        type F = <C as GenericConfig<D>>::F;
        let mut rng = OsRng;

        let x_value = BigUint::from_u128(rng.gen()).unwrap();
        let y_value = BigUint::from_u128(rng.gen()).unwrap();
        let expected_z_value = &x_value + &y_value;

        let config = CircuitConfig::standard_recursion_config();
        let mut pw = PartialWitness::new();
        let mut builder = CircuitBuilder::<F, D>::new(config);

        let x = builder.add_virtual_biguint_target(x_value.to_u32_digits().len());
        let y = builder.add_virtual_biguint_target(y_value.to_u32_digits().len());
        let z = builder.add_biguint(&x, &y);
        let expected_z = builder.add_virtual_biguint_target(expected_z_value.to_u32_digits().len());
        builder.connect_biguint(&z, &expected_z);

        pw.set_biguint_target(&x, &x_value);
        pw.set_biguint_target(&y, &y_value);
        pw.set_biguint_target(&expected_z, &expected_z_value);

        let data = builder.build::<C>();
        let proof = data.prove(pw).unwrap();
        data.verify(proof)
    }

    #[test]
    fn test_biguint_sub() -> Result<()> {
        const D: usize = 2;
        type C = PoseidonGoldilocksConfig;
        type F = <C as GenericConfig<D>>::F;
        let mut rng = OsRng;

        let mut x_value = BigUint::from_u128(rng.gen()).unwrap();
        let mut y_value = BigUint::from_u128(rng.gen()).unwrap();
        if y_value > x_value {
            (x_value, y_value) = (y_value, x_value);
        }
        let expected_z_value = &x_value - &y_value;

        let config = CircuitConfig::standard_recursion_config();
        let pw = PartialWitness::new();
        let mut builder = CircuitBuilder::<F, D>::new(config);

        let x = builder.constant_biguint(&x_value);
        let y = builder.constant_biguint(&y_value);
        let z = builder.sub_biguint(&x, &y);
        let expected_z = builder.constant_biguint(&expected_z_value);

        builder.connect_biguint(&z, &expected_z);

        let data = builder.build::<C>();
        let proof = data.prove(pw).unwrap();
        data.verify(proof)
    }

    #[test]
    fn test_biguint_mul() -> Result<()> {
        const D: usize = 2;
        type C = PoseidonGoldilocksConfig;
        type F = <C as GenericConfig<D>>::F;
        let mut rng = OsRng;

        let x_value = BigUint::from_u128(rng.gen()).unwrap();
        let y_value = BigUint::from_u128(rng.gen()).unwrap();
        let expected_z_value = &x_value * &y_value;

        let config = CircuitConfig::standard_recursion_config();
        let mut pw = PartialWitness::new();
        let mut builder = CircuitBuilder::<F, D>::new(config);

        let x = builder.add_virtual_biguint_target(x_value.to_u32_digits().len());
        let y = builder.add_virtual_biguint_target(y_value.to_u32_digits().len());
        let z = builder.mul_biguint(&x, &y);
        let expected_z = builder.add_virtual_biguint_target(expected_z_value.to_u32_digits().len());
        builder.connect_biguint(&z, &expected_z);

        pw.set_biguint_target(&x, &x_value);
        pw.set_biguint_target(&y, &y_value);
        pw.set_biguint_target(&expected_z, &expected_z_value);

        let data = builder.build::<C>();
        let proof = data.prove(pw).unwrap();
        data.verify(proof)
    }

    #[test]
    fn test_biguint_cmp() -> Result<()> {
        const D: usize = 2;
        type C = PoseidonGoldilocksConfig;
        type F = <C as GenericConfig<D>>::F;
        let mut rng = OsRng;

        let x_value = BigUint::from_u128(rng.gen()).unwrap();
        let y_value = BigUint::from_u128(rng.gen()).unwrap();

        let config = CircuitConfig::standard_recursion_config();
        let pw = PartialWitness::new();
        let mut builder = CircuitBuilder::<F, D>::new(config);

        let x = builder.constant_biguint(&x_value);
        let y = builder.constant_biguint(&y_value);
        let cmp = builder.cmp_biguint(&x, &y);
        let expected_cmp = builder.constant_bool(x_value <= y_value);

        builder.connect(cmp.target, expected_cmp.target);

        let data = builder.build::<C>();
        let proof = data.prove(pw).unwrap();
        data.verify(proof)
    }

    #[test]
    fn test_biguint_div_rem() -> Result<()> {
        const D: usize = 2;
        type C = PoseidonGoldilocksConfig;
        type F = <C as GenericConfig<D>>::F;
        let mut rng = OsRng;

        let mut x_value = BigUint::from_u128(rng.gen()).unwrap();
        let mut y_value = BigUint::from_u128(rng.gen()).unwrap();
        if y_value > x_value {
            (x_value, y_value) = (y_value, x_value);
        }
        let (expected_div_value, expected_rem_value) = x_value.div_rem(&y_value);

        let config = CircuitConfig::standard_recursion_config();
        let pw = PartialWitness::new();
        let mut builder = CircuitBuilder::<F, D>::new(config);

        let x = builder.constant_biguint(&x_value);
        let y = builder.constant_biguint(&y_value);
        let (div, rem) = builder.div_rem_biguint(&x, &y);

        let expected_div = builder.constant_biguint(&expected_div_value);
        let expected_rem = builder.constant_biguint(&expected_rem_value);

        builder.connect_biguint(&div, &expected_div);
        builder.connect_biguint(&rem, &expected_rem);

        let data = builder.build::<C>();
        let proof = data.prove(pw).unwrap();
        data.verify(proof)
    }
}

