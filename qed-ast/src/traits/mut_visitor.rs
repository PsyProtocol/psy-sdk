// use std::collections::HashMap;
//
// use crate::*;
//
// pub trait MutAstVisitor<F: Clone, C> {
//     type Context: Context<F, C>;
//     type Error: std::fmt::Debug;
//
//     fn visit_expr(
//         &mut self,
//         expr_id: ExprId,
//         ctx: &mut Self::Context,
//     ) -> Result<Self::ExprResult, Self::Error> {
//         match ctx.expression(expr_id).node_type() {
//             NodeType::PathExpr => Ok(self.visit_path(expr_id, ctx)?),
//             NodeType::ValueExpr => Ok(self.visit_value(expr_id, ctx)?),
//             NodeType::BinaryExpr => Ok(self.visit_binary(expr_id, ctx)?),
//             NodeType::UnaryExpr => Ok(self.visit_unary(expr_id, ctx)?),
//             NodeType::CallExpr => Ok(self.visit_call(expr_id, ctx)?),
//             NodeType::CastExpr => Ok(self.visit_cast(expr_id, ctx)?),
//             NodeType::IndexAccessExpr => Ok(self.visit_index_access(expr_id, ctx)?),
//             NodeType::MemberAccessExpr => Ok(self.visit_member_access(expr_id, ctx)?),
//             _ => unreachable!(),
//         }
//     }
//
//     fn visit_definition(
//         &mut self,
//         definition: &mut DefinitionNode,
//         ctx: &mut Self::Context,
//     ) -> Result<(), Self::Error> {
//         match definition {
//             DefinitionNode::Function(function) => self.visit_function(function, ctx)?,
//             DefinitionNode::Struct(r#struct) => self.visit_struct(r#struct, ctx)?,
//             DefinitionNode::Enum(r#enum) => self.visit_enum(r#enum, ctx)?,
//             DefinitionNode::Impl(r#impl) => self.visit_impl(r#impl, ctx)?,
//             DefinitionNode::Trait(r#trait) => self.visit_trait(r#trait, ctx)?,
//         }
//         Ok(())
//     }
//
//     fn visit_stmt(
//         &mut self,
//         stmt: &mut StmtNode,
//         ctx: &mut Self::Context,
//     ) -> Result<(), Self::Error> {
//         match stmt {
//             StmtNode::If(if_node) => self.visit_if(if_node, ctx)?,
//             StmtNode::While(while_node) => self.visit_while(while_node, ctx)?,
//             StmtNode::Block(block) => self.visit_block(block, ctx)?,
//             StmtNode::Assignment(assignment) => self.visit_assignment(assignment, ctx)?,
//             StmtNode::Variable(variable) => self.visit_variable(variable, ctx)?,
//             StmtNode::Return(expr) => self.visit_return(expr, ctx)?,
//             StmtNode::Definition(definition) => self.visit_definition(definition, ctx)?,
//             StmtNode::Expression(expr) => self.visit_expr(expr, ctx)?,
//         }
//         Ok(())
//     }
//
//     fn visit_use(&mut self, u: &mut UsePath, ctx: &mut Self::Context) -> Result<(), Self::Error>;
//
//     fn visit_module(
//         &mut self,
//         module: &mut ModuleNode,
//         ctx: &mut Self::Context,
//     ) -> Result<(), Self::Error> {
//         for u in &mut module.uses {
//             self.visit_use(u, ctx)?;
//         }
//
//         for definition in &mut module.definitions {
//             self.visit_definition(definition, ctx)?;
//         }
//
//         Ok(())
//     }
//
//     fn visit_program(
//         &mut self,
//         program: &mut Program,
//         ctx: &mut Self::Context,
//     ) -> Result<(), Self::Error> {
//         let mut visited = HashMap::new();
//         program
//             .dependency_graph
//             .ts(&ModuleId::root(), &mut visited, &mut |&module_id| {
//                 self.visit_module(program.modules[module_id].data_mut(), ctx)
//                     .unwrap();
//             });
//
//         Ok(())
//     }
//
//     fn visit_path(
//         &mut self,
//         node: &mut PathNode,
//         ctx: &mut Self::Context,
//     ) -> Result<(), Self::Error>;
//     fn visit_index_access(
//         &mut self,
//         node: &mut IndexAccessNode,
//         ctx: &mut Self::Context,
//     ) -> Result<(), Self::Error>;
//     fn visit_member_access(
//         &mut self,
//         node: &mut MemberAccessNode,
//         ctx: &mut Self::Context,
//     ) -> Result<(), Self::Error>;
//     fn visit_value(
//         &mut self,
//         node: &mut ValueNode<F>,
//         ctx: &mut Self::Context,
//     ) -> Result<(), Self::Error>;
//     fn visit_binary(
//         &mut self,
//         node: &mut BinaryNode,
//         ctx: &mut Self::Context,
//     ) -> Result<(), Self::Error>;
//     fn visit_unary(
//         &mut self,
//         node: &mut UnaryNode,
//         ctx: &mut Self::Context,
//     ) -> Result<(), Self::Error>;
//     fn visit_call(
//         &mut self,
//         node: &mut CallNode,
//         ctx: &mut Self::Context,
//     ) -> Result<(), Self::Error>;
//     fn visit_cast(
//         &mut self,
//         node: &mut CastNode,
//         ctx: &mut Self::Context,
//     ) -> Result<(), Self::Error>;
//
//     fn visit_if(&mut self, node: &mut IfNode, ctx: &mut Self::Context) -> Result<(), Self::Error>;
//     fn visit_while(
//         &mut self,
//         node: &mut WhileNode,
//         ctx: &mut Self::Context,
//     ) -> Result<(), Self::Error>;
//     fn visit_block(
//         &mut self,
//         node: &mut BlockNode,
//         ctx: &mut Self::Context,
//     ) -> Result<(), Self::Error>;
//     fn visit_assignment(
//         &mut self,
//         node: &mut AssignmentNode,
//         ctx: &mut Self::Context,
//     ) -> Result<(), Self::Error>;
//     fn visit_variable(
//         &mut self,
//         node: &mut VariableNode,
//         ctx: &mut Self::Context,
//     ) -> Result<(), Self::Error>;
//     fn visit_return(
//         &mut self,
//         expr: &mut ReturnNode,
//         ctx: &mut Self::Context,
//     ) -> Result<(), Self::Error>;
//
//     fn visit_impl(
//         &mut self,
//         node: &mut ImplNode,
//         ctx: &mut Self::Context,
//     ) -> Result<(), Self::Error>;
//     fn visit_trait(
//         &mut self,
//         node: &mut TraitNode,
//         ctx: &mut Self::Context,
//     ) -> Result<(), Self::Error>;
//     fn visit_function(
//         &mut self,
//         node: &mut FunctionNode,
//         ctx: &mut Self::Context,
//     ) -> Result<(), Self::Error>;
//     fn visit_struct(
//         &mut self,
//         node: &mut StructNode,
//         ctx: &mut Self::Context,
//     ) -> Result<(), Self::Error>;
//     fn visit_enum(
//         &mut self,
//         node: &mut EnumNode,
//         ctx: &mut Self::Context,
//     ) -> Result<(), Self::Error>;
// }
