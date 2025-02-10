use std::collections::HashMap;

use crate::*;

pub trait AstVisitor<F: Clone + From<u32>, C> {
    type Expr: NodeInfo;
    type Stmt: NodeInfo;
    type Definition: NodeInfo;
    type ExprResult;
    type StmtResult: From<Self::ExprResult> + From<Self::DefinitionResult>;
    type DefinitionResult;
    type Context: VisitorContext<
        F,
        C,
        Expr = Self::Expr,
        Stmt = Self::Stmt,
        Definition = Self::Definition,
    >;
    type Error;

    fn visit_expr(
        &mut self,
        expr_id: ExprId,
        ctx: &mut Self::Context,
    ) -> Result<Self::ExprResult, Self::Error> {
        ctx.push_node_id(NodeId::from(expr_id));
        let res = match ctx.expression(expr_id).node_type() {
            NodeType::PathExpr => self.visit_path(expr_id, ctx)?,
            NodeType::ValueExpr => self.visit_value(expr_id, ctx)?,
            NodeType::BinaryExpr => self.visit_binary(expr_id, ctx)?,
            NodeType::UnaryExpr => self.visit_unary(expr_id, ctx)?,
            NodeType::CallExpr => self.visit_call(expr_id, ctx)?,
            NodeType::CastExpr => self.visit_cast(expr_id, ctx)?,
            NodeType::IndexAccessExpr => self.visit_index_access(expr_id, ctx)?,
            NodeType::MemberAccessExpr => self.visit_member_access(expr_id, ctx)?,
            NodeType::StorageExpr => self.visit_storage_read(expr_id, ctx)?,
            NodeType::ContextExpr => self.visit_context(expr_id, ctx)?,
            NodeType::AssertExpr => self.visit_assert(expr_id, ctx)?,
            NodeType::AssertEqExpr => self.visit_assert_eq(expr_id, ctx)?,
            _ => unreachable!(),
        };
        ctx.pop_node_id();
        Ok(res)
    }

    fn visit_definition(
        &mut self,
        def_id: DefId,
        ctx: &mut Self::Context,
    ) -> Result<Self::DefinitionResult, Self::Error> {
        ctx.push_node_id(NodeId::from(def_id));
        let res = match ctx.definition(def_id).node_type() {
            NodeType::FunctionDef => self.visit_function(def_id, ctx)?,
            NodeType::StructDef => self.visit_struct(def_id, ctx)?,
            NodeType::EnumDef => self.visit_enum(def_id, ctx)?,
            NodeType::ImplDef => self.visit_impl(def_id, ctx)?,
            NodeType::TraitDef => self.visit_trait(def_id, ctx)?,
            _ => unreachable!(),
        };
        ctx.pop_node_id();
        Ok(res)
    }

    fn visit_stmt(
        &mut self,
        stmt_id: StmtId,
        ctx: &mut Self::Context,
    ) -> Result<Self::StmtResult, Self::Error> {
        ctx.push_node_id(NodeId::from(stmt_id));
        let res = match ctx.statement(stmt_id).node_type() {
            NodeType::IfStmt => self.visit_if(stmt_id, ctx)?,
            NodeType::WhileStmt => self.visit_while(stmt_id, ctx)?,
            NodeType::BlockStmt => self.visit_block(stmt_id, ctx)?,
            NodeType::AssignmentStmt => self.visit_assignment(stmt_id, ctx)?,
            NodeType::VariableStmt => self.visit_variable(stmt_id, ctx)?,
            NodeType::ReturnStmt => self.visit_return(stmt_id, ctx)?,
            NodeType::DefinitionStmt => {
                let def_id = ctx.statement(stmt_id).as_definition().unwrap().clone();
                Self::StmtResult::from(self.visit_definition(def_id, ctx)?)
            }
            NodeType::ExpressionStmt => {
                let expr_id = ctx.statement(stmt_id).as_expression().unwrap().clone();
                Self::StmtResult::from(self.visit_expr(expr_id, ctx)?)
            }
            NodeType::StorageStmt => self.visit_storage_write(stmt_id, ctx)?,
            _ => unreachable!(),
        };
        ctx.pop_node_id();
        Ok(res)
    }

    fn visit_use(&mut self, use_path: &UsePath, ctx: &mut Self::Context)
        -> Result<(), Self::Error>;

    fn visit_module(
        &mut self,
        module_id: ModuleId,
        ctx: &mut Self::Context,
    ) -> Result<(), Self::Error> {
        ctx.push_node_id(NodeId::from(module_id));
        // TODO: remove clone
        for u in &ctx.module(module_id).uses.clone() {
            self.visit_use(u, ctx)?;
        }

        // TODO: remove clone
        for &definition in &ctx.module(module_id).definitions.clone() {
            self.visit_definition(definition, ctx)?;
        }
        ctx.pop_node_id();

        Ok(())
    }

