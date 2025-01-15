use std::collections::HashMap;

use crate::*;

pub trait MutAstVisitor<F: Clone, C> {
    type Context;
    type Error: std::fmt::Debug;

    fn enter_expr(&mut self, node: &mut ExprNode<F>) -> Result<(), Self::Error>;
    fn exit_expr(&mut self, node: &mut ExprNode<F>) -> Result<(), Self::Error>;

    fn enter_statement(&mut self, node: &mut StmtNode<F>) -> Result<(), Self::Error>;
    fn exit_statement(&mut self, node: &mut StmtNode<F>) -> Result<(), Self::Error>;

    fn enter_definition(&mut self, node: &mut DefinitionNode) -> Result<(), Self::Error>;
    fn exit_definition(&mut self, node: &mut DefinitionNode) -> Result<(), Self::Error>;

    fn visit_expr(
        &mut self,
        expr: &mut ExprNode<F>,
        ctx: &mut Self::Context,
    ) -> Result<(), Self::Error> {
        self.enter_expr(expr)?;

        match expr {
            ExprNode::Path(ident) => self.visit_path(ident, ctx)?,
            ExprNode::Value(literal) => self.visit_value(literal, ctx)?,
            ExprNode::Binary(binary) => self.visit_binary(binary, ctx)?,
            ExprNode::Unary(unary) => self.visit_unary(unary, ctx)?,
            ExprNode::Call(call) => self.visit_call(call, ctx)?,
            ExprNode::Cast(cast) => self.visit_cast(cast, ctx)?,
            ExprNode::IndexAccess(index_access_node) => {
                self.visit_index_access(index_access_node, ctx)?
            }
            ExprNode::MemberAccess(member_access_node) => {
                self.visit_member_access(member_access_node, ctx)?
            }
        }

        self.exit_expr(expr)
    }

    fn visit_definition(
        &mut self,
        definition: &mut DefinitionNode,
        ctx: &mut Self::Context,
    ) -> Result<(), Self::Error> {
        self.enter_definition(definition)?;

        match definition {
            DefinitionNode::Function(function) => self.visit_function(function, ctx)?,
            DefinitionNode::Struct(r#struct) => self.visit_struct(r#struct, ctx)?,
            DefinitionNode::Enum(r#enum) => self.visit_enum(r#enum, ctx)?,
            DefinitionNode::Impl(r#impl) => self.visit_impl(r#impl, ctx)?,
            DefinitionNode::Trait(r#trait) => self.visit_trait(r#trait, ctx)?,
        }

        self.exit_definition(definition)
    }

    fn visit_stmt(
        &mut self,
        stmt: &mut StmtNode<F>,
        ctx: &mut Self::Context,
    ) -> Result<(), Self::Error> {
        self.enter_statement(stmt)?;

        match stmt {
            StmtNode::If(if_node) => self.visit_if(if_node, ctx)?,
            StmtNode::While(while_node) => self.visit_while(while_node, ctx)?,
            StmtNode::Block(block) => self.visit_block(block, ctx)?,
            StmtNode::Assignment(assignment) => self.visit_assignment(assignment, ctx)?,
            StmtNode::Variable(variable) => self.visit_variable(variable, ctx)?,
            StmtNode::Return(expr) => self.visit_return(expr, ctx)?,
            StmtNode::Definition(definition) => self.visit_definition(definition, ctx)?,
            StmtNode::Expression(expr) => self.visit_expr(expr, ctx)?,
        }

        self.exit_statement(stmt)
    }

    fn visit_use(&mut self, u: &mut UsePath, ctx: &mut Self::Context) -> Result<(), Self::Error>;

    fn visit_module(
        &mut self,
        module: &mut RawModule,
        ctx: &mut Self::Context,
    ) -> Result<(), Self::Error> {
        for u in &mut module.uses {
            self.visit_use(u, ctx);
        }

        for definition in &mut module.definitions {
            self.visit_definition(definition, ctx)?;
        }

        Ok(())
    }

    fn visit_program(
        &mut self,
        program: &mut Program,
        ctx: &mut Self::Context,
    ) -> Result<(), Self::Error> {
        let mut visited = HashMap::new();
        program
            .dependency_graph
            .ts(&program.root_file_id, &mut visited, &mut |file_id| {
                self.visit_module(&mut program.modules.get_mut(file_id).unwrap(), ctx)
                    .unwrap();
            });

        Ok(())
    }

    fn visit_path(
        &mut self,
        node: &mut PathNode,
        ctx: &mut Self::Context,
    ) -> Result<(), Self::Error>;
    fn visit_index_access(
        &mut self,
        node: &mut IndexAccessNode,
        ctx: &mut Self::Context,
    ) -> Result<(), Self::Error>;
    fn visit_member_access(
        &mut self,
        node: &mut MemberAccessNode,
        ctx: &mut Self::Context,
    ) -> Result<(), Self::Error>;
    fn visit_value(
        &mut self,
        node: &mut ValueNode<F>,
        ctx: &mut Self::Context,
    ) -> Result<(), Self::Error>;
    fn visit_binary(
        &mut self,
        node: &mut BinaryNode,
        ctx: &mut Self::Context,
    ) -> Result<(), Self::Error>;
    fn visit_unary(
        &mut self,
        node: &mut UnaryNode,
        ctx: &mut Self::Context,
    ) -> Result<(), Self::Error>;
    fn visit_call(
        &mut self,
        node: &mut CallNode,
        ctx: &mut Self::Context,
    ) -> Result<(), Self::Error>;
    fn visit_cast(
        &mut self,
        node: &mut CastNode,
        ctx: &mut Self::Context,
    ) -> Result<(), Self::Error>;

    fn visit_if(&mut self, node: &mut IfNode, ctx: &mut Self::Context) -> Result<(), Self::Error>;
    fn visit_while(
        &mut self,
        node: &mut WhileNode,
        ctx: &mut Self::Context,
    ) -> Result<(), Self::Error>;
    fn visit_block(
        &mut self,
        node: &mut BlockNode,
        ctx: &mut Self::Context,
    ) -> Result<(), Self::Error>;
    fn visit_assignment(
        &mut self,
        node: &mut AssignmentNode,
        ctx: &mut Self::Context,
    ) -> Result<(), Self::Error>;
    fn visit_variable(
        &mut self,
        node: &mut VariableNode,
        ctx: &mut Self::Context,
    ) -> Result<(), Self::Error>;
    fn visit_return(
        &mut self,
        expr: &mut ReturnNode,
        ctx: &mut Self::Context,
    ) -> Result<(), Self::Error>;

    fn visit_impl(
        &mut self,
        node: &mut ImplNode,
        ctx: &mut Self::Context,
    ) -> Result<(), Self::Error>;
    fn visit_trait(
        &mut self,
        node: &mut TraitNode,
        ctx: &mut Self::Context,
    ) -> Result<(), Self::Error>;
    fn visit_function(
        &mut self,
        node: &mut FunctionNode,
        ctx: &mut Self::Context,
    ) -> Result<(), Self::Error>;
    fn visit_struct(
        &mut self,
        node: &mut StructNode,
        ctx: &mut Self::Context,
    ) -> Result<(), Self::Error>;
    fn visit_enum(
        &mut self,
        node: &mut EnumNode,
        ctx: &mut Self::Context,
    ) -> Result<(), Self::Error>;
}
