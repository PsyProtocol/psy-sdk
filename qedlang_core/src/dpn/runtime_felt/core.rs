#[derive(Debug, Clone, Copy, Hash, Ord, Eq, PartialEq, PartialOrd)]
pub struct RuntimeFelt(pub u64);

use std::ops::{
    Add, Sub, Mul, Div, Rem, BitAnd, BitOr, BitXor, Shl, Shr, Not, Neg, AddAssign, SubAssign,
    MulAssign, DivAssign, RemAssign, BitAndAssign, BitOrAssign, BitXorAssign, ShlAssign, ShrAssign,
};

use plonky2::field::{goldilocks_field::GoldilocksField, types::{Field, Field64, PrimeField64}};

use crate::dpn::ops::context_trait::ContextFelt;
impl Add for RuntimeFelt {
    type Output = Self;
    fn add(self, other: Self) -> Self {
        RuntimeFelt((self.0 + other.0)%GoldilocksField::ORDER)
    }
}
impl Sub for RuntimeFelt {
    type Output = Self;
    fn sub(self, other: Self) -> Self {
        RuntimeFelt((GoldilocksField::from_noncanonical_u64(self.0) - GoldilocksField::from_noncanonical_u64(other.0)).to_canonical_u64())
    }
}
impl Mul for RuntimeFelt {
    type Output = Self;
    fn mul(self, other: Self) -> Self {
        RuntimeFelt((GoldilocksField::from_noncanonical_u64(self.0) * GoldilocksField::from_noncanonical_u64(other.0)).to_canonical_u64())
    }
}
impl Div for RuntimeFelt {
    type Output = Self;
    fn div(self, other: Self) -> Self {
        RuntimeFelt((GoldilocksField::from_noncanonical_u64(self.0) / GoldilocksField::from_noncanonical_u64(other.0)).to_canonical_u64())
    }
}
impl Rem for RuntimeFelt {
    type Output = Self;
    fn rem(self, other: Self) -> Self {
        RuntimeFelt(self.0 % other.0)
    }
}
impl BitAnd for RuntimeFelt {
    type Output = Self;
    fn bitand(self, other: Self) -> Self {
        RuntimeFelt((self.0 & other.0)&0xFFFFFFFFu64)
    }
}
impl BitOr for RuntimeFelt {
    type Output = Self;
    fn bitor(self, other: Self) -> Self {
        RuntimeFelt((self.0 | other.0)&0xFFFFFFFFu64)
    }
}
impl BitXor for RuntimeFelt {
    type Output = Self;
    fn bitxor(self, other: Self) -> Self {
        RuntimeFelt((self.0 ^ other.0)&0xFFFFFFFFu64)
    }
}
impl Shl for RuntimeFelt {
    type Output = Self;
    fn shl(self, other: Self) -> Self {
        RuntimeFelt((self.0 << other.0)&0xFFFFFFFFu64)
    }
}
impl Shr for RuntimeFelt {
    type Output = Self;
    fn shr(self, other: Self) -> Self {
        RuntimeFelt((self.0 >> other.0)&0xFFFFFFFFu64)
    }
}
impl Not for RuntimeFelt {
    type Output = Self;
    fn not(self) -> Self {
        RuntimeFelt((self.0 == 0) as u64)
    }
}
impl Neg for RuntimeFelt {
    type Output = Self;
    fn neg(self) -> Self {
        RuntimeFelt(GoldilocksField::from_noncanonical_u64(self.0).neg().to_canonical_u64())
    }
}
impl AddAssign for RuntimeFelt {
    fn add_assign(&mut self, other: Self) {
        *self = RuntimeFelt((self.0 + other.0)%GoldilocksField::ORDER)
    }
}
impl SubAssign for RuntimeFelt {
    fn sub_assign(&mut self, other: Self) {
        *self = RuntimeFelt((GoldilocksField::from_canonical_u64(self.0) - GoldilocksField::from_canonical_u64(other.0)).0)
    }
}
impl MulAssign for RuntimeFelt {
    fn mul_assign(&mut self, other: Self) {
        *self = RuntimeFelt((GoldilocksField::from_canonical_u64(self.0) * GoldilocksField::from_canonical_u64(other.0)).0)
    }
}
impl DivAssign for RuntimeFelt {
    fn div_assign(&mut self, other: Self) {
        *self = RuntimeFelt((GoldilocksField::from_canonical_u64(self.0) / GoldilocksField::from_canonical_u64(other.0)).0)
    }
}
impl RemAssign for RuntimeFelt {
    fn rem_assign(&mut self, other: Self) {
        *self = RuntimeFelt(self.0 % other.0)
    }
}
impl BitAndAssign for RuntimeFelt {
    fn bitand_assign(&mut self, other: Self) {
        *self = RuntimeFelt((self.0 & other.0)&0xFFFFFFFFu64)
    }
}
impl BitOrAssign for RuntimeFelt {
    fn bitor_assign(&mut self, other: Self) {
        *self = RuntimeFelt((self.0 | other.0)&0xFFFFFFFFu64)
    }
}
impl BitXorAssign for RuntimeFelt {
    fn bitxor_assign(&mut self, other: Self) {
        *self = RuntimeFelt((self.0 ^ other.0)&0xFFFFFFFFu64)
    }
}
impl ShlAssign for RuntimeFelt {
    fn shl_assign(&mut self, other: Self) {
        *self = RuntimeFelt((self.0 << other.0)&0xFFFFFFFFu64)
    }
}
impl ShrAssign for RuntimeFelt {
    fn shr_assign(&mut self, other: Self) {
        *self = RuntimeFelt((self.0 >> other.0)&0xFFFFFFFFu64)
    }
}

