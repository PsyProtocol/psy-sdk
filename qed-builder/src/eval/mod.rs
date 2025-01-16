use plonky2::field::{goldilocks_field::GoldilocksField, types::Field};

use crate::SymFeltRef;

pub mod cache;

pub trait EvalCache {
    fn contains(&self, key: SymFeltRef) -> bool;
    fn get(&self, key: SymFeltRef) -> u64;
    fn insert(&mut self, key: SymFeltRef, value: u64);
    fn contains_arr(&self, key: SymFeltRef) -> bool;
    fn get_arr_ref(&self, key: SymFeltRef) -> Box<Vec<u64>>;
    fn insert_arr(&mut self, key: SymFeltRef, value: Vec<u64>);
}

pub trait ContextInput {
    fn get_input(&self, index: u64) -> u64;
}

pub trait ContextEval {
    fn resolve_felt_ref_cached<I: ContextInput, C: EvalCache>(
        &self,
        felt_ref: SymFeltRef,
        input: &I,
        cache: &mut C,
    ) -> u64;
    fn resolve_array_ref_cached<I: ContextInput, C: EvalCache>(
        &self,
        felt_ref: SymFeltRef,
        input: &I,
        cache: &mut C,
    ) -> Box<Vec<u64>>;
}

pub trait EvalHelpers: ContextEval {
    fn resolve_binary_felt_args<I: ContextInput, C: EvalCache>(
        &self,
        parent: SymFeltRef,
        input: &I,
        cache: &mut C,
    ) -> (u64, u64);
    fn resolve_unary_felt_arg<I: ContextInput, C: EvalCache>(
        &self,
        parent: SymFeltRef,
        input: &I,
        cache: &mut C,
    ) -> u64;
    fn resolve_binary_felt_args_gl<I: ContextInput, C: EvalCache>(
        &self,
        parent: SymFeltRef,
        input: &I,
        cache: &mut C,
    ) -> (GoldilocksField, GoldilocksField) {
        let (a, b) = self.resolve_binary_felt_args(parent, input, cache);
        (
            GoldilocksField::from_noncanonical_u64(a),
            GoldilocksField::from_noncanonical_u64(b),
        )
    }
    fn resolve_unary_felt_arg_gl<I: ContextInput, C: EvalCache>(
        &self,
        parent: SymFeltRef,
        input: &I,
        cache: &mut C,
    ) -> GoldilocksField {
        let resolved = self.resolve_unary_felt_arg(parent, input, cache);
        GoldilocksField::from_noncanonical_u64(resolved)
    }
    fn resolve_array_args<I: ContextInput, C: EvalCache>(
        &self,
        parent: SymFeltRef,
        input: &I,
        cache: &mut C,
    ) -> Vec<u64>;
    fn resolve_array_args_gl<I: ContextInput, C: EvalCache>(
        &self,
        parent: SymFeltRef,
        input: &I,
        cache: &mut C,
    ) -> Vec<GoldilocksField> {
        let resolved = self.resolve_array_args(parent, input, cache);
        resolved
            .iter()
            .map(|x| GoldilocksField::from_noncanonical_u64(*x))
            .collect()
    }
}
