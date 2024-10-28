use std::collections::HashMap;

use crate::{DefId, DefinitionNode, ScopeId};

#[derive(Clone, Debug)]
pub struct Scope<K, V> {
    variables: HashMap<K, V>,
    parent: Option<ScopeId>,
    definitions: HashMap<DefId, DefinitionNode>,
    children: Vec<ScopeId>,
}

impl<K: std::hash::Hash + Eq + Clone, V> Scope<K, V> {
    pub fn new(parent: Option<ScopeId>) -> Self {
        Self {
            variables: HashMap::with_capacity(10),
            parent,
            definitions: HashMap::new(),
            children: vec![],
        }
    }
}

#[derive(Clone, Debug)]
pub struct SymbolTable<K, V> {
    scopes: Vec<Scope<K, V>>,
    scope_stack: Vec<Vec<ScopeId>>,
}

impl<K: std::fmt::Display, V: std::fmt::Display> std::fmt::Display for SymbolTable<K, V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Ok(())
    }
}

impl<K: std::hash::Hash + Eq + Clone, V> SymbolTable<K, V> {
    pub fn new() -> Self {
        SymbolTable {
            scopes: vec![Scope::new(None)],
            scope_stack: vec![vec![ScopeId::root()]],
        }
    }

    pub fn start_scope(&mut self) {
        let current_scope_id = *self.scope_stack.last().unwrap().last().unwrap();
        let child_scope_id = ScopeId(self.scopes.len());
        self.scopes.push(Scope::new(Some(current_scope_id)));
        self.scopes[current_scope_id.0]
            .children
            .push(child_scope_id);
        self.scope_stack.last_mut().unwrap().push(child_scope_id);
    }

    pub fn end_scope(&mut self) {
        self.scope_stack.last_mut().unwrap().pop();
    }

    pub fn start_function(&mut self) {
        let current_scope_id = *self.scope_stack.last().unwrap().last().unwrap();
        let child_scope_id = ScopeId(self.scopes.len());
        self.scopes.push(Scope::new(Some(current_scope_id)));
        self.scopes[current_scope_id.0]
            .children
            .push(child_scope_id);
        self.scope_stack.push(vec![ScopeId::root(), child_scope_id]);
    }

    pub fn end_function(&mut self) {
        self.scope_stack.pop();
    }

    pub fn get_var<Q: ?Sized>(&mut self, key: &Q) -> Option<&V>
    where
        K: std::borrow::Borrow<Q>,
        Q: std::hash::Hash + Eq,
    {
        for scope in self.scope_stack.last().unwrap().iter().rev() {
            if let Some(value) = self.scopes[scope.0].variables.get(key) {
                return Some(value);
            }
        }
        if self.scope_stack.len() > 1 {
            for scope in self.scope_stack.first().unwrap().iter().rev() {
                if let Some(value) = self.scopes[scope.0].variables.get(key) {
                    return Some(value);
                }
            }
        }
        None
    }

    pub fn set_var(&mut self, key: K, value: V) {
        for scope in self.scope_stack.last_mut().unwrap().iter_mut().rev() {
            if let Some(v) = self.scopes[scope.0].variables.get_mut(&key) {
                *v = value;
                return;
            }
        }
        if self.scope_stack.len() > 1 {
            for scope in self.scope_stack.first_mut().unwrap().iter_mut().rev() {
                if let Some(v) = self.scopes[scope.0].variables.get_mut(&key) {
                    *v = value;
                    return;
                }
            }
        }
    }

    pub fn define_var(&mut self, key: K, value: V) -> Option<V> {
        let current_scope_id = *self.scope_stack.last().unwrap().last().unwrap();
        self.scopes[current_scope_id.0].variables.insert(key, value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_symbol_table() {
        let table: SymbolTable<String, i32> = SymbolTable::new();
        assert_eq!(table.scope_stack.len(), 1);
    }

    #[test]
    fn test_start_and_end_scope() {
        let mut table: SymbolTable<String, i32> = SymbolTable::new();
        table.start_scope();
        assert_eq!(table.scope_stack[0].len(), 2);
        table.end_scope();
        assert_eq!(table.scope_stack[0].len(), 1);
    }

    #[test]
    fn test_start_and_end_function() {
        let mut table: SymbolTable<String, i32> = SymbolTable::new();
        table.start_function();
        assert_eq!(table.scope_stack.len(), 2);
        table.end_function();
        assert_eq!(table.scope_stack.len(), 1);
    }

    #[test]
    fn test_scope_set_and_get() {
        let mut table: SymbolTable<String, i32> = SymbolTable::new();
        table.start_scope();
        table.define_var("key".to_string(), 42);
        assert_eq!(table.get_var("key"), Some(&42));

        table.start_scope();
        assert_eq!(table.get_var("key"), Some(&42));
        table.define_var("key".to_string(), 24);
        assert_eq!(table.get_var("key"), Some(&24));
        table.end_scope();
        assert_eq!(table.get_var("key"), Some(&42));
        table.end_scope();
    }

    #[test]
    fn test_function_set_and_get() {
        let mut table: SymbolTable<String, i32> = SymbolTable::new();
        table.start_function();
        table.define_var("key".to_string(), 42);
        assert_eq!(table.get_var("key"), Some(&42));
        table.start_function();
        assert_eq!(table.get_var("key"), None);
        table.set_var("key".to_string(), 24);
        assert_eq!(table.get_var("key"), None);
        table.end_function();
        assert_eq!(table.get_var("key"), Some(&42));
        table.end_function();
    }

    #[test]
    fn test_global_value() {
        let mut table: SymbolTable<String, i32> = SymbolTable::new();
        table.define_var("key".to_string(), 42);
        table.start_scope();
        assert_eq!(table.get_var("key"), Some(&42));
        table.end_scope();
        table.start_function();
        assert_eq!(table.get_var("key"), Some(&42));
        table.set_var("key".to_string(), 24);
        table.end_function();
        assert_eq!(table.get_var("key"), Some(&24));
    }
}
