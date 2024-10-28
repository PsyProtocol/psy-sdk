use std::{collections::HashMap, ops::Index, rc::Rc};

use crate::{ExprNode, Ident, StmtNode};

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct ExprId(pub usize);

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct StmtId(pub usize);

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct IdentId(pub usize);

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct DefId(pub usize);

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct ScopeId(pub usize);

impl ScopeId {
    pub const fn root() -> Self {
        ScopeId(0)
    }
}

#[derive(Clone, Debug)]
pub struct Arena<F> {
    pub exprs: Vec<Rc<ExprNode<F>>>,
    pub stmts: Vec<Rc<StmtNode>>,
    pub idents: HashMap<Ident, IdentId>,
}

impl<F> Arena<F> {
    pub fn new() -> Self {
        Self {
            exprs: vec![],
            stmts: vec![],
            idents: HashMap::new(),
        }
    }

    pub fn alloc_ident<S: Into<Ident>>(&mut self, ident: S) -> IdentId {
        let ident = ident.into();
        if let Some(&value) = self.idents.get(&ident) {
            return value;
        }
        let identid = IdentId(self.idents.len());
        self.idents.insert(ident, identid);
        identid
    }

    pub fn alloc_stmt(&mut self, stmt: StmtNode) -> StmtId {
        self.stmts.push(Rc::new(stmt));
        StmtId(self.stmts.len() - 1)
    }

    pub fn alloc_expr(&mut self, expr: ExprNode<F>) -> ExprId {
        self.exprs.push(Rc::new(expr));
        ExprId(self.exprs.len() - 1)
    }

    pub fn alloc_idents<S: Into<Ident>>(&mut self, idents: Vec<S>) -> Vec<IdentId> {
        let mut result = vec![];
        for i in idents.into_iter() {
            result.push(self.alloc_ident(i));
        }
        result
    }

    pub fn alloc_stmts(&mut self, stmts: Vec<StmtNode>) -> Vec<StmtId> {
        let mut result = vec![];
        for i in stmts.into_iter() {
            result.push(self.alloc_stmt(i));
        }
        result
    }

    pub fn alloc_exprs(&mut self, exprs: Vec<ExprNode<F>>) -> Vec<ExprId> {
        let mut result = vec![];
        for i in exprs.into_iter() {
            result.push(self.alloc_expr(i));
        }
        result
    }
}

impl<F> Index<ExprId> for Arena<F> {
    type Output = Rc<ExprNode<F>>;

    fn index(&self, index: ExprId) -> &Self::Output {
        &self.exprs[index.0]
    }
}

impl<F> Index<StmtId> for Arena<F> {
    type Output = Rc<StmtNode>;

    fn index(&self, index: StmtId) -> &Self::Output {
        &self.stmts[index.0]
    }
}
