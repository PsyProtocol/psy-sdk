use std::collections::HashMap;

use qed_ast::{
    DefId, DefinitionNode, ExprId, ExprNode, Ident, IdentId, InsertPosition, ModuleId, ModuleNode,
    NodeId, NodeInfo, NodeType, Program, StmtId, StmtNode, VisitorContext,
};
use qed_common::Graph;
use qedlang_core::dpn::ops::context_trait::ContextFelt;
use regex::Regex;

use crate::{InferCtxt, SymbolTable, Type, TypeId};

pub struct TypeCheckerVisitorContext<F: Clone + From<u32> + ContextFelt, C> {
    path_stack: Vec<NodeId>,
    pub program: Program<F>,
    pub symbols: SymbolTable<F>,
    pub infcx: InferCtxt,
    _marker: std::marker::PhantomData<(F, C)>,
}

impl<F: Clone + From<u32> + ContextFelt, C> TypeCheckerVisitorContext<F, C> {
    pub fn new(program: Program<F>) -> Self {
        TypeCheckerVisitorContext {
            path_stack: vec![],
            program,
            symbols: SymbolTable::new(),
            infcx: InferCtxt::new(),
            _marker: std::marker::PhantomData,
        }
    }

    pub fn get_type_detail(&self, type_id: TypeId) -> String {
        match &self.symbols[type_id] {
            Type::Unknown => format!("Unknown"),
            Type::VOID => format!("void"),
            Type::Felt => format!("Felt"),
            Type::Bool => format!("Bool"),
            Type::U32 => format!("U32"),
            Type::Array(checked_array_node) => {
                format!(
                    "[{}; {}]",
                    self.get_type_detail(checked_array_node.inner_ty),
                    self.get_type_detail(checked_array_node.size_ty),
                )
            }
            Type::Struct(checked_struct_node) => {
                format!("Struct {}", self.ident(checked_struct_node.name))
            }
            Type::Enum(checked_enum_node) => format!("Enum {}", self.ident(checked_enum_node.name)),
            Type::Function(checked_function_node) => {
                format!("fn {}", self.ident(checked_function_node.name))
            }
            Type::Trait(checked_trait_node) => {
                format!("Trait {}", self.ident(checked_trait_node.name))
            }
            Type::Const(checked_const_node) => {
                format!(
                    "Const {}",
                    self.ident(checked_const_node.name.unwrap_or(IdentId::TYPE_VOID))
                )
            }
            Type::LambdaFunction(checked_lambda_function_node) => {
                format!("lamba fn {}", self.ident(checked_lambda_function_node.name))
            }
            Type::FunctionSignature(_checked_function_signature) => format!("fn sig"),
            Type::TypeVariable(type_variable_node) => {
                let mut type_variable_details = vec![];
                for type_id in type_variable_node.constraints.iter() {
                    type_variable_details.push(self.get_type_detail(type_id.clone()));
                }
                format!(": {}", type_variable_details.join(" + "))
            }
            Type::Tuple(type_ids) => {
                let mut tuple_details = vec![];
                for type_id in type_ids {
                    tuple_details.push(self.get_type_detail(*type_id));
                }
                format!("({})", tuple_details.join(", "))
            }
        }
    }

    //warn: debug only
    pub fn print_symbol_table_to_string(&self) {
        let debug_output = format!("{}", self.symbols);

        let ident_regex = Regex::new(r"IdentId\((\d+)\)").unwrap();

        // parse all `IdentId(NUM)` to `NUM`
        let mut id_to_name = HashMap::new();
        for capture in ident_regex.captures_iter(&debug_output) {
            let ident_id_str = &capture[1]; // get the number
            if let Ok(ident_id) = ident_id_str.parse::<usize>() {
                let ident_name = self.program[IdentId(ident_id)].clone();
                id_to_name.insert(ident_id_str.to_string(), ident_name);
            }
        }

        // replace all `IdentId(NUM)` to `IdentId(NUM: "name")`
        let formatted_output = ident_regex.replace_all(&debug_output, |caps: &regex::Captures| {
            let ident_id_str = &caps[1];
            if let Some(name) = id_to_name.get(ident_id_str) {
                format!("IdentId({}: \"{}\")", ident_id_str, name)
            } else {
                caps[0].to_string()
            }
        });
        println!("Symbol Table \n{}", formatted_output);
    }

