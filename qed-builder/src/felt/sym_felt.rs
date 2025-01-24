use std::collections::HashMap;
use std::fmt::Display;
use std::hash::Hasher;
use std::ops::{
    Add, AddAssign, BitAnd, BitAndAssign, BitOr, BitOrAssign, BitXor, BitXorAssign, Div, DivAssign,
    Mul, MulAssign, Neg, Not, Rem, RemAssign, Shl, ShlAssign, Shr, ShrAssign, Sub, SubAssign,
};

use plonky2::field::types::{Field, PrimeField64};
use plonky2::field::{goldilocks_field::GoldilocksField, types::Field64};
use plonky2::hash::poseidon::PoseidonHash;
use plonky2::plonk::config::{GenericHashOut, Hasher as PoseidonHasher};
use serde::{Deserialize, Serialize};
use twox_hash::xxh3::HasherExt;

use crate::eval::{ContextEval, ContextInput, EvalCache, EvalHelpers};
use crate::felt::context_felt::ContextFelt;
use crate::ops::DPNOpType;

pub const SYM_FELT_REF_STORE_TYPE_MASK: u128 = 0xffff0000000000000000000000000000u128;
pub const SYM_FELT_REF_STORE_VALUE_MASK: u128 = 0x0000ffffffffffffffffffffffffffffu128;

pub const CONSTANT_TRUE_OP: u128 = (DPNOpType::ConstantTrue as u128) << 112;
pub const CONSTANT_FALSE_OP: u128 = (DPNOpType::ConstantFalse as u128) << 112;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Hash, PartialOrd, Ord, Eq, Copy)]
pub struct SymFeltRef(pub u128);

impl Display for SymFeltRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.get_target_hash_value())
    }
}

impl SymFeltRef {
    pub fn new_input(index: u64) -> SymFeltRef {
        SymFeltRef((DPNOpType::InputTarget as u128) << 112 | index as u128)
    }
    pub const fn new_constant(value: u64) -> SymFeltRef {
        //assert!(value < GoldilocksField::ORDER, "Constant value {} is too large", value);
        SymFeltRef((DPNOpType::Constant as u128) << 112 | (value % GoldilocksField::ORDER) as u128)
    }
    pub fn cns<T: Into<SymFeltRef>>(val: T) -> SymFeltRef {
        val.into()
    }
    pub fn new_constant_reduce(value: u128) -> SymFeltRef {
        SymFeltRef(
            (DPNOpType::Constant as u128) << 112
                | (value % (GoldilocksField::ORDER as u128)) as u128,
        )
    }
    pub fn is_constant_type(&self) -> bool {
        let op_type = self.get_op_type();
        op_type == DPNOpType::Constant
            || op_type == DPNOpType::ConstantTrue
            || op_type == DPNOpType::ConstantFalse
    }
    pub fn get_constant_value_multi(&self) -> u64 {
        let op_type = self.get_op_type();
        match op_type {
            DPNOpType::Constant => (self.0 & 0xffffffffffffffffu128) as u64,
            DPNOpType::ConstantTrue => 1,
            DPNOpType::ConstantFalse => 0,
            _ => panic!("Not a constant type"),
        }
    }
    pub fn get_constant_value_multi_u128(&self) -> u128 {
        self.get_constant_value_multi() as u128
    }

    pub fn get_constant_bool_value_multi(&self) -> bool {
        self.get_constant_value_multi() != 0
    }
    pub fn get_constant_value(&self) -> u64 {
        if self.0 == CONSTANT_TRUE_OP {
            1
        } else if self.0 == CONSTANT_FALSE_OP {
            0
        } else {
            (self.0 & 0xffffffffffffffffu128) as u64
        }
    }
    pub fn get_input_index(&self) -> u64 {
        (self.0 & 0xffffffffffffffffu128) as u64
    }
    pub fn get_target_hash_value(&self) -> u128 {
        self.0 & SYM_FELT_REF_STORE_VALUE_MASK
    }
    pub fn new_valueless(op_type: DPNOpType) -> SymFeltRef {
        SymFeltRef((op_type as u128) << 112)
    }

    pub fn get_op_type(&self) -> DPNOpType {
        ((self.0 >> 112) as u16).into()
    }
    pub fn needs_store(&self) -> bool {
        ((self.0 >> 112) as u16) > 1
    }
    pub fn constant_true() -> SymFeltRef {
        SymFeltRef((DPNOpType::ConstantTrue as u128) << 112)
    }
    pub fn constant_false() -> SymFeltRef {
        SymFeltRef((DPNOpType::ConstantFalse as u128) << 112)
    }
    pub fn constant_bool(val: bool) -> SymFeltRef {
        if val {
            SymFeltRef::constant_true()
        } else {
            SymFeltRef::constant_false()
        }
    }
    pub fn get_inline_def(&self) -> SymFeltDef {
        assert!(
            self.needs_store() == false,
            "Cannot get inline ref for non-store ref"
        );
        SymFeltDef {
            op_type: self.get_op_type(),
            const_param: self.get_constant_value(),
            inputs: vec![],
        }
    }
}

