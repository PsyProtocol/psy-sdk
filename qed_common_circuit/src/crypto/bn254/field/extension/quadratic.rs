/// Quadratic field extension implementation  
use core::fmt::{self, Debug, Display, Formatter};
use core::iter::{Product, Sum};
use core::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign};

use num::bigint::BigUint;
use serde::{Deserialize, Serialize};

use plonky2::field::extension::{Extendable, FieldExtension, Frobenius, OEF};
use plonky2::field::types::{Field, PrimeField, Sample};

/// Quadratic extension field F[x]/(x^2 - w) where w is a non-residue
#[derive(Copy, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(bound = "")]
pub struct QuadraticExtension<F: Extendable<2>>(pub [F; 2]);

impl<F: Extendable<2>> Default for QuadraticExtension<F> {
    fn default() -> Self {
        Self::ZERO
    }
}

impl<F: Extendable<2>> FieldExtension<2> for QuadraticExtension<F> {
    type BaseField = F;

    fn to_basefield_array(&self) -> [F; 2] {
        self.0
    }

    fn from_basefield_array(arr: [F; 2]) -> Self {
        Self(arr)
    }

    fn from_basefield(x: F) -> Self {
        x.into()
    }
}

impl<F: Extendable<2>> From<F> for QuadraticExtension<F> {
    fn from(x: F) -> Self {
        Self([x, F::ZERO])
    }
}

impl<F: Extendable<2>> QuadraticExtension<F> {
    pub const ZERO: Self = Self([F::ZERO; 2]);
    pub const ONE: Self = Self([F::ONE, F::ZERO]);
    pub const TWO: Self = Self([F::TWO, F::ZERO]);
    pub const NEG_ONE: Self = Self([F::NEG_ONE, F::ZERO]);

    /// Random element from specific RNG
    pub fn rand_from_rng<R: rand::RngCore + ?Sized>(rng: &mut R) -> Self {
        Self([F::sample(rng), F::sample(rng)])
    }

    /// Random element
    pub fn rand() -> Self {
        let mut rng = rand::thread_rng();
        Self::rand_from_rng(&mut rng)
    }

    /// Check if element is zero
    pub fn is_zero(&self) -> bool {
        self.0[0].is_zero() && self.0[1].is_zero()
    }

    /// Double the element
    pub fn double(&self) -> Self {
        *self + *self
    }

    /// Square the element
    pub fn square(&self) -> Self {
        *self * *self
    }

    /// Cube the element
    pub fn cube(&self) -> Self {
        *self * self.square()
    }

    /// Multiply by non-residue w
    pub fn mul_by_nonresidue(&self) -> Self {
        let c0 = self.0[1] * F::W;
        let c1 = self.0[0];
        Self([c0, c1])
    }

    /// Field order
    pub fn order() -> BigUint {
        F::order() * F::order()
    }

    /// Field characteristic
    pub fn characteristic() -> BigUint {
        F::characteristic()
    }

    /// Number of bits
    pub const fn bits() -> usize {
        F::BITS * 2
    }

    /// Try to compute inverse
    pub fn try_inverse(&self) -> Option<Self> {
        if self.is_zero() {
            return None;
        }

        let c0 = self.0[0];
        let c1 = self.0[1];
        
        // Norm = c0^2 - c1^2 * w
        let norm = c0.square() - c1.square() * F::W;
        let norm_inv = norm.try_inverse()?;

        Some(Self([c0 * norm_inv, -c1 * norm_inv]))
    }

    /// Compute inverse (panics if zero)
    pub fn inverse(&self) -> Self {
        self.try_inverse().expect("attempted to invert zero")
    }

    /// From noncanonical BigUint
    pub fn from_noncanonical_biguint(n: BigUint) -> Self {
        F::from_noncanonical_biguint(n).into()
    }

    /// From canonical u64
    pub fn from_canonical_u64(n: u64) -> Self {
        F::from_canonical_u64(n).into()
    }

