use std::fmt::Debug;
use std::ops::*;

pub trait ContextFelt:
    Copy
    + Debug
    + Clone
    + PartialEq
    + Ord
    + Eq
    + Add
    + Sub
    + Mul
    + Div
    + Rem
    + BitAnd
    + BitOr
    + BitXor
    + Shl
    + Shr
    + Not
    + Neg
    + AddAssign
    + SubAssign
    + MulAssign
    + DivAssign
    + RemAssign
    + BitAndAssign
    + BitOrAssign
    + BitXorAssign
    + ShlAssign
    + ShrAssign
    + Add<u64, Output = Self>
    + Sub<u64, Output = Self>
    + Mul<u64, Output = Self>
    + Div<u64, Output = Self>
    + Rem<u64, Output = Self>
    + BitAnd<u64, Output = Self>
    + BitOr<u64, Output = Self>
    + BitXor<u64, Output = Self>
    + Shl<u64, Output = Self>
    + Shr<u64, Output = Self>
    + AddAssign<u64>
    + SubAssign<u64>
    + MulAssign<u64>
    + DivAssign<u64>
    + RemAssign<u64>
    + BitAndAssign<u64>
    + BitOrAssign<u64>
    + BitXorAssign<u64>
    + ShlAssign<u64>
    + ShrAssign<u64>
    + PartialEq<u64>
    + PartialOrd<u64>
{
    fn cns(value: u64) -> Self;
    fn cns_inverse(value: u64) -> Self;
    fn get_u64(&self) -> u64;
}
