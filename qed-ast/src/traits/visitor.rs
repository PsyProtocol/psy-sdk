use std::collections::HashMap;

use crate::*;

pub trait AstVisitor<F: Clone, C> {
    type ExprResult;
    type StmtResult: From<Self::ExprResult>;
    type Context;
    type Error;

    fn visit_expr(
        &mut self,
        expr: &ExprNode<F>,
        ctx: &mut Self::Context,
    ) -> Result<Self::ExprResult, Self::Error> {
        match expr {
            ExprNode::Path(ident) => Ok(self.visit_path(ident, ctx)?),
            ExprNode::Value(literal) => Ok(self.visit_value(literal, ctx)?),
            ExprNode::Binary(binary) => Ok(self.visit_binary(binary, ctx)?),
            ExprNode::Unary(unary) => Ok(self.visit_unary(unary, ctx)?),
            ExprNode::Call(call) => Ok(self.visit_call(call, ctx)?),
            ExprNode::Cast(cast) => Ok(self.visit_cast(cast, ctx)?),
            ExprNode::IndexAccess(index_access_node) => {
                Ok(self.visit_index_access(index_access_node, ctx)?)
            }
            ExprNode::MemberAccess(member_access_node) => {
                Ok(self.visit_member_access(member_access_node, ctx)?)
            }
        }
    }

    fn visit_definition(
        &mut self,
        definition: &DefinitionNode,
        ctx: &mut Self::Context,
    ) -> Result<Self::StmtResult, Self::Error> {
        match definition {
            DefinitionNode::Function(function) => Ok(self.visit_function(function, ctx)?),
            DefinitionNode::Struct(r#struct) => Ok(self.visit_struct(r#struct, ctx)?),
            DefinitionNode::Enum(r#enum) => Ok(self.visit_enum(r#enum, ctx)?),
            DefinitionNode::Impl(r#impl) => Ok(self.visit_impl(r#impl, ctx)?),
            DefinitionNode::Trait(r#trait) => Ok(self.visit_trait(r#trait, ctx)?),
        }
    }

    fn visit_stmt(
        &mut self,
        stmt: &StmtNode<F>,
        ctx: &mut Self::Context,
    ) -> Result<Self::StmtResult, Self::Error> {
        match stmt {
            StmtNode::If(if_node) => Ok(self.visit_if(if_node, ctx)?),
            StmtNode::While(while_node) => Ok(self.visit_while(while_node, ctx)?),
            StmtNode::Block(block) => Ok(self.visit_block(block, ctx)?),
            StmtNode::Assignment(assignment) => Ok(self.visit_assignment(assignment, ctx)?),
            StmtNode::Variable(variable) => Ok(self.visit_variable(variable, ctx)?),
            StmtNode::Return(expr) => Ok(self.visit_return(expr, ctx)?),
            StmtNode::Definition(definition) => Ok(self.visit_definition(definition, ctx)?),
            StmtNode::Expression(expr) => Ok(Self::StmtResult::from(self.visit_expr(expr, ctx)?)),
        }
    }

    fn visit_use(&mut self, use_path: &UsePath, ctx: &mut Self::Context)
        -> Result<(), Self::Error>;

    fn visit_module(
        &mut self,
        module: &RawModule,
        ctx: &mut Self::Context,
    ) -> Result<(), Self::Error> {
        for u in &module.uses {
            self.visit_use(u, ctx);
        }

        for definition in &module.definitions {
            self.visit_definition(definition, ctx);
        }

        Ok(())
    }

    fn visit_program(
        &mut self,
        program: &Program,
        ctx: &mut Self::Context,
    ) -> Result<(), Self::Error> {
        let mut visited = HashMap::new();
        program
            .dependency_graph
            .ts(&ModuleId::root(), &mut visited, &mut |&module_id| {
                self.visit_module(program.modules[module_id].data(), ctx);
            });

        Ok(())
    }

    fn visit_path(
        &mut self,
        node: &PathNode,
        ctx: &mut Self::Context,
    ) -> Result<Self::ExprResult, Self::Error>;
    fn visit_index_access(
        &mut self,
        node: &IndexAccessNode,
        ctx: &mut Self::Context,
    ) -> Result<Self::ExprResult, Self::Error>;
    fn visit_member_access(
        &mut self,
        node: &MemberAccessNode,
        ctx: &mut Self::Context,
    ) -> Result<Self::ExprResult, Self::Error>;
    fn visit_value(
        &mut self,
        node: &ValueNode<F>,
        ctx: &mut Self::Context,
    ) -> Result<Self::ExprResult, Self::Error>;
    fn visit_binary(
        &mut self,
        node: &BinaryNode,
        ctx: &mut Self::Context,
    ) -> Result<Self::ExprResult, Self::Error>;
    fn visit_unary(
        &mut self,
        node: &UnaryNode,
        ctx: &mut Self::Context,
    ) -> Result<Self::ExprResult, Self::Error>;
    fn visit_call(
        &mut self,
        node: &CallNode,
        ctx: &mut Self::Context,
    ) -> Result<Self::ExprResult, Self::Error>;
    fn visit_cast(
        &mut self,
        node: &CastNode,
        ctx: &mut Self::Context,
    ) -> Result<Self::ExprResult, Self::Error>;

    fn visit_if(
        &mut self,
        node: &IfNode,
        ctx: &mut Self::Context,
    ) -> Result<Self::StmtResult, Self::Error>;
    fn visit_while(
        &mut self,
        node: &WhileNode,
        ctx: &mut Self::Context,
    ) -> Result<Self::StmtResult, Self::Error>;
    fn visit_block(
        &mut self,
        node: &BlockNode,
        ctx: &mut Self::Context,
    ) -> Result<Self::StmtResult, Self::Error>;
    fn visit_assignment(
        &mut self,
        node: &AssignmentNode,
        ctx: &mut Self::Context,
    ) -> Result<Self::StmtResult, Self::Error>;
    fn visit_variable(
        &mut self,
        node: &VariableNode,
        ctx: &mut Self::Context,
    ) -> Result<Self::StmtResult, Self::Error>;
    fn visit_return(
        &mut self,
        expr: &ReturnNode,
        ctx: &mut Self::Context,
    ) -> Result<Self::StmtResult, Self::Error>;

    fn visit_impl(
        &mut self,
        node: &ImplNode,
        ctx: &mut Self::Context,
    ) -> Result<Self::StmtResult, Self::Error>;
    fn visit_trait(
        &mut self,
        node: &TraitNode,
        ctx: &mut Self::Context,
    ) -> Result<Self::StmtResult, Self::Error>;
    fn visit_function(
        &mut self,
        node: &FunctionNode,
        ctx: &mut Self::Context,
    ) -> Result<Self::StmtResult, Self::Error>;
    fn visit_struct(
        &mut self,
        node: &StructNode,
        ctx: &mut Self::Context,
    ) -> Result<Self::StmtResult, Self::Error>;
    fn visit_enum(
        &mut self,
        node: &EnumNode,
        ctx: &mut Self::Context,
    ) -> Result<Self::StmtResult, Self::Error>;
}
