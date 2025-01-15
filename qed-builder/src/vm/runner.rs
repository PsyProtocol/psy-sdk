use plonky2::hash::hash_types::RichField;

use super::{
    def::DPNFunctionCircuitDefinition,
    exec::{IExecutionContext, IExtendedExecutionContext, SimpleDPNExecutor},
};

pub struct SimpleVMRunner<F: RichField> {
    c: SimpleDPNExecutor<F>,
}

impl<F: RichField> SimpleVMRunner<F> {
    pub fn new(c: SimpleDPNExecutor<F>) -> Self {
        SimpleVMRunner { c }
    }

    pub fn resolve_queries() {
        todo!()
    }

    pub fn step(&mut self) -> usize {
        let index = self.c.definition_index;
        // step_eval_circuit_function_vm_def(this.c, d);
        let d = self.c.ctx.def.definitions[index].clone();
        self.c.process_var_def(&d);
        self.c.definition_index = index + 1;
        return index; // returns the index of the definition evaluated
    }

    pub fn execute_until(&mut self, definition_index: usize) -> usize {
        let mut cur_index = self.c.definition_index;
        let defs_len = self.c.ctx.def.definitions.len();
        while cur_index < definition_index && cur_index < defs_len {
            cur_index = self.step();
        }
        return cur_index;
    }
    pub fn finish(&mut self) -> Vec<F> {
        let def = &self.c.ctx.def;
        // let exec_inputs = &self.c.ctx.exec_inputs;
        // let execution_context = &self.c.ctx.execution_context;

        for i in 0..def.assertions.len() {
            let a = def.assertions[i].clone();
            let left_result = self.c.resolve_target(a.left);
            let right_result = self.c.resolve_target(a.right);

            assert_eq!(
                left_result, right_result,
                "equality assertion failed (left: {:?}, right {:?}): {:?}",
                left_result, right_result, a.message
            );
        }
        def.circuit_outputs
            .iter()
            .map(|&c| self.c.resolve_target(c))
            .collect()
    }

    pub fn exec_precomputed_query_results(&mut self) -> Vec<F> {
        self.execute_until(self.c.ctx.def.definitions.len() - 1);
        self.finish()
    }
}

pub fn exec_circuit_function_vm<F: RichField>(
    exec_inputs: Vec<u64>,
    def: DPNFunctionCircuitDefinition,
    execution_context: IExecutionContext<F>,
    // circuit_context: QCircuitContext,
) -> Vec<F> {
    let extended_ctx = IExtendedExecutionContext::new(exec_inputs, def, execution_context);
    let vm_ctx = SimpleDPNExecutor::new_with_ctx(extended_ctx);
    let mut vm_runner = SimpleVMRunner::new(vm_ctx);
    // let queryResults = await queryResolver.resolveStateQuery(vm_runner, def.queries);
    vm_runner.exec_precomputed_query_results()
}
