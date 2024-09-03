use std::ops::{
    Add, AddAssign, BitAnd, BitAndAssign, BitOr, BitOrAssign, BitXor, BitXorAssign, Div, DivAssign,
    Mul, MulAssign, Neg, Not, Rem, RemAssign, Shl, ShlAssign, Shr, ShrAssign, Sub, SubAssign,
};

pub trait ContextFelt:
    Copy
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
    + PartialOrd<u64> {
    fn cns(value: u64) -> Self;
    fn cns_inverse(value: u64) -> Self;
    fn get_u64(&self) -> u64;
}

pub trait DPNContext<F: ContextFelt> {
    fn op_cast_u32(&mut self, a: F) -> F;
    fn op_select(&mut self, condition: F, a: F, b: F) -> F;
    fn op_const(&mut self, value: u64) -> F;
    fn op_bool_not(&mut self, a: F) -> F;
    fn op_bool_and(&mut self, a: F, b: F) -> F;
    fn op_bool_or(&mut self, a: F, b: F) -> F;
    fn op_bool_or_many(&mut self, values: &[F]) -> F;
    fn op_bool_and_many(&mut self, values: &[F]) -> F;
    fn op_add(&mut self, a: F, b: F) -> F;
    fn op_sub(&mut self, a: F, b: F) -> F;
    fn op_mul(&mut self, a: F, b: F) -> F;
    fn op_div(&mut self, a: F, b: F) -> F;
    fn op_mod(&mut self, a: F, b: F) -> F;
    fn op_exp(&mut self, a: F, b: F) -> F;
    fn op_eq(&mut self, a: F, b: F) -> F;
    fn op_neq(&mut self, a: F, b: F) -> F;
    fn op_lt(&mut self, a: F, b: F) -> F;
    fn op_lte(&mut self, a: F, b: F) -> F;
    fn op_gt(&mut self, a: F, b: F) -> F;
    fn op_gte(&mut self, a: F, b: F) -> F;

    // start u32 ops
    fn op_u32_xor(&mut self, a: F, b: F) -> F;
    fn op_u32_or(&mut self, a: F, b: F) -> F;
    fn op_u32_and(&mut self, a: F, b: F) -> F;
    fn op_u32_shl(&mut self, a: F, b: F) -> F;
    fn op_u32_shr(&mut self, a: F, b: F) -> F;

    // end u32 ops
    fn op_true(&mut self) -> F;

    fn op_false(&mut self) -> F;

    fn add_input(&mut self) -> F;
    fn add_inputs(&mut self, count: u64) -> Vec<F>;
    fn assert_eq(&mut self, left: F, right: F, message: &'static str);
    fn assert_true(&mut self, left: F, message: &'static str);
    fn cset(&mut self, old_value: F, new_value: F) -> F;
    fn start_if_block(&mut self, condition: F);
    fn start_else_if_block(&mut self, condition: F);
    fn start_else_block(&mut self);
    fn end_if_block(&mut self);
    fn resolve_current_condition(&mut self) -> F;
    fn pop_condition(&mut self);

    // std lib
    fn hash(&mut self, values: &[F]) -> [F; 4];
    fn split_bits(&mut self, value: F, num_bits: u64) -> Vec<F>;
    fn sum_bits(&mut self, bits: &[F]) -> F;

    fn get_user_id(&mut self) -> F;
    fn get_contract_id(&mut self) -> F;
    fn get_checkpoint_id(&mut self) -> F;
    fn get_last_nonce(&mut self) -> F;
    fn get_user_public_key_hash(&mut self) -> [F; 4];
}