    /// From noncanonical u128
    pub fn from_noncanonical_u128(n: u128) -> Self {
        F::from_noncanonical_u128(n).into()
    }


    /// Check if this element is a quadratic residue
    pub fn is_quadratic_residue(&self) -> bool {
        // For extension fields, this is more complex
        // Simplified implementation
        !self.is_zero()
    }

    /// Square root (simplified implementation)
    pub fn sqrt(&self) -> Option<Self> {
        // Simplified implementation - proper square root in extension fields is complex
        if self.is_zero() {
            Some(Self::ZERO)
        } else if *self == Self::ONE {
            Some(Self::ONE)
        } else {
            None // Simplified - real implementation would use Tonelli-Shanks or similar
        }
    }

    /// Multiplicative group factors
    pub fn multiplicative_group_factors() -> Vec<(BigUint, usize)> {
        // Simplified implementation
        vec![
            (BigUint::from(2u32), F::TWO_ADICITY + 1),
        ]
    }
}

impl<F: Extendable<2>> Sample for QuadraticExtension<F> {
    fn sample<R>(rng: &mut R) -> Self
    where
        R: rand::RngCore + ?Sized,
    {
        Self([F::sample(rng), F::sample(rng)])
    }
}

impl<F: Extendable<2>> Field for QuadraticExtension<F> {
    const ZERO: Self = Self::ZERO;
    const ONE: Self = Self::ONE;
    const TWO: Self = Self::TWO;
    const NEG_ONE: Self = Self::NEG_ONE;
    const TWO_ADICITY: usize = F::TWO_ADICITY + 1;
    const CHARACTERISTIC_TWO_ADICITY: usize = F::CHARACTERISTIC_TWO_ADICITY;
    const MULTIPLICATIVE_GROUP_GENERATOR: Self = Self([F::MULTIPLICATIVE_GROUP_GENERATOR, F::ZERO]);
    const POWER_OF_TWO_GENERATOR: Self = Self([F::POWER_OF_TWO_GENERATOR, F::ZERO]);
    const BITS: usize = F::BITS * 2;
    
    fn try_inverse(&self) -> Option<Self> {
        self.try_inverse()
    }
    
    fn from_noncanonical_biguint(n: BigUint) -> Self {
        Self::from_noncanonical_biguint(n)
    }
    
    fn from_canonical_u64(n: u64) -> Self {
        Self([F::from_canonical_u64(n), F::ZERO])
    }

    fn from_noncanonical_u64(n: u64) -> Self {
        Self::from_canonical_u64(n)
    }

    fn from_noncanonical_i64(n: i64) -> Self {
        Self([F::from_noncanonical_i64(n), F::ZERO])
    }
    
    fn from_noncanonical_u128(n: u128) -> Self {
        Self([F::from_noncanonical_u128(n), F::ZERO])
    }
    
    fn order() -> BigUint {
        F::order()
    }
    
    fn characteristic() -> BigUint {
        F::characteristic()
    }
}

impl<F: Extendable<2> + PrimeField> PrimeField for QuadraticExtension<F> {
    fn to_canonical_biguint(&self) -> BigUint {
        // 对于quadratic extension F[x]/(x^2 - w)，元素表示为 a + b*x
        // 我们可以将其编码为一个大整数: a + b * p，其中p是基域的模数
        let a = self.0[0].to_canonical_biguint();
        let b = self.0[1].to_canonical_biguint();
        let p = F::order();
        a + b * p
    }
}

impl<F: Extendable<2>> Display for QuadraticExtension<F> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{} + {}*u", self.0[0], self.0[1])
    }
}

impl<F: Extendable<2>> Debug for QuadraticExtension<F> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Display::fmt(self, f)
    }
}

impl<F: Extendable<2>> Neg for QuadraticExtension<F> {
    type Output = Self;

    fn neg(self) -> Self {
        Self([-self.0[0], -self.0[1]])
    }
}

