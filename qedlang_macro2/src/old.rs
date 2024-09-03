static FOO: &str = r#"
impl ExampleContract2 {
    pub fn new() -> ExampleContract2 {
        ExampleContract2 {}
    }

    #[trace_var]
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

use quote::{ToTokens, TokenStreamExt};

use proc_macro2::TokenStream;
use syn::{punctuated::Punctuated, Attribute, Block, Expr, ExprBinary, ImplItem, ImplItemFn, Item, ItemImpl, Local, Stmt};
pub struct RewriterVisitor {}
impl RewriterVisitor {
    pub fn visit_item(&mut self, item: &Item) -> Item {
        item.clone()
    }
    pub fn visit_local(&mut self, loc: &Local) -> Local {
        if loc.init.is_some() {

            let mut loc_init = loc.init.as_ref().unwrap().to_owned();
            let expr = Box::new(self.visit_expr(&loc_init.expr));
            let diverge = match loc_init.diverge {
                Some((el, e)) => Some((el, Box::new(self.visit_expr(&e)))),
                None => None,
            };
            loc_init.expr = expr;
            loc_init.diverge = diverge;

            let mut loc_copy = loc.clone();
            loc_copy.init = Some(loc_init);
            loc_copy
        }else{
            loc.clone()
        }
    }
    pub fn visit_stmt(&mut self, stmt: &Stmt) -> Stmt {
        match stmt {
            Stmt::Local(loc) => Stmt::Local(self.visit_local(loc)),
            Stmt::Item(item) => Stmt::Item(self.visit_item(item)),
            Stmt::Expr(e, t) => Stmt::Expr(self.visit_expr(e), t.clone()),
            Stmt::Macro(x) => Stmt::Macro(x.clone()),
        }

    }
    pub fn visit_binary_expression(&mut self, exp: &ExprBinary) -> ExprBinary {
        exp.clone()
    }
    pub fn visit_block(&mut self, block: &Block) -> Block {
        
        Block { brace_token: block.brace_token.clone(), stmts: block.stmts.iter().map(|x| self.visit_stmt(x)).collect() }
    
    }
    pub fn visit_expr(&mut self, expr: &Expr) -> Expr {

    match expr {
        Expr::Array(x) => {
            let mut dv = x.clone();
            let items = x.elems.iter().map(|item| self.visit_expr(item));

            dv.elems.clear();
            dv.elems.extend(items);
            Expr::Array(dv)
        },
        Expr::Assign(x) => {
            let left = self.visit_expr(&x.left);
            let right = self.visit_expr(&x.right);
            let mut copy = x.clone();
            copy.left = left.into();
            copy.right = right.into();
            Expr::Assign(copy)
        },
        Expr::Async(b) => {
            let mut expr_block = b.clone();

            let new_block = self.visit_block(&expr_block.block);
            expr_block.block = new_block;
            Expr::Async(expr_block)
        },
        Expr::Await(aw) => {
            let mut new_aw = aw.clone();
            new_aw.base = Box::new(self.visit_expr(&aw.base));
            Expr::Await(new_aw)
        },
        Expr::Binary(exp) => Expr::Binary(self.visit_binary_expression(exp)),
        Expr::Block(b) => {
            let mut expr_block = b.clone();

            let new_block = self.visit_block(&expr_block.block);
            expr_block.block = new_block;
            Expr::Block(expr_block)
        },
        Expr::Break(br) => {
            let mut new_br = br.clone();
            new_br.expr = match &br.expr {
                Some(x) => Some(Box::new(self.visit_expr(&x))),
                None => None,
            };
            Expr::Break(new_br)
        },
        Expr::Call(_) => todo!(),
        Expr::Cast(_) => todo!(),
        Expr::Closure(_) => todo!(),
        Expr::Const(_) => todo!(),
        Expr::Continue(_) => todo!(),
        Expr::Field(_) => todo!(),
        Expr::ForLoop(_) => todo!(),
        Expr::Group(_) => todo!(),
        Expr::If(_) => todo!(),
        Expr::Index(_) => todo!(),
        Expr::Infer(_) => todo!(),
        Expr::Let(_) => todo!(),
        Expr::Lit(_) => todo!(),
        Expr::Loop(_) => todo!(),
        Expr::Macro(_) => todo!(),
        Expr::Match(_) => todo!(),
        Expr::MethodCall(_) => todo!(),
        Expr::Paren(_) => todo!(),
        Expr::Path(_) => todo!(),
        Expr::Range(_) => todo!(),
        Expr::Reference(_) => todo!(),
        Expr::Repeat(_) => todo!(),
        Expr::Return(_) => todo!(),
        Expr::Struct(_) => todo!(),
        Expr::Try(_) => todo!(),
        Expr::TryBlock(_) => todo!(),
        Expr::Tuple(_) => todo!(),
        Expr::Unary(_) => todo!(),
        Expr::Unsafe(_) => todo!(),
        Expr::Verbatim(_) => todo!(),
        Expr::While(_) => todo!(),
        Expr::Yield(_) => todo!(),
        _ => todo!(),
    }
    }
}
fn rewrite_impl_item(impl_item: ImplItem) -> ImplItem {
    match impl_item {
        ImplItem::Fn(x) => {
            let mut visitor = RewriterVisitor{};
            let new_block = visitor.visit_block(&x.block);

            let mut new_impl = x.clone();

            new_impl.block = new_block;
            ImplItem::Fn(new_impl)
        },
        alt => alt,
    }

}
fn rewrite_item_impl(item: ItemImpl) -> ItemImpl{

    let mut new_item = item;

    let items = new_item.items.drain(..).map(|x|{
        rewrite_impl_item(x)
    }).collect::<Vec<_>>();
    
    new_item.items = items;



    new_item
    


}

fn main() {
    let res = syn::parse_str::<ItemImpl>(FOO).unwrap();

    

    println!("{:#?}", res.to_token_stream());

    println!("Hello, world!");
}