impl Add for SymFeltRef {
    type Output = Self;
    fn add(self, other: Self) -> Self {
        SymFeltRef::new_constant((self.get_u64() + other.get_u64()) % GoldilocksField::ORDER)
    }
}
impl Sub for SymFeltRef {
    type Output = Self;
    fn sub(self, other: Self) -> Self {
        SymFeltRef::new_constant(
            (GoldilocksField::from_noncanonical_u64(self.get_u64())
                - GoldilocksField::from_noncanonical_u64(other.get_u64()))
            .to_canonical_u64(),
        )
    }
}
impl Mul for SymFeltRef {
    type Output = Self;
    fn mul(self, other: Self) -> Self {
        SymFeltRef::new_constant(
            (GoldilocksField::from_noncanonical_u64(self.get_u64())
                * GoldilocksField::from_noncanonical_u64(other.get_u64()))
            .to_canonical_u64(),
        )
    }
}
impl Div for SymFeltRef {
    type Output = Self;
    fn div(self, other: Self) -> Self {
        SymFeltRef::new_constant(
            (GoldilocksField::from_noncanonical_u64(self.get_u64())
                / GoldilocksField::from_noncanonical_u64(other.get_u64()))
            .to_canonical_u64(),
        )
    }
}
impl Rem for SymFeltRef {
    type Output = Self;
    fn rem(self, other: Self) -> Self {
        SymFeltRef::new_constant(self.get_u64() % other.get_u64())
    }
}
impl BitAnd for SymFeltRef {
    type Output = Self;
    fn bitand(self, other: Self) -> Self {
        SymFeltRef::new_constant((self.get_u64() & other.get_u64()) & 0xFFFFFFFFu64)
    }
}
impl BitOr for SymFeltRef {
    type Output = Self;
    fn bitor(self, other: Self) -> Self {
        SymFeltRef::new_constant((self.get_u64() | other.get_u64()) & 0xFFFFFFFFu64)
    }
}
impl BitXor for SymFeltRef {
    type Output = Self;
    fn bitxor(self, other: Self) -> Self {
        SymFeltRef::new_constant((self.get_u64() ^ other.get_u64()) & 0xFFFFFFFFu64)
    }
}
impl Shl for SymFeltRef {
    type Output = Self;
    fn shl(self, other: Self) -> Self {
        SymFeltRef::new_constant((self.get_u64() << other.get_u64()) & 0xFFFFFFFFu64)
    }
}
impl Shr for SymFeltRef {
    type Output = Self;
    fn shr(self, other: Self) -> Self {
        SymFeltRef::new_constant((self.get_u64() >> other.get_u64()) & 0xFFFFFFFFu64)
    }
}
impl Not for SymFeltRef {
    type Output = Self;
    fn not(self) -> Self {
        SymFeltRef::new_constant((self.get_u64() == 0) as u64)
    }
}
impl Neg for SymFeltRef {
    type Output = Self;
    fn neg(self) -> Self {
        SymFeltRef::new_constant(
            GoldilocksField::from_noncanonical_u64(self.get_u64())
                .neg()
                .to_canonical_u64(),
        )
    }
}
impl AddAssign for SymFeltRef {
    fn add_assign(&mut self, other: Self) {
        *self =
            SymFeltRef::new_constant((self.get_u64() + other.get_u64()) % GoldilocksField::ORDER)
    }
}
impl SubAssign for SymFeltRef {
    fn sub_assign(&mut self, other: Self) {
        *self = SymFeltRef::new_constant(
            (GoldilocksField::from_canonical_u64(self.get_u64())
                - GoldilocksField::from_canonical_u64(other.get_u64()))
            .to_canonical_u64(),
        )
    }
}
impl MulAssign for SymFeltRef {
    fn mul_assign(&mut self, other: Self) {
        *self = SymFeltRef::new_constant(
            (GoldilocksField::from_canonical_u64(self.get_u64())
                * GoldilocksField::from_canonical_u64(other.get_u64()))
            .to_canonical_u64(),
        )
    }
}
impl DivAssign for SymFeltRef {
    fn div_assign(&mut self, other: Self) {
        *self = SymFeltRef::new_constant(
            (GoldilocksField::from_canonical_u64(self.get_u64())
                / GoldilocksField::from_canonical_u64(other.get_u64()))
            .to_canonical_u64(),
        )
    }
}
impl RemAssign for SymFeltRef {
    fn rem_assign(&mut self, other: Self) {
        *self = SymFeltRef::new_constant(self.get_u64() % other.get_u64())
    }
}
impl BitAndAssign for SymFeltRef {
    fn bitand_assign(&mut self, other: Self) {
        *self = SymFeltRef::new_constant((self.get_u64() & other.get_u64()) & 0xFFFFFFFFu64)
    }
}
impl BitOrAssign for SymFeltRef {
    fn bitor_assign(&mut self, other: Self) {
        *self = SymFeltRef::new_constant((self.get_u64() | other.get_u64()) & 0xFFFFFFFFu64)
    }
}
impl BitXorAssign for SymFeltRef {
    fn bitxor_assign(&mut self, other: Self) {
        *self = SymFeltRef::new_constant((self.get_u64() ^ other.get_u64()) & 0xFFFFFFFFu64)
    }
}
impl ShlAssign for SymFeltRef {
    fn shl_assign(&mut self, other: Self) {
        *self = SymFeltRef::new_constant((self.get_u64() << other.get_u64()) & 0xFFFFFFFFu64)
    }
}
impl ShrAssign for SymFeltRef {
    fn shr_assign(&mut self, other: Self) {
        *self = SymFeltRef::new_constant((self.get_u64() >> other.get_u64()) & 0xFFFFFFFFu64)
    }
}