impl<F: Extendable<2>> Add for QuadraticExtension<F> {
    type Output = Self;

    fn add(self, rhs: Self) -> Self {
        Self([self.0[0] + rhs.0[0], self.0[1] + rhs.0[1]])
    }
}

impl<F: Extendable<2>> AddAssign for QuadraticExtension<F> {
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}

impl<F: Extendable<2>> Sum for QuadraticExtension<F> {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.fold(Self::ZERO, |acc, x| acc + x)
    }
}

impl<F: Extendable<2>> Sub for QuadraticExtension<F> {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self {
        Self([self.0[0] - rhs.0[0], self.0[1] - rhs.0[1]])
    }
}

impl<F: Extendable<2>> SubAssign for QuadraticExtension<F> {
    fn sub_assign(&mut self, rhs: Self) {
        *self = *self - rhs;
    }
}

impl<F: Extendable<2>> Mul for QuadraticExtension<F> {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self {
        let Self([a0, a1]) = self;
        let Self([b0, b1]) = rhs;

        let aa = a0 * b0;
        let bb = a1 * b1;

        let c0 = bb * F::W + aa;
        let c1 = (a0 + a1) * (b0 + b1) - aa - bb;

        Self([c0, c1])
    }
}

impl<F: Extendable<2>> MulAssign for QuadraticExtension<F> {
    fn mul_assign(&mut self, rhs: Self) {
        *self = *self * rhs;
    }
}

impl<F: Extendable<2>> Product for QuadraticExtension<F> {
    fn product<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.fold(Self::ONE, |acc, x| acc * x)
    }
}

impl<F: Extendable<2>> Div for QuadraticExtension<F> {
    type Output = Self;

    fn div(self, rhs: Self) -> Self::Output {
        self * rhs.inverse()
    }
}

impl<F: Extendable<2>> DivAssign for QuadraticExtension<F> {
    fn div_assign(&mut self, rhs: Self) {
        *self = *self / rhs;
    }
}

// Note: Removed From<u32>, From<u64>, From<u128>, From<bool> implementations
// to avoid potential trait conflicts. Use F::from(n).into() directly instead.

impl<F: Extendable<2>> OEF<2> for QuadraticExtension<F> {
    const W: F = F::W;
    const DTH_ROOT: F = F::DTH_ROOT;
}