impl Add<u64> for RuntimeFelt {
    type Output = Self;
    fn add(self, other: u64) -> Self {
        RuntimeFelt((self.0 + other)%GoldilocksField::ORDER)
    }
}
impl Sub<u64> for RuntimeFelt {
    type Output = Self;
    fn sub(self, other: u64) -> Self {
        RuntimeFelt((GoldilocksField::from_noncanonical_u64(self.0) - GoldilocksField::from_noncanonical_u64(other)).to_canonical_u64())
    }
}
impl Mul<u64> for RuntimeFelt {
    type Output = Self;
    fn mul(self, other: u64) -> Self {
        RuntimeFelt((GoldilocksField::from_noncanonical_u64(self.0) * GoldilocksField::from_noncanonical_u64(other)).to_canonical_u64())
    }
}
impl Div<u64> for RuntimeFelt {
    type Output = Self;
    fn div(self, other: u64) -> Self {
        RuntimeFelt((GoldilocksField::from_noncanonical_u64(self.0) / GoldilocksField::from_noncanonical_u64(other)).to_canonical_u64())
    }
}
impl Rem<u64> for RuntimeFelt {
    type Output = Self;
    fn rem(self, other: u64) -> Self {
        RuntimeFelt(self.0 % other)
    }
}
impl BitAnd<u64> for RuntimeFelt {
    type Output = Self;
    fn bitand(self, other: u64) -> Self {
        RuntimeFelt((self.0 & other)&0xFFFFFFFFu64)
    }
}
impl BitOr<u64> for RuntimeFelt {
    type Output = Self;
    fn bitor(self, other: u64) -> Self {
        RuntimeFelt((self.0 | other)&0xFFFFFFFFu64)
    }
}
impl BitXor<u64> for RuntimeFelt {
    type Output = Self;
    fn bitxor(self, other: u64) -> Self {
        RuntimeFelt((self.0 ^ other)&0xFFFFFFFFu64)
    }
}
impl Shl<u64> for RuntimeFelt {
    type Output = Self;
    fn shl(self, other: u64) -> Self {
        RuntimeFelt((self.0 << other)&0xFFFFFFFFu64)
    }
}
impl Shr<u64> for RuntimeFelt {
    type Output = Self;
    fn shr(self, other: u64) -> Self {
        RuntimeFelt((self.0 >> other)&0xFFFFFFFFu64)
    }
}

