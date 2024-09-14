use proc_macro2::TokenTree;
use quote::{ToTokens, TokenStreamExt};

use syn::{
    fold::{self, Fold}, parse_quote, punctuated::Punctuated, token::Token, Attribute, BinOp, Block, Expr, ExprBinary, ExprBlock, ExprIf, ExprIndex, ExprMethodCall, ImplItem, ImplItemFn, Item, ItemImpl, Local, Stmt, Token
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Ord, PartialOrd, Hash)]
pub enum QRefMode {
    Normal = 0,
    ImmutableRef = 1,
    MutableRef = 2,
}
pub struct RewriterVisitor {
    pub mode_stack: Vec<QRefMode>,
    pub contract_state_mode_stack: Vec<bool>,
}
impl RewriterVisitor {
    pub fn new() -> Self {
        Self {
            mode_stack: vec![QRefMode::Normal],
            contract_state_mode_stack: vec![false],
        }
    }
    pub fn push_ref_mode(&mut self, mode: QRefMode) {
        self.mode_stack.push(mode);
    }
    pub fn pop_ref_mode(&mut self) -> QRefMode {
        self.mode_stack.pop().unwrap()
    }
    pub fn get_ref_mode(&self) -> QRefMode {
        *self.mode_stack.last().unwrap()
    }
    pub fn push_contract_state_mode(&mut self, mode: bool){
        self.contract_state_mode_stack.push(mode);
    }
    pub fn pop_contract_state_mode(&mut self) -> bool {
        self.contract_state_mode_stack.pop().unwrap()
    }
    pub fn get_contract_state_mode(&self) -> bool {
        *self.contract_state_mode_stack.last().unwrap()
    }
}

fn is_else_if(e: Option<(Token![else], Box<Expr>)>) -> bool {
    if e.is_some() {
        let (_, e) = e.unwrap();
        if let Expr::If(_) = *e {
            return true;
        }
    }
    false
}
fn is_self_state_var(e: &Expr) -> bool {
    println!("is_self_state_var: {:#?}", e.to_token_stream());
    let tokens = e.to_token_stream();
    let token_vec = tokens.into_iter().take(4).collect::<Vec<_>>();
    if token_vec.len() > 3 {
        if let TokenTree::Ident(x) = &token_vec[0] {
            println!("x0: {}", x.to_string());
            if x.to_string() == "self" {
                if let TokenTree::Punct(x) = &token_vec[1] {
                    println!("x1: {}", x.to_string());
                    if x.as_char() == '.' {
                        if let TokenTree::Ident(x) = &token_vec[2] {
                            println!("x2: {}", x.to_string());
                            if x.to_string() == "state" {
                                return true;
                            }
                        }
                    }
                }
            }
        }
    }
    false
}
impl RewriterVisitor {
    // returns (Vec<(Condition, Block)>, Option<Block>)
    fn flatten_else_ifs(&mut self, e: ExprIf) -> (Vec<(Expr, Block)>, Option<Expr>) {
        let root_cond = self.fold_expr(*e.cond);
        let root_then = self.fold_block(e.then_branch);
        let mut ifs = vec![(root_cond, root_then)];
        let mut next = e.else_branch;
        while next.is_some() {
            let (_, next_expr) = next.unwrap().into();
            let next_expr = *next_expr;
            match next_expr {
                Expr::If(ife) => {
                    let cond = self.fold_expr(*ife.cond);
                    let then_branch = self.fold_block(ife.then_branch);
                    ifs.push((cond, then_branch));
                    next = ife.else_branch;
                }
                _ => return (ifs, Some(self.fold_expr(next_expr))),
            }
        }
        (ifs, None)
    }
    fn exprify_else_ifs(&mut self, ifs: Vec<(Expr, Block)>, else_block: Option<Expr>) -> Expr {
        let mut stmts = Vec::new();
        let mut is_first = true;
        for (cond, block) in ifs {
            if is_first {
                let stmt: Stmt = parse_quote! {
                    {
                        let qed_rwv_cond = #cond;
                        ctx.start_if_block(qed_rwv_cond);
                        #block
                    }
                };
                stmts.push(stmt);
                is_first = false;
            } else {
                let stmt: Stmt = parse_quote! {
                    {
                        let qed_rwv_cond = #cond;
                        ctx.start_else_if_block(qed_rwv_cond);
                        #block
                    }
                };
                stmts.push(stmt);
            }
        }
        if let Some(else_block) = else_block {
            let stmt: Stmt = parse_quote! {
                {
                    ctx.start_else_block();
                    #else_block
                }
            };
            stmts.push(stmt);
        }
        let block: ExprBlock = parse_quote!({
            #(#stmts)*

            ctx.end_if_block();
        });
        Expr::Block(block)
    }

