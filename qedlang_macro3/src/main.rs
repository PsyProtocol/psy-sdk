mod rwv;
mod heap1;
mod feltsized1;
mod heapsize;
mod state_init;
use feltsized1::derive_felt_sized_core;
use heap1::derive_heap_size_core;
use quote::ToTokens;
use rwv::RewriterVisitor;
use state_init::derive_state_init_core;
use syn::{fold::Fold, DeriveInput, ItemImpl};

static FOO: &str = r#"
#[derive(QStateInitializable)]
pub struct SimpleContractState {
    pub x: Felt,
    pub y: Felt,
    pub z: SparseArray<Felt, 12>,
}
"#;

static FOO_2: &str = r#"
impl<C: DPNContext<Felt>> SimpleContractStateful<C> {
    pub fn simple_set(&mut self, ctx: &mut C, a: Felt, b: Felt) -> Felt {
        if self.state.x > a {
            self.state.x.set_state(ctx, a*b);
        }else{
            self.state.y.set_state(ctx, a*b);
        }
        self.state.x
    }
}

"#;


fn main() {
    let res = syn::parse_str::<DeriveInput>(FOO).unwrap();

    let res = derive_state_init_core(res);

    println!("{}", res.to_token_stream().to_string());


    let res = syn::parse_str::<ItemImpl>(FOO_2).unwrap();

    let mut visitor = RewriterVisitor {};
    let res = visitor.fold_item_impl(res);

    println!("{}", res.to_token_stream().to_string());
}
