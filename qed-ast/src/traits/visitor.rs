use std::collections::HashMap;

use crate::*;

pub trait AstVisitor<F: Clone, C> {
    type ExprResult;
    type StmtResult: From<Self::ExprResult>;
    type Context: VisitorContext<F, C>;
    type Error;

    fn visit_expr(
        &mut self,
        expr_id: ExprId,
        ctx: &mut Self::Context,
    ) -> Result<Self::ExprResult, Self::Error> {
        match ctx.expression(expr_id).node_type() {
            NodeType::PathExpr => Ok(self.visit_path(expr_id, ctx)?),
            NodeType::ValueExpr => Ok(self.visit_value(expr_id, ctx)?),
            NodeType::BinaryExpr => Ok(self.visit_binary(expr_id, ctx)?),
            NodeType::UnaryExpr => Ok(self.visit_unary(expr_id, ctx)?),
            NodeType::CallExpr => Ok(self.visit_call(expr_id, ctx)?),
            NodeType::CastExpr => Ok(self.visit_cast(expr_id, ctx)?),
            NodeType::IndexAccessExpr => Ok(self.visit_index_access(expr_id, ctx)?),
            NodeType::MemberAccessExpr => Ok(self.visit_member_access(expr_id, ctx)?),
            _ => unreachable!(),
        }
    }

    fn visit_definition(
        &mut self,
        def_id: DefId,
        ctx: &mut Self::Context,
    ) -> Result<Self::StmtResult, Self::Error> {
        match ctx.definition(def_id).node_type() {
            NodeType::FunctionDef => Ok(self.visit_function(def_id, ctx)?),
            NodeType::StructDef => Ok(self.visit_struct(def_id, ctx)?),
            NodeType::EnumDef => Ok(self.visit_enum(def_id, ctx)?),
            NodeType::ImplDef => Ok(self.visit_impl(def_id, ctx)?),
            NodeType::TraitDef => Ok(self.visit_trait(def_id, ctx)?),
            _ => unreachable!(),
        }
    }

    fn visit_stmt(
        &mut self,
        stmt_id: StmtId,
        ctx: &mut Self::Context,
    ) -> Result<Self::StmtResult, Self::Error> {
        match ctx.statement(stmt_id).node_type() {
            NodeType::IfStmt => Ok(self.visit_if(stmt_id, ctx)?),
            NodeType::WhileStmt => Ok(self.visit_while(stmt_id, ctx)?),
            NodeType::BlockStmt => Ok(self.visit_block(stmt_id, ctx)?),
            NodeType::AssignmentStmt => Ok(self.visit_assignment(stmt_id, ctx)?),
            NodeType::VariableStmt => Ok(self.visit_variable(stmt_id, ctx)?),
            NodeType::ReturnStmt => Ok(self.visit_return(stmt_id, ctx)?),
            NodeType::DefinitionStmt => Ok(self
                .visit_definition(ctx.statement(stmt_id).as_definition().unwrap().clone(), ctx)?),
            NodeType::ExpressionStmt => Ok(Self::StmtResult::from(
                self.visit_expr(ctx.statement(stmt_id).as_expression().unwrap().clone(), ctx)?,
            )),
            _ => unreachable!(),
        }
    }

    fn visit_use(&mut self, use_path: &UsePath, ctx: &mut Self::Context)
        -> Result<(), Self::Error>;

    fn visit_module(
        &mut self,
        module_id: ModuleId,
        ctx: &mut Self::Context,
    ) -> Result<(), Self::Error> {
        // TODO: remove clone
        for u in &ctx.module(module_id).uses.clone() {
            self.visit_use(u, ctx)?;
        }

        // TODO: remove clone
        for &definition in &ctx.module(module_id).definitions.clone() {
            self.visit_definition(definition, ctx)?;
        }

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
    fn visit_return(
        &mut self,
        expr: StmtId,
        ctx: &mut Self::Context,
    ) -> Result<Self::StmtResult, Self::Error>;

    fn visit_impl(
        &mut self,
        node: DefId,
        ctx: &mut Self::Context,
    ) -> Result<Self::StmtResult, Self::Error>;
    fn visit_trait(
        &mut self,
        node: DefId,
        ctx: &mut Self::Context,
    ) -> Result<Self::StmtResult, Self::Error>;
    fn visit_function(
        &mut self,
        node: DefId,
        ctx: &mut Self::Context,
    ) -> Result<Self::StmtResult, Self::Error>;
    fn visit_struct(
        &mut self,
        node: DefId,
        ctx: &mut Self::Context,
    ) -> Result<Self::StmtResult, Self::Error>;
    fn visit_enum(
        &mut self,
        node: DefId,
        ctx: &mut Self::Context,
    ) -> Result<Self::StmtResult, Self::Error>;
}