impl Add<u64> for SymFeltRef {
    type Output = Self;
    fn add(self, other: u64) -> Self {
        SymFeltRef::new_constant((self.get_u64() + other) % GoldilocksField::ORDER)
    }
}
impl Sub<u64> for SymFeltRef {
    type Output = Self;
    fn sub(self, other: u64) -> Self {
        SymFeltRef::new_constant(
            (GoldilocksField::from_noncanonical_u64(self.get_u64())
                - GoldilocksField::from_noncanonical_u64(other))
            .to_canonical_u64(),
        )
    }
}
impl Mul<u64> for SymFeltRef {
    type Output = Self;
    fn mul(self, other: u64) -> Self {
        SymFeltRef::new_constant(
            (GoldilocksField::from_noncanonical_u64(self.get_u64())
                * GoldilocksField::from_noncanonical_u64(other))
            .to_canonical_u64(),
        )
    }
}
impl Div<u64> for SymFeltRef {
    type Output = Self;
    fn div(self, other: u64) -> Self {
        SymFeltRef::new_constant(
            (GoldilocksField::from_noncanonical_u64(self.get_u64())
                / GoldilocksField::from_noncanonical_u64(other))
            .to_canonical_u64(),
        )
    }
}
impl Rem<u64> for SymFeltRef {
    type Output = Self;
    fn rem(self, other: u64) -> Self {
        SymFeltRef::new_constant(self.get_u64() % other)
    }
}
impl BitAnd<u64> for SymFeltRef {
    type Output = Self;
    fn bitand(self, other: u64) -> Self {
        SymFeltRef::new_constant((self.get_u64() & other) & 0xFFFFFFFFu64)
    }
}
impl BitOr<u64> for SymFeltRef {
    type Output = Self;
    fn bitor(self, other: u64) -> Self {
        SymFeltRef::new_constant((self.get_u64() | other) & 0xFFFFFFFFu64)
    }
}
impl BitXor<u64> for SymFeltRef {
    type Output = Self;
    fn bitxor(self, other: u64) -> Self {
        SymFeltRef::new_constant((self.get_u64() ^ other) & 0xFFFFFFFFu64)
    }
}
impl Shl<u64> for SymFeltRef {
    type Output = Self;
    fn shl(self, other: u64) -> Self {
        SymFeltRef::new_constant((self.get_u64() << other) & 0xFFFFFFFFu64)
    }
}
impl Shr<u64> for SymFeltRef {
    type Output = Self;
    fn shr(self, other: u64) -> Self {
        SymFeltRef::new_constant((self.get_u64() >> other) & 0xFFFFFFFFu64)
    }
}

