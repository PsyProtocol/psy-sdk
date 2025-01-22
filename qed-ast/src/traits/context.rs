use enum_as_inner::EnumAsInner;
use qed_common::Graph;

use crate::{
    DefId, DefinitionNode, ExprId, ExprNode, Ident, IdentId, ModuleId, ModuleNode, Program, StmtId,
    StmtNode,
};

#[derive(Debug, Clone, PartialEq, EnumAsInner)]
pub enum NodeId {
    Expr(ExprId),
    Stmt(StmtId),
    Def(DefId),
    Module(ModuleId),
}

impl From<ExprId> for NodeId {
    fn from(value: ExprId) -> Self {
        Self::Expr(value)
    }
}

impl From<StmtId> for NodeId {
    fn from(value: StmtId) -> Self {
        Self::Stmt(value)
    }
}

impl From<DefId> for NodeId {
    fn from(value: DefId) -> Self {
        Self::Def(value)
    }
}

impl From<ModuleId> for NodeId {
    fn from(value: ModuleId) -> Self {
        Self::Module(value)
    }
}

#[derive(Debug, Clone, PartialEq, EnumAsInner)]
pub enum NodeType {
    PathExpr,
    ValueExpr,
    BinaryExpr,
    UnaryExpr,
    CallExpr,
    CastExpr,
    MemberAccessExpr,
    IndexAccessExpr,

    IfStmt,
    WhileStmt,
    BlockStmt,
    AssignmentStmt,
    VariableStmt,
    DefinitionStmt,
    ExpressionStmt,
    ReturnStmt,
    StorageStmt,

    FunctionDef,
    StructDef,
    EnumDef,
    ImplDef,
    TraitDef,

    Module,

    FeltValue,
    BoolValue,
    ArrayValue,
    StructValue,
    TypeValue,
}

pub trait VisitorContext<F: Clone + From<u32>, C> {
    fn node_id(&self) -> NodeId;
    fn parent_node_id(&self) -> NodeId;
    fn node_path(&self) -> &[NodeId];
    fn push_node_id(&mut self, node_id: NodeId);
    fn pop_node_id(&mut self);
    fn node_type(&self) -> NodeType;
    fn parent_node_type(&self) -> NodeType;
    fn ident(&self, id: IdentId) -> &Ident;
    fn intern<S: Into<Ident>>(&mut self, s: S) -> IdentId;
    fn module(&self, module_id: ModuleId) -> &ModuleNode;
    fn program(&self) -> &Program<F>;
    fn dependency_graph(&self) -> Graph<ModuleId>;
    fn alloc_expression(&mut self, expr: ExprNode<F>) -> ExprId;
    fn alloc_statement(&mut self, stmt: StmtNode<F>) -> StmtId;
    fn alloc_definition(&mut self, definition: DefinitionNode) -> DefId;
    fn expression(&self, expr_id: ExprId) -> &ExprNode<F>;
    fn statement(&self, stmt_id: StmtId) -> &StmtNode<F>;
    fn definition(&self, def_id: DefId) -> &DefinitionNode;
    fn append_definition(&mut self, definition: DefinitionNode);
}