    fn collasce_else_if(&mut self, e: ExprIf) -> Expr {
        if e.else_branch.is_none() {
            let qed_rwv_cond = self.fold_expr(*e.cond);
            let then_branch = self.fold_block(e.then_branch);
            parse_quote! {
                {
                    let qed_rwv_cond = #qed_rwv_cond;
                    ctx.start_if_block(qed_rwv_cond);
                    #then_branch
                    ctx.end_if_block();
                }
            }
        } else {
            let (ifs, else_block) = self.flatten_else_ifs(e);
            self.exprify_else_ifs(ifs, else_block)
        }
    }

    fn ctx_bin_op(&mut self, op: Box<ExprBinary>) -> Expr {
        let bop = op.op;
        let left = self.fold_expr(*op.left.clone());
        let right = self.fold_expr(*op.right.clone());
        match bop {
            BinOp::Add(_) => parse_quote!({
                let tmp_left = (#left);
                let tmp_right = (#right);
                ctx.op_add(tmp_left, tmp_right)
            }),
            BinOp::Sub(_) => parse_quote!({
                let tmp_left = (#left);
                let tmp_right = (#right);
                ctx.op_sub(tmp_left, tmp_right)
            }),
            BinOp::Mul(_) => parse_quote!({
                let tmp_left = (#left);
                let tmp_right = (#right);
                ctx.op_mul(tmp_left, tmp_right)
            }),
            BinOp::Div(_) => parse_quote!({
                let tmp_left = (#left);
                let tmp_right = (#right);
                ctx.op_div(tmp_left, tmp_right)
            }),
            BinOp::Eq(_) => parse_quote!({
                let tmp_left = (#left);
                let tmp_right = (#right);
                ctx.op_eq(tmp_left, tmp_right)
            }),
            BinOp::Lt(_) => parse_quote!({
                let tmp_left = (#left);
                let tmp_right = (#right);
                ctx.op_lt(tmp_left, tmp_right)
            }),
            BinOp::Le(_) => parse_quote!({
                let tmp_left = (#left);
                let tmp_right = (#right);
                ctx.op_lte(tmp_left, tmp_right)
            }),
            BinOp::Ne(_) => parse_quote!({
                let tmp_left = (#left);
                let tmp_right = (#right);
                ctx.op_neq(tmp_left, tmp_right)
            }),
            BinOp::Gt(_) => parse_quote!({
                let tmp_left = (#left);
                let tmp_right = (#right);
                ctx.op_gt(tmp_left, tmp_right)
            }),
            BinOp::Ge(_) => parse_quote!({
                let tmp_left = (#left);
                let tmp_right = (#right);
                ctx.op_gte(tmp_left, tmp_right)
            }),
            BinOp::Rem(_) => parse_quote!({
                let tmp_left = (#left);
                let tmp_right = (#right);
                ctx.op_mod(tmp_left, tmp_right)
            }),
            BinOp::And(_) => parse_quote!({
                let tmp_left = (#left);
                let tmp_right = (#right);
                ctx.op_bool_and(tmp_left, tmp_right)
            }),
            BinOp::Or(_) => parse_quote!({
                let tmp_left = (#left);
                let tmp_right = (#right);
                ctx.op_bool_or(tmp_left, tmp_right)
            }),
            BinOp::BitXor(_) => parse_quote!({
                let tmp_left = (#left);
                let tmp_right = (#right);
                ctx.op_xor_u32(tmp_left, tmp_right)
            }),
            BinOp::BitAnd(_) => parse_quote!({
                let tmp_left = (#left);
                let tmp_right = (#right);
                ctx.op_and_u32(tmp_left, tmp_right)
            }),
            BinOp::BitOr(_) => parse_quote!({
                let tmp_left = (#left);
                let tmp_right = (#right);
                ctx.op_or_u32(tmp_left, tmp_right)
            }),
            BinOp::Shl(_) => parse_quote!({
                let tmp_left = (#left);
                let tmp_right = (#right);
                ctx.op_shl_u32(tmp_left, tmp_right)
            }),
            BinOp::Shr(_) => parse_quote!({
                let tmp_left = (#left);
                let tmp_right = (#right);
                ctx.op_shr_u32(tmp_left, tmp_right)
            }),
    
            _ => Expr::Binary(*op.clone()),
            /*
            BinOp::AddAssign(_) => todo!(),
            BinOp::SubAssign(_) => todo!(),
            BinOp::MulAssign(_) => todo!(),
            BinOp::DivAssign(_) => todo!(),
            BinOp::RemAssign(_) => todo!(),
            BinOp::BitXorAssign(_) => todo!(),
            BinOp::BitAndAssign(_) => todo!(),
            BinOp::BitOrAssign(_) => todo!(),
            BinOp::ShlAssign(_) => todo!(),
            BinOp::ShrAssign(_) => todo!(),
            _ => todo!(),*/
        }
    }
    fn ctx_bin_op_maybe_assign(&mut self, op: Box<ExprBinary>) -> Expr {
        let bop = op.op;
        let left = &op.left;
        let right = &op.right;
        match bop {
            BinOp::AddAssign(_) => {
                self.fold_expr(parse_quote!({
                    let tmp_val = #left + #right;
                    #left = tmp_val;
                }))
            }

            BinOp::SubAssign(_) => {
                self.fold_expr(parse_quote!({
                    let tmp_val = #left - #right;
                    #left = tmp_val;
                }))
            }
            BinOp::MulAssign(_) => {
                self.fold_expr(parse_quote!({
                    let tmp_val = #left * #right;
                    #left = tmp_val;
                }))
            }
            BinOp::DivAssign(_) => {
                self.fold_expr(parse_quote!({
                    let tmp_val = #left / #right;
                    #left = tmp_val;
                }))
            }
            BinOp::RemAssign(_) => {
                self.fold_expr(parse_quote!({
                    let tmp_val = #left % #right;
                    #left = tmp_val;
                }))
            }
            BinOp::BitXorAssign(_) => {
                self.fold_expr(parse_quote!({
                    let tmp_val = #left ^ #right;
                    #left = tmp_val;
                }))
            }
            BinOp::BitAndAssign(_) => {
                self.fold_expr(parse_quote!({
                    let tmp_val = #left & #right;
                    #left = tmp_val;
                }))
            }
            BinOp::BitOrAssign(_) => {
                self.fold_expr(parse_quote!({
                    let tmp_val = #left | #right;
                    #left = tmp_val;
                }))
            }
            BinOp::ShlAssign(_) => {
                self.fold_expr(parse_quote!({
                    let tmp_val = #left << #right;
                    #left = tmp_val;
                }))
            }
            BinOp::ShrAssign(_) => {
                self.fold_expr(parse_quote!({
                    let tmp_val = #left >> #right;
                    #left = tmp_val;
                }))
            }
    
            _ => self.ctx_bin_op(op),
            /*
            BinOp::AddAssign(_) => todo!(),
            BinOp::SubAssign(_) => todo!(),
            BinOp::MulAssign(_) => todo!(),
            BinOp::DivAssign(_) => todo!(),
            BinOp::RemAssign(_) => todo!(),
            BinOp::BitXorAssign(_) => todo!(),
            BinOp::BitAndAssign(_) => todo!(),
            BinOp::BitOrAssign(_) => todo!(),
            BinOp::ShlAssign(_) => todo!(),
            BinOp::ShrAssign(_) => todo!(),
            _ => todo!(),*/
        }
    }
}
impl Fold for RewriterVisitor {
    fn fold_expr(&mut self, e: Expr) -> Expr {
        match e {
            Expr::Range(r) => {
                // disable dynamic range expressions
                Expr::Range(r)
            }
            Expr::Loop(_) => {
                panic!("loop {{ ... }} expressions are not allowed in qedlang");
            }
            Expr::If(e) => self.collasce_else_if(e),
            /* 
            Expr::Field(_)=>{
                if is_self_state_var(&e){
                    self.push_contract_state_mode(true);

                    let result = fold::fold_expr(self, e);
                    self.pop_contract_state_mode();
                    result

                }else{
                    e
                }
            }*/
            Expr::Index(ex)=>{
                println!("index: {}",ex.to_token_stream().to_string());
                if is_self_state_var(&Expr::Index(ex.clone())){
                    self.push_contract_state_mode(true);
                    let expr_result = self.fold_expr(*ex.expr.clone());
                    self.pop_contract_state_mode();
                    self.push_contract_state_mode(false);
                    let index_result = self.fold_expr(*ex.index.clone());
                    self.pop_contract_state_mode();
                    println!("here: {}",expr_result.to_token_stream().to_string());
                    parse_quote!({
                        let q_tmp_index_result = #index_result;
                        #expr_result.q_get(ctx, q_tmp_index_result)
                    })
                    //Expr::Index(ExprIndex { attrs: ex.attrs, expr: Box::new(expr_result), bracket_token: ex.bracket_token, index: Box::new(index_result) })

                }else{
                    Expr::Index(ex)
                }
            }
            Expr::Assign(ex) => {
                let is_state = is_self_state_var(&ex.left);
                let left = self.fold_expr(*ex.left);
                let right = self.fold_expr(*ex.right);

                if is_state {
                    parse_quote!({
                        let qed_rwv_old_value = (#left).clone();
                        let qed_rwv_new_value = (#right).clone();
                        ctx.cset_state(qed_rwv_old_value, qed_rwv_new_value);
                    })

                }else{
                    parse_quote!({
                        let qed_rwv_old_value = (#left).clone();
                        let qed_rwv_new_value = (#right).clone();
                        #left = ctx.cset(qed_rwv_old_value, qed_rwv_new_value);
                    })
                }
            }
            Expr::Binary(e) => {
                println!("Binary expression: {:#?}", e.to_token_stream());
                self.ctx_bin_op(Box::new(e))
            }
            Expr::Reference(e) => {
                if e.mutability.is_some() {
                    self.push_ref_mode(QRefMode::MutableRef);
                } else {
                    self.push_ref_mode(QRefMode::ImmutableRef);
                }
                let result = self.fold_expr(*e.expr);
                self.pop_ref_mode();
                result
            }
            Expr::Unary(e) => match &e.op {
                syn::UnOp::Not(_) => {
                    let expr = self.fold_expr(*e.expr);
                    return parse_quote! {
                        ctx.op_not(#expr)
                    };
                }
                syn::UnOp::Neg(_) => {
                    let expr = self.fold_expr(*e.expr);
                    return parse_quote! {
                        ctx.op_neg(#expr)
                    };
                }
                _ => Expr::Unary(e),
            },
            Expr::Lit(ex) => match ex.lit {
                syn::Lit::Int(_) => {
                    let new_expr: Expr = parse_quote! {
                        SymFeltRef::cns(#ex)
                    };
                    new_expr
                }
                syn::Lit::Bool(b) => {
                    parse_quote! {
                        SymFeltRef::constant_bool(#b)
                    }
                }
                _ => Expr::Lit(ex),
            },
            Expr::MethodCall(e) => {
                /*
                convert:
                ctx.example(a*2, b+3)

                to:
                {
                    let tmp_arg_0 = (a*2).into();
                    let tmp_arg_1 = (b+3).into();
                    ctx.example(tmp_arg_0, tmp_arg_1)
                }
                 */
                println!("Method call expression: {:#?}", e.to_token_stream());
                let receiver = Box::new(self.fold_expr(*e.receiver));
                let args: Vec<Expr> = e.args.into_iter().map(|arg| self.fold_expr(arg)).collect();
                let method = e.method;

                let mut stmts = Vec::new();
                let mut new_args = Vec::new();

                for (i, arg) in args.iter().enumerate() {
                    let tmp_ident = syn::Ident::new(
                        &format!("tmp_arg_{}", i),
                        proc_macro2::Span::call_site().into(),
                    );
                    let stmt: Stmt = parse_quote! {
                        let #tmp_ident = (#arg);
                    };
                    if let Expr::Path(x) = arg.clone() {
                        if x.path.segments.len() == 1 {
                            let seg = x.path.segments.first().unwrap();
                            if seg.ident == "ctx" {
                                new_args.push(seg.ident.clone().into());
                                continue;
                            }
                        }
                    } else {
                        stmts.push(stmt);
                        new_args.push(tmp_ident);
                    }
                }

                let new_method_call: Expr = parse_quote! {
                    #receiver.#method(#(#new_args),*)
                };

                let new_block: ExprBlock = parse_quote!({
                        #(#stmts)*
                        #new_method_call
                });
                Expr::Block(new_block)
            }
            _ => fold::fold_expr(self, e),
        }
    }

    fn fold_stmt(&mut self, s: Stmt) -> Stmt {
        match s {
            _ => fold::fold_stmt(self, s),
        }
    }
}