impl Add<SymFeltRef> for u64 {
    type Output = SymFeltRef;
    fn add(self, other: SymFeltRef) -> SymFeltRef {
        SymFeltRef::new_constant((self + other.get_u64()) % GoldilocksField::ORDER)
    }
}
impl Sub<SymFeltRef> for u64 {
    type Output = SymFeltRef;
    fn sub(self, other: SymFeltRef) -> SymFeltRef {
        SymFeltRef::new_constant(
            (GoldilocksField::from_noncanonical_u64(self)
                - GoldilocksField::from_noncanonical_u64(other.get_u64()))
            .to_canonical_u64(),
        )
    }
}
impl Mul<SymFeltRef> for u64 {
    type Output = SymFeltRef;
    fn mul(self, other: SymFeltRef) -> SymFeltRef {
        SymFeltRef::new_constant(
            (GoldilocksField::from_noncanonical_u64(self)
                * GoldilocksField::from_noncanonical_u64(other.get_u64()))
            .to_canonical_u64(),
        )
    }
}
impl Div<SymFeltRef> for u64 {
    type Output = SymFeltRef;
    fn div(self, other: SymFeltRef) -> SymFeltRef {
        SymFeltRef::new_constant(
            (GoldilocksField::from_noncanonical_u64(self)
                / GoldilocksField::from_noncanonical_u64(other.get_u64()))
            .to_canonical_u64(),
        )
    }
}
impl Rem<SymFeltRef> for u64 {
    type Output = SymFeltRef;
    fn rem(self, other: SymFeltRef) -> SymFeltRef {
        SymFeltRef::new_constant(self % other.get_u64())
    }
}
impl BitAnd<SymFeltRef> for u64 {
    type Output = SymFeltRef;
    fn bitand(self, other: SymFeltRef) -> SymFeltRef {
        SymFeltRef::new_constant((self & other.get_u64()) & 0xFFFFFFFFu64)
    }
}
impl BitOr<SymFeltRef> for u64 {
    type Output = SymFeltRef;
    fn bitor(self, other: SymFeltRef) -> SymFeltRef {
        SymFeltRef::new_constant((self | other.get_u64()) & 0xFFFFFFFFu64)
    }
}
impl BitXor<SymFeltRef> for u64 {
    type Output = SymFeltRef;
    fn bitxor(self, other: SymFeltRef) -> SymFeltRef {
        SymFeltRef::new_constant((self ^ other.get_u64()) & 0xFFFFFFFFu64)
    }
}
impl Shl<SymFeltRef> for u64 {
    type Output = SymFeltRef;
    fn shl(self, other: SymFeltRef) -> SymFeltRef {
        SymFeltRef::new_constant((self << other.get_u64()) & 0xFFFFFFFFu64)
    }
}
impl Shr<SymFeltRef> for u64 {
    type Output = SymFeltRef;
    fn shr(self, other: SymFeltRef) -> SymFeltRef {
        SymFeltRef::new_constant((self >> other.get_u64()) & 0xFFFFFFFFu64)
    }
}
impl AddAssign<SymFeltRef> for u64 {
    fn add_assign(&mut self, other: SymFeltRef) {
        *self = *self + other.get_u64() % GoldilocksField::ORDER
    }
}
impl SubAssign<SymFeltRef> for u64 {
    fn sub_assign(&mut self, other: SymFeltRef) {
        *self = (GoldilocksField::from_canonical_u64(*self)
            - GoldilocksField::from_canonical_u64(other.get_u64()))
        .to_canonical_u64()
    }
}
impl MulAssign<SymFeltRef> for u64 {
    fn mul_assign(&mut self, other: SymFeltRef) {
        *self = (GoldilocksField::from_canonical_u64(*self)
            * GoldilocksField::from_canonical_u64(other.get_u64()))
        .to_canonical_u64()
    }
}
impl DivAssign<SymFeltRef> for u64 {
    fn div_assign(&mut self, other: SymFeltRef) {
        *self = (GoldilocksField::from_canonical_u64(*self)
            / GoldilocksField::from_canonical_u64(other.get_u64()))
        .to_canonical_u64()
    }
}
impl RemAssign<SymFeltRef> for u64 {
    fn rem_assign(&mut self, other: SymFeltRef) {
        *self = *self % other.get_u64()
    }
}
impl BitAndAssign<SymFeltRef> for u64 {
    fn bitand_assign(&mut self, other: SymFeltRef) {
        *self = (*self & other.get_u64()) & 0xFFFFFFFFu64
    }
}
impl BitOrAssign<SymFeltRef> for u64 {
    fn bitor_assign(&mut self, other: SymFeltRef) {
        *self = (*self | other.get_u64()) & 0xFFFFFFFFu64
    }
}
impl BitXorAssign<SymFeltRef> for u64 {
    fn bitxor_assign(&mut self, other: SymFeltRef) {
        *self = (*self ^ other.get_u64()) & 0xFFFFFFFFu64
    }
}
impl ShlAssign<SymFeltRef> for u64 {
    fn shl_assign(&mut self, other: SymFeltRef) {
        *self = (*self << other.get_u64()) & 0xFFFFFFFFu64
    }
}
impl ShrAssign<SymFeltRef> for u64 {
    fn shr_assign(&mut self, other: SymFeltRef) {
        *self = (*self >> other.get_u64()) & 0xFFFFFFFFu64
    }
}
impl AddAssign<u64> for SymFeltRef {
    fn add_assign(&mut self, other: u64) {
        *self = SymFeltRef::new_constant((self.get_u64() + other) % GoldilocksField::ORDER)
    }
}
impl SubAssign<u64> for SymFeltRef {
    fn sub_assign(&mut self, other: u64) {
        *self = SymFeltRef::new_constant(
            (GoldilocksField::from_canonical_u64(self.get_u64())
                - GoldilocksField::from_canonical_u64(other))
            .to_canonical_u64(),
        )
    }
}
impl MulAssign<u64> for SymFeltRef {
    fn mul_assign(&mut self, other: u64) {
        *self = SymFeltRef::new_constant(
            (GoldilocksField::from_canonical_u64(self.get_u64())
                * GoldilocksField::from_canonical_u64(other))
            .to_canonical_u64(),
        )
    }
}
impl DivAssign<u64> for SymFeltRef {
    fn div_assign(&mut self, other: u64) {
        *self = SymFeltRef::new_constant(
            (GoldilocksField::from_canonical_u64(self.get_u64())
                / GoldilocksField::from_canonical_u64(other))
            .to_canonical_u64(),
        )
    }
}
impl RemAssign<u64> for SymFeltRef {
    fn rem_assign(&mut self, other: u64) {
        *self = SymFeltRef::new_constant(self.get_u64() % other)
    }
}
impl BitAndAssign<u64> for SymFeltRef {
    fn bitand_assign(&mut self, other: u64) {
        *self = SymFeltRef::new_constant((self.get_u64() & other) & 0xFFFFFFFFu64)
    }
}
impl BitOrAssign<u64> for SymFeltRef {
    fn bitor_assign(&mut self, other: u64) {
        *self = SymFeltRef::new_constant((self.get_u64() | other) & 0xFFFFFFFFu64)
    }
}
impl BitXorAssign<u64> for SymFeltRef {
    fn bitxor_assign(&mut self, other: u64) {
        *self = SymFeltRef::new_constant((self.get_u64() ^ other) & 0xFFFFFFFFu64)
    }
}
impl ShlAssign<u64> for SymFeltRef {
    fn shl_assign(&mut self, other: u64) {
        *self = SymFeltRef::new_constant((self.get_u64() << other) & 0xFFFFFFFFu64)
    }
}
impl ShrAssign<u64> for SymFeltRef {
    fn shr_assign(&mut self, other: u64) {
        *self = SymFeltRef::new_constant((self.get_u64() >> other) & 0xFFFFFFFFu64)
    }
}
impl PartialEq<u64> for SymFeltRef {
    fn eq(&self, other: &u64) -> bool {
        self.get_u64() == *other
    }
}
impl PartialOrd<u64> for SymFeltRef {
    fn partial_cmp(&self, other: &u64) -> Option<std::cmp::Ordering> {
        self.get_u64().partial_cmp(other)
    }
}
impl From<u8> for SymFeltRef {
    fn from(val: u8) -> SymFeltRef {
        SymFeltRef((val as u128) | ((DPNOpType::Constant as u128) << 112))
    }
}
impl From<u16> for SymFeltRef {
    fn from(val: u16) -> SymFeltRef {
        SymFeltRef((val as u128) | ((DPNOpType::Constant as u128) << 112))
    }
}
impl From<u32> for SymFeltRef {
    fn from(val: u32) -> SymFeltRef {
        SymFeltRef((val as u128) | ((DPNOpType::Constant as u128) << 112))
    }
}