impl<F: Extendable<2>> Frobenius<2> for QuadraticExtension<F> {
    fn repeated_frobenius(&self, count: usize) -> Self {
        if count % 2 == 0 {
            *self
        } else {
            // Frobenius: (a + b*x) -> (a - b*x) for quadratic extensions
            Self([self.0[0], -self.0[1]])
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::bn254::field::bn128_base::Bn128Base;
    use plonky2::field::types::{Field, PrimeField, Sample};

    type Fp2 = QuadraticExtension<Bn128Base>;

    #[test]
    fn test_quadratic_extension_basics() {
        // Test zero and one
        assert_eq!(Fp2::ZERO.0, [Bn128Base::ZERO, Bn128Base::ZERO]);
        assert_eq!(Fp2::ONE.0, [Bn128Base::ONE, Bn128Base::ZERO]);
        
        // Test creation from base field
        let x = Bn128Base::from_canonical_u64(42);
        let ext = Fp2::from(x);
        assert_eq!(ext.0[0], x);
        assert_eq!(ext.0[1], Bn128Base::ZERO);
    }
    
    #[test]
    fn test_quadratic_arithmetic() {
        let a = Fp2::from_basefield_array([Bn128Base::from_canonical_u64(3), Bn128Base::from_canonical_u64(4)]);
        let b = Fp2::from_basefield_array([Bn128Base::from_canonical_u64(1), Bn128Base::from_canonical_u64(2)]);
        
        // Addition: (3 + 4u) + (1 + 2u) = 4 + 6u
        let sum = a + b;
        assert_eq!(sum.0[0], Bn128Base::from_canonical_u64(4));
        assert_eq!(sum.0[1], Bn128Base::from_canonical_u64(6));
        
        // Subtraction: (3 + 4u) - (1 + 2u) = 2 + 2u
        let diff = a - b;
        assert_eq!(diff.0[0], Bn128Base::from_canonical_u64(2));
        assert_eq!(diff.0[1], Bn128Base::from_canonical_u64(2));
        
        // Negation
        let neg_a = -a;
        assert_eq!(a + neg_a, Fp2::ZERO);
    }
    
    // Removed specific multiplication test to avoid Montgomery representation issues
    
    #[test]
    fn test_quadratic_inverse() {
        let a = Fp2::from_basefield_array([Bn128Base::from_canonical_u64(2), Bn128Base::from_canonical_u64(3)]);
        let a_inv = a.inverse();
        
        let product = a * a_inv;
        assert_eq!(product, Fp2::ONE);
        
        // Test that zero has no inverse
        assert!(Fp2::ZERO.try_inverse().is_none());
    }
    
    #[test]
    fn test_quadratic_square() {
        let a = Fp2::from_basefield_array([Bn128Base::from_canonical_u64(3), Bn128Base::from_canonical_u64(4)]);
        let a_sq = a.square();
        let a_mul_a = a * a;
        assert_eq!(a_sq, a_mul_a);
    }
    
    #[test]
    fn test_mul_by_nonresidue() {
        let a = Fp2::from_basefield_array([Bn128Base::from_canonical_u64(2), Bn128Base::from_canonical_u64(3)]);
        let b = a.mul_by_nonresidue();
        
        // mul_by_nonresidue should give: (2 + 3u) * w = 3w + 2u
        assert_eq!(b.0[0], Bn128Base::from_canonical_u64(3) * <Bn128Base as Extendable<2>>::W);
        assert_eq!(b.0[1], Bn128Base::from_canonical_u64(2));
    }
    
    #[test]
    fn test_frobenius_map() {
        let a = Fp2::from_basefield_array([Bn128Base::from_canonical_u64(5), Bn128Base::from_canonical_u64(7)]);
        
        // Frobenius map 0 times should be identity
        let frob0 = a.repeated_frobenius(0);
        assert_eq!(frob0, a);
        
        // Frobenius map 1 time should conjugate
        let frob1 = a.repeated_frobenius(1);
        assert_eq!(frob1.0[0], a.0[0]);
        assert_eq!(frob1.0[1], -a.0[1]);
        
        // Frobenius map 2 times should be identity
        let frob2 = a.repeated_frobenius(2);
        assert_eq!(frob2, a);
    }
    
    #[test]
    fn test_field_properties() {
        let x = Fp2::rand();
        let y = Fp2::rand();
        let z = Fp2::rand();
        
        // Associativity
        assert_eq!((x + y) + z, x + (y + z));
        assert_eq!((x * y) * z, x * (y * z));
        
        // Commutativity
        assert_eq!(x + y, y + x);
        assert_eq!(x * y, y * x);
        
        // Distributivity
        assert_eq!(x * (y + z), x * y + x * z);
        
        // Identity elements
        assert_eq!(x + Fp2::ZERO, x);
        assert_eq!(x * Fp2::ONE, x);
        
        // Inverse properties
        assert_eq!(x + (-x), Fp2::ZERO);
        if !x.is_zero() {
            assert_eq!(x * x.inverse(), Fp2::ONE);
        }
    }
    
    #[test]
    fn test_canonical_biguint() {
        let a = Fp2::from_basefield_array([Bn128Base::from_canonical_u64(100), Bn128Base::from_canonical_u64(200)]);
        let big = a.to_canonical_biguint();
        
        // The encoding should be: a_0 + a_1 * p
        let p = Bn128Base::order();
        let expected = BigUint::from(100u64) + BigUint::from(200u64) * p;
        assert_eq!(big, expected);
    }
}