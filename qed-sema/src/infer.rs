use std::collections::HashMap;

use crate::TypeId;

pub struct InferCtxt {
    contexts: Vec<Vec<HashMap<TypeId, TypeId>>>,
}

impl InferCtxt {
    pub fn new() -> Self {
        InferCtxt {
            contexts: vec![vec![HashMap::new()]],
        }
    }

    pub fn enter_context(&mut self) {
        self.contexts.push(vec![HashMap::new()]);
    }

    pub fn exit_context(&mut self) {
        self.contexts.pop();
    }

    pub fn enter_scope(&mut self) {
        self.contexts.last_mut().unwrap().push(HashMap::new());
    }

    pub fn exit_scope(&mut self) {
        self.contexts.last_mut().unwrap().pop();
    }

    pub fn has_equations(&self) -> bool {
        self.contexts.last().unwrap().iter().any(|x| !x.is_empty())
    }

    pub fn probe(&self, type_id: TypeId) -> Option<TypeId> {
        self.contexts
            .last()
            .unwrap()
            .iter()
            .rev()
            .find_map(|x| x.get(&type_id))
            .cloned()
    }

    pub fn equate(&mut self, lhs_ty: TypeId, rhs_ty: TypeId) {
        self.contexts
            .last_mut()
            .unwrap()
            .last_mut()
            .unwrap()
            .insert(lhs_ty, rhs_ty);
    }
}