impl From<u64> for SymFeltRef {
    fn from(val: u64) -> SymFeltRef {
        SymFeltRef((val as u128) | ((DPNOpType::Constant as u128) << 112))
    }
}

impl From<i32> for SymFeltRef {
    fn from(val: i32) -> SymFeltRef {
        assert!(val >= 0, "Negative values are not supported");
        SymFeltRef((val as u128) | ((DPNOpType::Constant as u128) << 112))
    }
}
impl From<i64> for SymFeltRef {
    fn from(val: i64) -> SymFeltRef {
        assert!(val >= 0, "Negative values are not supported");
        SymFeltRef((val as u128) | ((DPNOpType::Constant as u128) << 112))
    }
}
impl From<bool> for SymFeltRef {
    fn from(val: bool) -> SymFeltRef {
        SymFeltRef::constant_bool(val)
    }
}

impl ContextFelt for SymFeltRef {
    fn cns(value: u64) -> Self {
        SymFeltRef::new_constant(value)
    }
    fn cns_inverse(value: u64) -> Self {
        SymFeltRef::new_constant(
            GoldilocksField::from_noncanonical_u64(value)
                .inverse()
                .to_canonical_u64(),
        )
    }

    fn get_u64(&self) -> u64 {
        self.get_constant_value()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Hash, PartialOrd, Ord, Eq)]
pub struct SymFeltRefValue {
    pub op_type: DPNOpType,
    pub const_param: u64,
    pub inputs: Vec<SymFeltRef>,
}

