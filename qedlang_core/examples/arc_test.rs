use std::sync::{Arc, RwLock};

use qedlang_core::dpn::ops::sym_felt::SymFeltRef;
#[derive(Debug, Clone)]
struct ContextA {
    pub fields: Vec<SymFeltRef>,
    pub touched_inds: Vec<usize>,
}
impl ContextA {
    pub fn new() -> Self {
        Self {
            fields: Vec::new(),
            touched_inds: Vec::new(),
        }
    }
    pub fn add(&mut self, field: SymFeltRef) {
        self.fields.push(field);
    }
}

#[derive(Default, Debug, Clone)]
struct ArrayA<T: Clone> {
    pub data: Vec<T>,
}

impl<T: Clone> ArrayA<T> {
    pub fn get(&self, ctx: &mut ContextA, index: usize) -> T{
        ctx.touched_inds.push(index);
        self.data[index].clone()
    }
    pub fn new(data: Vec<T>) -> Self{
        Self {
            data,
        }
    }
}
#[derive(Default, Debug, Clone)]
struct User {
    pub favorite_numbers: ArrayA<u32>,
}
impl User {
    pub fn new(favorite_numbers: Vec<u32>) -> Self{
        Self {
            favorite_numbers: ArrayA::new(favorite_numbers),
        }
    }
}
struct StateA {
    pub users: ArrayA<User>,
}
impl StateA {
    pub fn example() -> Self {
        let favs = (0..100).map((|i| i as u32)).collect::<Vec<_>>();
        let users = (0..32).map(|_| User::new(favs.clone())).collect::<Vec<_>>();
        Self {
            users: ArrayA::new(users),
        }
    }
}
struct ContractA {
    pub state: StateA,
}
impl ContractA {
    pub fn new() -> Self {
        Self {
            state: StateA::example(),
        }
    }
    pub fn test_fnc(&mut self, ctx: &mut ContextA, user_index: usize, fav_index: usize) -> u32 {
        {self.state.users.get(ctx, user_index)}.favorite_numbers.get(ctx, fav_index)
    }
}
fn main() {
    let mut ca = ContractA::new();
    let mut ctx = ContextA::new();
    let v = ca.test_fnc(&mut ctx, 1, 3);
    println!("v = {}",v);



}