    fn visit_program(&mut self, ctx: &mut Self::Context) -> Result<(), Self::Error> {
        let mut visited = HashMap::new();
        ctx.dependency_graph()
            .ts(&ModuleId::root(), &mut visited, &mut |&module_id| {
                self.visit_module(module_id, ctx);
            });

        Ok(())
    }

    fn visit_path(
        &mut self,
        node: ExprId,
        ctx: &mut Self::Context,
    ) -> Result<Self::ExprResult, Self::Error>;
    fn visit_index_access(
        &mut self,
        node: ExprId,
        ctx: &mut Self::Context,
    ) -> Result<Self::ExprResult, Self::Error>;
    fn visit_member_access(
        &mut self,
        node: ExprId,
        ctx: &mut Self::Context,
    ) -> Result<Self::ExprResult, Self::Error>;
    fn visit_storage_read(
        &mut self,
        node: ExprId,
        ctx: &mut Self::Context,
    ) -> Result<Self::ExprResult, Self::Error>;
    fn visit_context(
        &mut self,
        node: ExprId,
        ctx: &mut Self::Context,
    ) -> Result<Self::ExprResult, Self::Error>;
    fn visit_assert(
        &mut self,
        node: ExprId,
        ctx: &mut Self::Context,
    ) -> Result<Self::ExprResult, Self::Error>;
    fn visit_assert_eq(
        &mut self,
        node: ExprId,
        ctx: &mut Self::Context,
    ) -> Result<Self::ExprResult, Self::Error>;
    fn visit_value(
        &mut self,
        node: ExprId,
        ctx: &mut Self::Context,
    ) -> Result<Self::ExprResult, Self::Error>;
    fn visit_binary(
        &mut self,
        node: ExprId,
        ctx: &mut Self::Context,
    ) -> Result<Self::ExprResult, Self::Error>;
    fn visit_unary(
        &mut self,
        node: ExprId,
        ctx: &mut Self::Context,
    ) -> Result<Self::ExprResult, Self::Error>;
    fn visit_call(
        &mut self,
        node: ExprId,
        ctx: &mut Self::Context,
    ) -> Result<Self::ExprResult, Self::Error>;
    fn visit_cast(
        &mut self,
        node: ExprId,
        ctx: &mut Self::Context,
    ) -> Result<Self::ExprResult, Self::Error>;

    fn visit_if(
        &mut self,
        node: StmtId,
        ctx: &mut Self::Context,
    ) -> Result<Self::StmtResult, Self::Error>;
    fn visit_while(
        &mut self,
        node: StmtId,
        ctx: &mut Self::Context,
    ) -> Result<Self::StmtResult, Self::Error>;
    fn visit_block(
        &mut self,
        node: StmtId,
        ctx: &mut Self::Context,
    ) -> Result<Self::StmtResult, Self::Error>;
    fn visit_assignment(
        &mut self,
        node: StmtId,
        ctx: &mut Self::Context,
    ) -> Result<Self::StmtResult, Self::Error>;
    fn visit_variable(
        &mut self,
        node: StmtId,
        ctx: &mut Self::Context,
    ) -> Result<Self::StmtResult, Self::Error>;
    fn visit_storage_write(
        &mut self,
        node: StmtId,
        ctx: &mut Self::Context,
    ) -> Result<Self::StmtResult, Self::Error>;
    fn visit_return(
        &mut self,
        expr: StmtId,
        ctx: &mut Self::Context,
    ) -> Result<Self::StmtResult, Self::Error>;

    fn visit_impl(
        &mut self,
        node: DefId,
        ctx: &mut Self::Context,
    ) -> Result<Self::DefinitionResult, Self::Error>;
    fn visit_trait(
        &mut self,
        node: DefId,
        ctx: &mut Self::Context,
    ) -> Result<Self::DefinitionResult, Self::Error>;
    fn visit_function(
        &mut self,
        node: DefId,
        ctx: &mut Self::Context,
    ) -> Result<Self::DefinitionResult, Self::Error>;
    fn visit_struct(
        &mut self,
        node: DefId,
        ctx: &mut Self::Context,
    ) -> Result<Self::DefinitionResult, Self::Error>;
    fn visit_enum(
        &mut self,
        node: DefId,
        ctx: &mut Self::Context,
    ) -> Result<Self::DefinitionResult, Self::Error>;
}