impl SymFeltRefValue {
    pub fn get_ref_key(&self) -> SymFeltRef {
        if self.op_type == DPNOpType::Constant || self.op_type == DPNOpType::InputTarget {
            return SymFeltRef(((self.op_type as u128) << 112) | self.const_param as u128);
        } else {
            let mut hasher = twox_hash::Xxh3Hash128::default();
            hasher.write(&bincode::serialize(&self).unwrap());
            SymFeltRef(
                (hasher.finish_ext() & SYM_FELT_REF_STORE_VALUE_MASK)
                    | ((self.op_type as u128) << 112),
            )
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, Eq, PartialEq)]
pub struct SymRefAssertion {
    pub left: SymFeltRef,
    pub right: SymFeltRef,
    pub message: &'static str,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, Eq, PartialEq)]
pub struct SetSymFeltRef {
    pub before_external_function_call: u16,
    pub index: SymFeltRef,
    pub value: SymFeltRef,
}

impl SetSymFeltRef {
    pub fn new(
        before_external_function_call: u16,
        index: SymFeltRef,
        value: SymFeltRef,
    ) -> SetSymFeltRef {
        SetSymFeltRef {
            before_external_function_call,
            index: index,
            value: value,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Hash, PartialOrd, Ord, Eq)]
pub struct SymFeltDef {
    pub op_type: DPNOpType,
    pub const_param: u64,
    pub inputs: Vec<SymFeltDef>,
}

#[derive(Debug, Clone)]
pub struct SymFeltStore {
    pub store: HashMap<SymFeltRef, SymFeltRefValue>,
}

impl SymFeltStore {
    pub fn new() -> SymFeltStore {
        SymFeltStore {
            store: HashMap::new(),
        }
    }
    pub fn get_opt(&self, key: SymFeltRef) -> Option<&SymFeltRefValue> {
        self.store.get(&key)
    }

    pub fn get(&self, key: SymFeltRef) -> &SymFeltRefValue {
        self.store.get(&key).unwrap()
    }

    pub fn insert(&mut self, value: SymFeltRefValue) -> SymFeltRef {
        let key = value.get_ref_key();
        if key.needs_store() && !self.store.contains_key(&key) {
            self.store.insert(key, value);
        }
        key
    }

    pub fn contains(&self, key: SymFeltRef) -> bool {
        self.store.contains_key(&key)
    }
    pub fn get_direct_children(&self, key: SymFeltRef) -> Vec<SymFeltRef> {
        let mut result = vec![];
        if key.needs_store() {
            let base = self.get(key);
            for input in base.inputs.iter() {
                result.push(*input);
            }
        }
        result
    }
    pub fn get_def(&self, key: SymFeltRef) -> SymFeltDef {
        if key.needs_store() {
            let base = self.get(key);
            SymFeltDef {
                op_type: base.op_type,
                const_param: base.const_param,
                inputs: base.inputs.iter().map(|x| self.get_def(*x)).collect(),
            }
        } else {
            key.get_inline_def()
        }
    }
}

fn split_bits(x: u64, num_bits: u64) -> Vec<u64> {
    let mut result = vec![0u64; num_bits as usize];
    for i in 0..num_bits {
        result[i as usize] = (x >> i) & 1;
    }
    result
}
fn sum_bits(bits: &[u64]) -> u64 {
    assert!(bits.len() <= 64, "cannot sum more than 64 bits");
    let result = bits.iter().fold(0, |acc, x| acc + x);
    GoldilocksField::from_noncanonical_u64(result).to_canonical_u64()
}

impl EvalHelpers for SymFeltStore {
    fn resolve_binary_felt_args<I: ContextInput, C: EvalCache>(
        &self,
        parent: SymFeltRef,
        input: &I,
        cache: &mut C,
    ) -> (u64, u64) {
        let resolved = &self.get(parent).inputs;
        assert_eq!(resolved.len(), 2);
        let left = self.resolve_felt_ref_cached(resolved[0], input, cache);
        let right = self.resolve_felt_ref_cached(resolved[1], input, cache);
        (left, right)
    }
    fn resolve_unary_felt_arg<I: ContextInput, C: EvalCache>(
        &self,
        parent: SymFeltRef,
        input: &I,
        cache: &mut C,
    ) -> u64 {
        let resolved = &self.get(parent).inputs;
        assert_eq!(resolved.len(), 1);
        self.resolve_felt_ref_cached(resolved[0], input, cache)
    }

