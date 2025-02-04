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


static NO_STATE: &str = r#"
#[qcontract]
impl<C: DPNContext<Felt>> SimpleContractStateful<C> {
    pub fn simple_math(&mut self, ctx: &mut C, a: Felt, b: Felt) -> Felt {
        let k = (a + 2) * 4 * b - 3 * (a + b);
        let z = k + a;
        //ctx.op_set_state_felt(a, b);
        let abc = [z, z, b, b];
        ctxa.assert_true(z*2, "Test");
        //ctx.cset_state_hash_at(a, abc);

        ctx.assert_true(z > 12, "z must be gt than 12");
        z
    }
    pub fn if_test_2(&mut self, ctx: &mut C, a: Felt, b: Felt) -> Felt {
        let mut c = a * b;
        let mut k = 123;
        if a < b {
            c = a + b;
        } else if a == b {
            c = a;
        } else if a == 1337 {
            c = b;
        } else {
            k = 456;
        }
        c + k
    }
}

"#;
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
            self.state.x = a*b;
        }else{
            self.state.y.set_state(ctx, a*b);
        }
        self.state.x
    }
}

"#;

static FOO_3: &str = r#"
impl<C: DPNContext<Felt>> SimpleContractStateful<C> {
    pub fn simple_set(&mut self, ctx: &mut C, a: Felt, b: Felt) -> Felt {
        if self.state.z[a] > a {
            self.state.z[2*b] = a*b;
        }else{
            self.state.y.set_state(ctx, a*b);
        }
        self.state.z[a+b]
    }
}

"#;


fn main() {
    let res = syn::parse_str::<DeriveInput>(FOO).unwrap();

    let res = derive_state_init_core(res);

    println!("{}", res.to_token_stream().to_string());


    let res = syn::parse_str::<ItemImpl>(NO_STATE).unwrap();

    let mut visitor = RewriterVisitor::new();
    let res = visitor.fold_item_impl(res);

    println!("{}", res.to_token_stream().to_string());
}
