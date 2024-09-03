use crate::dpn::ops::{exec_context::QExecContext, sym_felt::SymFeltRef};

use super::{cache::SimpleEvalCache, simple::DummyContextEvalInput, traits::ContextEval};

pub fn exec_eval_simple(inputs: Vec<u64>, ctx: &QExecContext, output: Option<Vec<SymFeltRef>>) -> Vec<u64>{
    let mut cache = SimpleEvalCache::new();
    let input = DummyContextEvalInput::new(inputs);
    for assertion in ctx.assertions.iter() {
        let left = ctx.store.resolve_felt_ref_cached(assertion.left, &input, &mut cache);
        let right = ctx.store.resolve_felt_ref_cached(assertion.right, &input, &mut cache);
        assert_eq!(left, right, "Assertion failed: {}", assertion.message);
    }
    if let Some(output) = output {
        output.iter().map(|felt_ref| ctx.store.resolve_felt_ref_cached(*felt_ref, &input, &mut cache)).collect()
    } else {
        vec![]
    }
}