    fn resolve_array_args<I: ContextInput, C: EvalCache>(
        &self,
        parent: SymFeltRef,
        input: &I,
        cache: &mut C,
    ) -> Vec<u64> {
        let resolved = &self.get(parent).inputs;
        resolved
            .iter()
            .map(|felt_ref| self.resolve_felt_ref_cached(*felt_ref, input, cache))
            .collect()
    }
}

impl ContextEval for SymFeltStore {
    fn resolve_felt_ref_cached<I: ContextInput, C: EvalCache>(
        &self,
        felt_ref: SymFeltRef,
        input: &I,
        cache: &mut C,
    ) -> u64 {
        if felt_ref.is_constant_type() {
            felt_ref.get_constant_value()
        } else if cache.contains(felt_ref) {
            cache.get(felt_ref)
        } else {
            let op_type = felt_ref.get_op_type();
            let result = match op_type {
                DPNOpType::InputTarget => input.get_input(felt_ref.get_input_index()),
                DPNOpType::Constant => felt_ref.get_constant_value(),
                DPNOpType::ConstantTrue => 1,
                DPNOpType::ConstantFalse => 0,
                DPNOpType::Add => {
                    let (a, b) = self.resolve_binary_felt_args_gl(felt_ref, input, cache);
                    (a + b).to_canonical_u64()
                }
                DPNOpType::Sub => {
                    let (a, b) = self.resolve_binary_felt_args_gl(felt_ref, input, cache);
                    (a - b).to_canonical_u64()
                }
                DPNOpType::Mul => {
                    let (a, b) = self.resolve_binary_felt_args_gl(felt_ref, input, cache);
                    (a * b).to_canonical_u64()
                }
                DPNOpType::Div => {
                    let (a, b) = self.resolve_binary_felt_args_gl(felt_ref, input, cache);
                    (a / b).to_canonical_u64()
                }
                DPNOpType::BoolNot => {
                    (self.resolve_unary_felt_arg(felt_ref, input, cache) == 0) as u64
                }
                DPNOpType::BoolAnd => {
                    let (a, b) = self.resolve_binary_felt_args(felt_ref, input, cache);
                    ((a != 0) && (b != 0)) as u64
                }
                DPNOpType::BoolOr => {
                    let (a, b) = self.resolve_binary_felt_args(felt_ref, input, cache);
                    ((a != 0) || (b != 0)) as u64
                }
                DPNOpType::Xor => {
                    let (a, b) = self.resolve_binary_felt_args(felt_ref, input, cache);
                    (a ^ b) & 0xFFFFFFFFu64
                }
                DPNOpType::Nor => {
                    let (a, b) = self.resolve_binary_felt_args(felt_ref, input, cache);
                    (!(a | b)) & 0xFFFFFFFFu64
                }
                DPNOpType::Eq => {
                    let (a, b) = self.resolve_binary_felt_args(felt_ref, input, cache);
                    (a == b) as u64
                }
                DPNOpType::Lte => {
                    let (a, b) = self.resolve_binary_felt_args(felt_ref, input, cache);
                    (a <= b) as u64
                }
                DPNOpType::Gte => {
                    let (a, b) = self.resolve_binary_felt_args(felt_ref, input, cache);
                    (a >= b) as u64
                }
                DPNOpType::Gt => {
                    let (a, b) = self.resolve_binary_felt_args(felt_ref, input, cache);
                    (a > b) as u64
                }
                DPNOpType::Lt => {
                    let (a, b) = self.resolve_binary_felt_args(felt_ref, input, cache);
                    (a < b) as u64
                }
                DPNOpType::SplitBits => panic!("you cannot directly evaluate SumBits"),
                DPNOpType::SumBits => sum_bits(&self.resolve_array_args(felt_ref, input, cache)),
                DPNOpType::TargetAt => {
                    let base = &self.get(felt_ref).inputs;
                    let index = self.resolve_felt_ref_cached(base[1], input, cache);
                    let array = self.resolve_array_ref_cached(base[0], input, cache);
                    assert!(index < array.len() as u64, "index out of bounds");
                    array[index as usize]
                }
                DPNOpType::HashNoPad => panic!("you cannot directly evaluate HashNoPad"),
                DPNOpType::HashPad => panic!("you cannot directly evaluate HashPad"),
                DPNOpType::Select => {
                    let args = self.resolve_array_args(felt_ref, input, cache);
                    if args[0] != 0 {
                        args[1]
                    } else {
                        args[2]
                    }
                }
                DPNOpType::Exp => {
                    let (base, exponent) = self.resolve_binary_felt_args_gl(felt_ref, input, cache);
                    base.exp_u64(exponent.to_canonical_u64()).to_canonical_u64()
                }
                DPNOpType::ExpConstantPower => panic!("ExpConstantPower is not implemented"),
                DPNOpType::ExpConstantBase => panic!("ExpConstantBase is not implemented"),
                DPNOpType::Mod => {
                    let (a, b) = self.resolve_binary_felt_args(felt_ref, input, cache);
                    a % b
                }
                DPNOpType::ModConstantDividend => panic!("ModConstantDividend is not implemented"),
                DPNOpType::ModConstantDivisor => panic!("ModConstantDivisor is not implemented"),
                DPNOpType::DivRem4 => {
                    todo!("DivRem4 is not implemented");
                }
                DPNOpType::CastU32 => {
                    let value = self.resolve_unary_felt_arg(felt_ref, input, cache);
                    value & 0xFFFFFFFFu64
                }
                DPNOpType::U32And => {
                    let (a, b) = self.resolve_binary_felt_args(felt_ref, input, cache);
                    (a & b) & 0xFFFFFFFFu64
                }
                DPNOpType::U32AndConstant => todo!("U32AndConstant is not implemented"),
                DPNOpType::U32Or => {
                    let (a, b) = self.resolve_binary_felt_args(felt_ref, input, cache);
                    (a | b) & 0xFFFFFFFFu64
                }
                DPNOpType::U32OrConstant => todo!("U32OrConstant is not implemented"),
                DPNOpType::U32Xor => {
                    let (a, b) = self.resolve_binary_felt_args(felt_ref, input, cache);
                    (a ^ b) & 0xFFFFFFFFu64
                }
                DPNOpType::U32XorConstant => todo!("U32XorConstant is not implemented"),
                DPNOpType::U32ShiftLeft => {
                    let (a, b) = self.resolve_binary_felt_args(felt_ref, input, cache);
                    (a << b) & 0xFFFFFFFFu64
                }
                DPNOpType::U32ShiftLeftConstantBitDistance => {
                    todo!("U32ShiftLeftConstantBitDistance is not implemented")
                }
                DPNOpType::U32ShiftLeftConstantValue => {
                    todo!("U32ShiftLeftConstantValue is not implemented")
                }
                DPNOpType::U32ShiftRight => {
                    let (a, b) = self.resolve_binary_felt_args(felt_ref, input, cache);
                    (a >> b) & 0xFFFFFFFFu64
                }
                DPNOpType::U32ShiftRightConstantBitDistance => {
                    todo!("U32ShiftLeftConstantValue is not implemented")
                }
                DPNOpType::U32ShiftRightConstantValue => {
                    todo!("U32ShiftLeftConstantValue is not implemented")
                }
                DPNOpType::CalculateMerkleRoot => todo!("CalculateMerkleRoot is not implemented"),
                DPNOpType::GetUserId => todo!(),
                DPNOpType::GetContractId => todo!(),
                DPNOpType::GetCheckpointId => todo!(),
                DPNOpType::GetNonce => todo!(),
                DPNOpType::GetUserPublicKeyHash => todo!(),
                DPNOpType::GetStateQueryResult => todo!(),
                DPNOpType::GetStateQueryResultSingle => todo!(),
                DPNOpType::UnaryInverse => self
                    .resolve_unary_felt_arg_gl(felt_ref, input, cache)
                    .inverse()
                    .to_canonical_u64(),
                DPNOpType::UnaryNegative => self
                    .resolve_unary_felt_arg_gl(felt_ref, input, cache)
                    .neg()
                    .to_canonical_u64(),
                DPNOpType::GetStateCommandResultHash => todo!(),
                DPNOpType::GetStateCommandResultSingle => todo!(),
                DPNOpType::GetStateCommandResultArray => todo!(),
            };
            result
        }
    }

