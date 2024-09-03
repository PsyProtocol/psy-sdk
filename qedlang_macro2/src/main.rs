mod rwv;
use quote::ToTokens;
use rwv::RewriterVisitor;
use syn::{fold::Fold, ItemImpl};
static FOO1: &str = r#"
impl ExampleContract2 {
    pub fn new() -> ExampleContract2 {
        ExampleContract2 {}
    }

    pub fn inc_counter_small(&mut self, ctx: &mut QExecContext, a: u64, b: u64) {
        let mut c = a * b;
        if a < b {
            c = a + b;
        }
        //ctx.assert_eq(c,100, "c must be less than 100");
        assert!(c < 100, "c must be less than 100");
        
    }
}
"#;
static FOO: &str = r#"
impl<C: DPNContext<Felt>> SimpleContractStateless<C> {
    pub fn simple_math(&mut self, ctx: &mut C, a: Felt, b: Felt) -> Felt {
        let k = (a + 2) * 4 * b - 3 * (a + b);
        let z = k + a;

        ctx.assert_true(z > 3, "z must be gt than 3");
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


fn main() {
    let res = syn::parse_str::<ItemImpl>(FOO).unwrap();

    let mut visitor = RewriterVisitor {};
    let res = visitor.fold_item_impl(res);

    println!("{}", res.to_token_stream().to_string());
}
