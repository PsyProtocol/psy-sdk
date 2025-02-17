use crate::IdentId;
use lazy_static::lazy_static;
use once_cell::sync::Lazy;
use smol_str::SmolStr;
use std::collections::HashMap;
use std::sync::Mutex;

//global hashmap to store id to SmolStr
lazy_static! {
    pub static ref GLOBAL_IDENT_TABLE: Mutex<HashMap<IdentId, SmolStr>> =
        Mutex::new(HashMap::new());
}
pub fn insert_ident<S: AsRef<str>>(id: usize, ident: S) {
    let mut global_index = GLOBAL_IDENT_TABLE.lock().unwrap();
    global_index
        .entry(IdentId(id))
        .or_insert_with(|| SmolStr::new(ident.as_ref()));
}

//todo! cfg with test
pub fn get_ident(id: IdentId) -> Option<SmolStr> {
    let global_index = GLOBAL_IDENT_TABLE.lock().unwrap();
    global_index.get(&id).cloned()
    // #[cfg(not(test))]
    // {
    //     Some(SmolStr::new(&id.to_string()))
    // }
}