    fn resolve_array_ref_cached<I: ContextInput, C: EvalCache>(
        &self,
        felt_ref: SymFeltRef,
        input: &I,
        cache: &mut C,
    ) -> Box<Vec<u64>> {
        if cache.contains_arr(felt_ref) {
            cache.get_arr_ref(felt_ref)
        } else {
            let result = match felt_ref.get_op_type() {
                DPNOpType::SplitBits => {
                    let (x, num_bits) = self.resolve_binary_felt_args(felt_ref, input, cache);
                    let bits = split_bits(x, num_bits);
                    bits
                }
                DPNOpType::HashNoPad => {
                    let data = self.resolve_array_args_gl(felt_ref, input, cache);
                    let result = PoseidonHash::hash_no_pad(&data).to_vec();
                    result.iter().map(|x| x.to_canonical_u64()).collect()
                }
                DPNOpType::HashPad => {
                    let data = self.resolve_array_args_gl(felt_ref, input, cache);
                    let result = PoseidonHash::hash_pad(&data).to_vec();
                    result.iter().map(|x| x.to_canonical_u64()).collect()
                }
                DPNOpType::GetUserPublicKeyHash => todo!(),
                _ => panic!("you cannot directly evaluate an array ref"),
            };

            cache.insert_arr(felt_ref, result);
            cache.get_arr_ref(felt_ref)
        }
    }
}