impl Add<RuntimeFelt> for u64 {
    type Output = RuntimeFelt;
    fn add(self, other: RuntimeFelt) -> RuntimeFelt {
       RuntimeFelt ((self + other.0)%GoldilocksField::ORDER)
    }
}
impl Sub<RuntimeFelt> for u64 {
    type Output = RuntimeFelt;
    fn sub(self, other: RuntimeFelt) -> RuntimeFelt {
        RuntimeFelt((GoldilocksField::from_noncanonical_u64(self) - GoldilocksField::from_noncanonical_u64(other.0)).to_canonical_u64())
    }
}
impl Mul<RuntimeFelt> for u64 {
    type Output = RuntimeFelt;
    fn mul(self, other: RuntimeFelt) -> RuntimeFelt {
        RuntimeFelt((GoldilocksField::from_noncanonical_u64(self) * GoldilocksField::from_noncanonical_u64(other.0)).to_canonical_u64())
    }
}
impl Div<RuntimeFelt> for u64 {
    type Output = RuntimeFelt;
    fn div(self, other: RuntimeFelt) -> RuntimeFelt {
        RuntimeFelt((GoldilocksField::from_noncanonical_u64(self) / GoldilocksField::from_noncanonical_u64(other.0)).to_canonical_u64())
    }
}
impl Rem<RuntimeFelt> for u64 {
    type Output = RuntimeFelt;
    fn rem(self, other: RuntimeFelt) -> RuntimeFelt {
        RuntimeFelt(self % other.0)
    }
}
impl BitAnd<RuntimeFelt> for u64 {
    type Output = RuntimeFelt;
    fn bitand(self, other: RuntimeFelt) -> RuntimeFelt {
        RuntimeFelt((self & other.0)&0xFFFFFFFFu64)
    }
}
impl BitOr<RuntimeFelt> for u64 {
    type Output = RuntimeFelt;
    fn bitor(self, other: RuntimeFelt) -> RuntimeFelt {
        RuntimeFelt((self | other.0)&0xFFFFFFFFu64)
    }
}
impl BitXor<RuntimeFelt> for u64 {
    type Output = RuntimeFelt;
    fn bitxor(self, other: RuntimeFelt) -> RuntimeFelt {
        RuntimeFelt((self ^ other.0)&0xFFFFFFFFu64)
    }
}
impl Shl<RuntimeFelt> for u64 {
    type Output = RuntimeFelt;
    fn shl(self, other: RuntimeFelt) -> RuntimeFelt {
        RuntimeFelt((self << other.0)&0xFFFFFFFFu64)
    }
}
impl Shr<RuntimeFelt> for u64 {
    type Output = RuntimeFelt;
    fn shr(self, other: RuntimeFelt) -> RuntimeFelt {
        RuntimeFelt((self >> other.0)&0xFFFFFFFFu64)
    }
}
impl AddAssign<RuntimeFelt> for u64 {
    fn add_assign(&mut self, other: RuntimeFelt) {
        *self = ((*self + other.0)%GoldilocksField::ORDER)
    }
}
impl SubAssign<RuntimeFelt> for u64 {
    fn sub_assign(&mut self, other: RuntimeFelt) {
        *self = ((GoldilocksField::from_canonical_u64(*self) - GoldilocksField::from_canonical_u64(other.0)).0)
    }
}
impl MulAssign<RuntimeFelt> for u64 {
    fn mul_assign(&mut self, other: RuntimeFelt) {
        *self = ((GoldilocksField::from_canonical_u64(*self) * GoldilocksField::from_canonical_u64(other.0)).0)
    }
}
impl DivAssign<RuntimeFelt> for u64 {
    fn div_assign(&mut self, other: RuntimeFelt) {
        *self = ((GoldilocksField::from_canonical_u64(*self) / GoldilocksField::from_canonical_u64(other.0)).0)
    }
}
impl RemAssign<RuntimeFelt> for u64 {
    fn rem_assign(&mut self, other: RuntimeFelt) {
        *self = (*self % other.0)
    }
}
impl BitAndAssign<RuntimeFelt> for u64 {
    fn bitand_assign(&mut self, other: RuntimeFelt) {
        *self = ((*self & other.0)&0xFFFFFFFFu64)
    }
}
impl BitOrAssign<RuntimeFelt> for u64 {
    fn bitor_assign(&mut self, other: RuntimeFelt) {
        *self = ((*self | other.0)&0xFFFFFFFFu64)
    }
}
impl BitXorAssign<RuntimeFelt> for u64 {
    fn bitxor_assign(&mut self, other: RuntimeFelt) {
        *self = ((*self ^ other.0)&0xFFFFFFFFu64)
    }
}
impl ShlAssign<RuntimeFelt> for u64 {
    fn shl_assign(&mut self, other: RuntimeFelt) {
        *self = ((*self << other.0)&0xFFFFFFFFu64)
    }
}
impl ShrAssign<RuntimeFelt> for u64 {
    fn shr_assign(&mut self, other: RuntimeFelt) {
        *self = ((*self >> other.0)&0xFFFFFFFFu64)
    }
}
impl AddAssign<u64> for RuntimeFelt {
    fn add_assign(&mut self, other: u64) {
        *self = RuntimeFelt((self.0 + other)%GoldilocksField::ORDER)
    }
}
impl SubAssign<u64> for RuntimeFelt {
    fn sub_assign(&mut self, other: u64) {
        *self = RuntimeFelt((GoldilocksField::from_canonical_u64(self.0) - GoldilocksField::from_canonical_u64(other)).0)
    }
}
impl MulAssign<u64> for RuntimeFelt {
    fn mul_assign(&mut self, other: u64) {
        *self = RuntimeFelt((GoldilocksField::from_canonical_u64(self.0) * GoldilocksField::from_canonical_u64(other)).0)
    }
}
impl DivAssign<u64> for RuntimeFelt {
    fn div_assign(&mut self, other: u64) {
        *self = RuntimeFelt((GoldilocksField::from_canonical_u64(self.0) / GoldilocksField::from_canonical_u64(other)).0)
    }
}
impl RemAssign<u64> for RuntimeFelt {
    fn rem_assign(&mut self, other: u64) {
        *self = RuntimeFelt(self.0 % other)
    }
}
impl BitAndAssign<u64> for RuntimeFelt {
    fn bitand_assign(&mut self, other: u64) {
        *self = RuntimeFelt((self.0 & other)&0xFFFFFFFFu64)
    }
}
impl BitOrAssign<u64> for RuntimeFelt {
    fn bitor_assign(&mut self, other: u64) {
        *self = RuntimeFelt((self.0 | other)&0xFFFFFFFFu64)
    }
}
impl BitXorAssign<u64> for RuntimeFelt {
    fn bitxor_assign(&mut self, other: u64) {
        *self = RuntimeFelt((self.0 ^ other)&0xFFFFFFFFu64)
    }
}
impl ShlAssign<u64> for RuntimeFelt {
    fn shl_assign(&mut self, other: u64) {
        *self = RuntimeFelt((self.0 << other)&0xFFFFFFFFu64)
    }
}
impl ShrAssign<u64> for RuntimeFelt {
    fn shr_assign(&mut self, other: u64) {
        *self = RuntimeFelt((self.0 >> other)&0xFFFFFFFFu64)
    }
}
impl PartialEq<u64> for RuntimeFelt {
    fn eq(&self, other: &u64) -> bool {
        self.0 == *other
    }
}
impl PartialOrd<u64> for RuntimeFelt {
    fn partial_cmp(&self, other: &u64) -> Option<std::cmp::Ordering> {
        self.0.partial_cmp(other)
    }
}
impl ContextFelt for RuntimeFelt {
    fn cns(value: u64) -> Self {
        RuntimeFelt(value%GoldilocksField::ORDER)
    }
    fn cns_inverse(value: u64) -> Self {
        RuntimeFelt(GoldilocksField::from_noncanonical_u64(value).inverse().to_canonical_u64())
    }
    fn get_u64(&self) -> u64 {
        self.0
    }
}

impl From<u64> for RuntimeFelt {
    fn from(value: u64) -> RuntimeFelt {
        RuntimeFelt(value%GoldilocksField::ORDER)
    }
}
impl From<u8> for RuntimeFelt {
    fn from(value: u8) -> RuntimeFelt {
        RuntimeFelt(value as u64)
    }
}
impl From<u16> for RuntimeFelt {
    fn from(value: u16) -> RuntimeFelt {
        RuntimeFelt(value as u64)
    }
}
impl From<u32> for RuntimeFelt {
    fn from(value: u32) -> RuntimeFelt {
        RuntimeFelt(value as u64)
    }
}
impl From<bool> for RuntimeFelt {
    fn from(value: bool) -> RuntimeFelt {
        RuntimeFelt(if value { 1 } else { 0 })
    }
}