    pub fn is_trait_imported(&self, trait_type_id: TypeId) -> bool {
        self.symbols
            .get_type_id(None, self.symbols[trait_type_id].key())
            .is_some()
    }
}

impl<F: Clone + From<u32> + ContextFelt, C> VisitorContext<F, C>
    for TypeCheckerVisitorContext<F, C>
{
    type Expr = ExprNode<F>;

    type Stmt = StmtNode;

    type Definition = DefinitionNode;

    fn node_id(&self) -> NodeId {
        self.path_stack.last().unwrap().clone()
    }

    fn ancestor_node_id(&self, offset_from_top: usize) -> NodeId {
        self.path_stack[self.path_stack.len() - 1 - offset_from_top].clone()
    }

    fn node_path(&self) -> &[NodeId] {
        &self.path_stack
    }

    fn push_node_id(&mut self, node_id: NodeId) {
        self.path_stack.push(node_id);
    }

    fn pop_node_id(&mut self) {
        self.path_stack.pop();
    }

    fn node_type(&self) -> NodeType {
        match self.node_id() {
            NodeId::Expr(expr_id) => self.expression(expr_id).node_type(),
            NodeId::Stmt(stmt_id) => self.statement(stmt_id).node_type(),
            NodeId::Def(def_id) => self.definition(def_id).node_type(),
            NodeId::Module(_) => NodeType::Module,
        }
    }

    fn ancestor_node_type(&self, offset_from_top: usize) -> NodeType {
        match self.ancestor_node_id(offset_from_top) {
            NodeId::Expr(expr_id) => self.expression(expr_id).node_type(),
            NodeId::Stmt(stmt_id) => self.statement(stmt_id).node_type(),
            NodeId::Def(def_id) => self.definition(def_id).node_type(),
            NodeId::Module(_) => NodeType::Module,
        }
    }

    fn ident(&self, id: IdentId) -> &Ident {
        &self.program.interner[id]
    }

    fn intern<S: Into<Ident>>(&mut self, s: S) -> IdentId {
        self.program.interner.intern_ident(s)
    }

    fn module(&self, module_id: ModuleId) -> &ModuleNode {
        self.program.modules[module_id].data()
    }

    fn program(&self) -> &Program<F> {
        &self.program
    }

    fn dependency_graph(&self) -> Graph<ModuleId> {
        self.program.dependency_graph.clone()
    }

    fn expression(&self, expr_id: ExprId) -> &Self::Expr {
        &self.program.exprs[expr_id]
    }

    fn statement(&self, stmt_id: StmtId) -> &Self::Stmt {
        &self.program.stmts[stmt_id]
    }

    fn definition(&self, def_id: DefId) -> &Self::Definition {
        &self.program.defs[def_id]
    }

    fn insert_definition(&mut self, _definition: Self::Definition, _pos: InsertPosition) {
        unimplemented!()
    }

    fn alloc_expression(&mut self, _expr: Self::Expr) -> ExprId {
        unimplemented!()
    }

    fn alloc_statement(&mut self, _stmt: Self::Stmt) -> StmtId {
        unimplemented!()
    }

    fn alloc_definition(&mut self, _definition: Self::Definition) -> DefId {
        unimplemented!()
    }

    fn replace_definition(&mut self, _def_id: DefId, _definition: Self::Definition) {
        unimplemented!()
    }

    fn replace_statement(&mut self, _stmt_id: StmtId, _statement: Self::Stmt) {
        unimplemented!()
    }

    fn intern_lambda(&mut self) -> IdentId {
        self.program.interner.intern_lambda()
    }
}
