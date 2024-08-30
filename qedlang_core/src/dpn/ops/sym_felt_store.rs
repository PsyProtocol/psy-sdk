use super::sym_felt::{SymFeltRef, SymFeltRefValue};
#[derive(Debug, Clone)]
pub struct SymFeltStore {
    pub store: hashbrown::HashMap<SymFeltRef, SymFeltRefValue>,
}

impl SymFeltStore {
    pub fn new() -> SymFeltStore {
        SymFeltStore {
            store: hashbrown::HashMap::new(),
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
}