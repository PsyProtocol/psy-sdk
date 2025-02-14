use once_cell::sync::Lazy;
use smol_str::SmolStr;
use std::collections::HashMap;
use std::sync::Mutex;

//global hashmap to store id to SmolStr
pub static GLOBAL_INDEX: Lazy<Mutex<HashMap<usize, SmolStr>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

pub fn push(id: usize, ident: &str) {
    let mut global_index = GLOBAL_INDEX.lock().unwrap();
    global_index.insert(id, SmolStr::new(ident));
}

pub fn pushes(idents: &[(usize, &str)]) {
    let mut global_index = GLOBAL_INDEX.lock().unwrap();
    for (id, ident) in idents {
        global_index.insert(*id, SmolStr::new(ident));
    }
}

//todo! cfg with test
pub fn get_ident(id: usize) -> Option<SmolStr> {
    #[cfg(test)]
    {
        let global_index = GLOBAL_INDEX.lock().unwrap();
        global_index.get(&id).cloned()
    }
    #[cfg(not(test))]
    {
        Some(SmolStr::new(&id.to_string()))
    }
}
