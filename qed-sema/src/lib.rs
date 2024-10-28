pub mod error;

use error::*;
use qed_ast::*;

use crate::arena::IdentId;

pub struct SemanticChecker;

impl Resolver for SemanticChecker {
    fn enter_scope(&mut self) {}

    fn exit_scope(&mut self) {}

    fn enter_function(&mut self) {}

    fn exit_function(&mut self) {}

    fn define_variable(&mut self, name: &IdentId, value: ValueNode) {}

    fn set_variable(&mut self, name: IdentId, value: ValueNode) {}

    fn resolve_variable(&mut self, name: &IdentId) -> ValueNode {
        ValueNode::Felt(0)
    }
}

impl AstVisitor for SemanticChecker {
    type ExprResult = Result<()>;

    type StmtResult = Result<()>;

    fn visit_variable(&mut self, node: &VariableNode) -> Self::ExprResult {
        Ok(())
    }

    fn visit_value(&mut self, node: &ValueNode) -> Self::ExprResult {
        Ok(())
    }

    fn visit_binary(&mut self, node: &BinaryNode) -> Self::ExprResult {
        Ok(())
    }

    fn visit_unary(&mut self, node: &UnaryNode) -> Self::ExprResult {
        Ok(())
    }

    fn visit_call(&mut self, node: &CallNode) -> Self::ExprResult {
        Ok(())
    }

    fn visit_if(&mut self, node: &IfNode) -> Self::StmtResult {
        Ok(())
    }

    fn visit_loop(&mut self, node: &WhileNode) -> Self::StmtResult {
        // for stmt in node.stmts.iter() {
        //     if stmt.is_break() || stmt.is_return() {
        //         return Ok(());
        //     }
        // }
        // Err(Error::InfiniteLoop)
        Ok(())
    }

    fn visit_break(&mut self, label: &IdentId) -> Self::StmtResult {
        Ok(())
    }

    fn visit_continue(&mut self, label: &IdentId) -> Self::StmtResult {
        Ok(())
    }

    fn visit_block(&mut self, node: &BlockNode) -> Self::StmtResult {
        Ok(())
    }

    fn visit_assignment(&mut self, node: &AssignmentNode) -> Self::StmtResult {
        Ok(())
    }

    fn visit_var_decl(&mut self, node: &VarDeclNode) -> Self::StmtResult {
        Ok(())
    }

    fn visit_return(&mut self, expr: &ReturnNode) -> Self::StmtResult {
        Ok(())
    }

    fn visit_impl(&mut self, node: &ImplNode) -> Self::StmtResult {
        Ok(())
    }

    fn visit_function(&mut self, node: &FunctionNode) -> Self::StmtResult {
        Ok(())
    }

    fn visit_struct(&mut self, node: &StructNode) -> Self::StmtResult {
        Ok(())
    }

    fn visit_enum(&mut self, node: &EnumNode) -> Self::StmtResult {
        Ok(())
    }

    // fn visit_constant(&mut self, node: &ConstantNode) {}
}
