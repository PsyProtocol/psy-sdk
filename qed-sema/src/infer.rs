use std::collections::HashMap;

use indexmap::IndexMap;
use itertools::Itertools;
use qedlang_core::dpn::ops::context_trait::ContextFelt;

use crate::{
    CheckedArrayNode, CheckedStructField, CheckedStructNode, Result, ScopeId, Type,
    TypeCheckerVisitorContext, TypeId,
};

#[derive(Debug)]
pub struct InferCtxt<F: Clone + From<u32> + ContextFelt, C> {
    contexts: Vec<Vec<HashMap<TypeId, TypeId>>>,
    _marker: std::marker::PhantomData<(F, C)>,
}

impl<F: Clone + From<u32> + ContextFelt, C> InferCtxt<F, C> {
    pub fn new() -> Self {
        InferCtxt {
            contexts: vec![vec![HashMap::new()]],
            _marker: std::marker::PhantomData,
        }
    }

    pub fn enter_context(&mut self) {
        self.contexts.push(vec![HashMap::new()]);
    }

    pub fn exit_context(&mut self) {
        self.contexts.pop();
    }

    pub fn enter_scope(&mut self) {
        self.contexts.last_mut().unwrap().push(HashMap::new());
    }

    pub fn exit_scope(&mut self) {
        self.contexts.last_mut().unwrap().pop();
    }

    pub fn has_equations(&self) -> bool {
        self.contexts.last().unwrap().iter().any(|x| !x.is_empty())
    }

    pub fn probe(&self, type_id: TypeId) -> Option<TypeId> {
        self.contexts
            .last()
            .unwrap()
            .iter()
            .rev()
            .find_map(|x| x.get(&type_id))
            .cloned()
    }

    pub fn equate(&mut self, lhs_ty: TypeId, rhs_ty: TypeId) {
        self.contexts
            .last_mut()
            .unwrap()
            .last_mut()
            .unwrap()
            .insert(lhs_ty, rhs_ty);
    }
}

pub trait Inferer<F: Clone + From<u32> + ContextFelt, C> {
    fn unify(&mut self, lhs_ty: TypeId, rhs_ty: TypeId) -> bool;

    fn substitute_all(&mut self, type_id: TypeId) -> Result<TypeId>;

    fn substitute_type(&mut self, type_id: TypeId) -> Result<TypeId>;

    fn is_concrete(&self, type_id: TypeId) -> bool;
}

impl<F: Clone + From<u32> + ContextFelt, C> Inferer<F, C> for TypeCheckerVisitorContext<F, C> {
    fn unify(&mut self, lhs_ty: TypeId, rhs_ty: TypeId) -> bool {
        let lhs_ty = self.substitute_all(lhs_ty).unwrap();
        let rhs_ty = self.substitute_all(rhs_ty).unwrap();

        match (&self.symbols[lhs_ty], &self.symbols[rhs_ty]) {
            (Type::TypeVariable(_), _) => {
                self.infcx.equate(lhs_ty, rhs_ty);
                true
            }
            (_, Type::TypeVariable(_)) => {
                self.infcx.equate(rhs_ty, lhs_ty);
                true
            }
            (Type::Struct(s1), Type::Struct(s2)) => {
                s1.name == s2.name
                    && s1.scope_id == s2.scope_id
                    && s1.generic_parameters.len() == s2.generic_parameters.len()
                    && s1
                        .generic_parameters
                        .clone()
                        .into_iter()
                        .zip_eq(s2.generic_parameters.clone().into_iter())
                        .all(|(p1, p2)| self.unify(p1, p2))
            }
            (Type::Array(a1), Type::Array(a2)) => {
                // TODO: remove clone
                let a1 = a1.clone();
                let a2 = a2.clone();
                self.unify(a1.inner_ty, a2.inner_ty) && self.unify(a1.size_ty, a2.size_ty)
            }
            (Type::Tuple(t1), Type::Tuple(t2)) => {
                for (lhs_ty, rhs_ty) in t1.clone().into_iter().zip_eq(t2.clone().into_iter()) {
                    if !self.unify(lhs_ty, rhs_ty) {
                        return false;
                    }
                }
                true
            }

            (Type::Function(f), Type::FunctionSignature(sig))
            | (Type::FunctionSignature(sig), Type::Function(f)) => &f.signature() == sig,
            (Type::LambdaFunction(f), Type::FunctionSignature(sig))
            | (Type::FunctionSignature(sig), Type::LambdaFunction(f)) => &f.signature() == sig,
            (Type::Const(c), Type::Const(d)) => c.ty == d.ty,
            (Type::Const(c), _) => c.ty == rhs_ty,
            (_, Type::Const(c)) => c.ty == lhs_ty,
            (Type::Unknown, Type::Unknown) => false,
            (Type::Unknown, _) | (_, Type::Unknown) => true,
            _ => lhs_ty == rhs_ty,
        }
    }

    fn substitute_all(&mut self, type_id: TypeId) -> Result<TypeId> {
        if !self.infcx.has_equations() {
            return Ok(type_id);
        }

        let mut result = self.substitute_type(type_id)?;

        loop {
            let fixed_point = self.substitute_type(type_id)?;

            if fixed_point == result {
                break;
            } else {
                result = fixed_point;
            }
        }

        Ok(result)
    }

    fn substitute_type(&mut self, type_id: TypeId) -> Result<TypeId> {
        if let Some(subst_type) = self.infcx.probe(type_id) {
            return Ok(subst_type);
        }

        match self.symbols[type_id].clone() {
            Type::TypeVariable(_) => Ok(type_id),

            Type::Array(array) => {
                let ty = Type::Array(CheckedArrayNode {
                    inner_ty: self.substitute_all(array.inner_ty)?,
                    size_ty: self.substitute_all(array.size_ty)?,
                    scope_id: array.scope_id,
                });
                self.symbols
                    .get_or_add_type(Some(ScopeId::primitive()), ty.key(), ty)
            }

            Type::Struct(struct_node) => {
                let mut new_fields = IndexMap::new();
                for (
                    field_name,
                    CheckedStructField {
                        ty: field_type,
                        visibility,
                        span,
                    },
                ) in struct_node.fields
                {
                    let substituted_type = self.substitute_all(field_type)?;
                    new_fields.insert(
                        field_name,
                        CheckedStructField {
                            ty: substituted_type,
                            visibility,
                            span,
                        },
                    );
                }

                let mut new_generic_parameters = Vec::new();
                for generic_param in struct_node.generic_parameters {
                    new_generic_parameters.push(self.substitute_all(generic_param)?);
                }

                let ty = Type::Struct(CheckedStructNode {
                    name: struct_node.name,
                    generic_parameters: new_generic_parameters,
                    fields: new_fields,
                    scope_id: struct_node.scope_id,
                    visibility: struct_node.visibility,
                    span: struct_node.span,
                });
                let scope_id = self.symbols[struct_node.scope_id].parent;
                self.symbols.get_or_add_type(scope_id, ty.key(), ty)
            }

            _ => Ok(type_id),
        }
    }

    fn is_concrete(&self, type_id: TypeId) -> bool {
        match &self.symbols[type_id] {
            Type::TypeVariable(_) => false,

            Type::Struct(struct_node) => {
                struct_node
                    .generic_parameters
                    .iter()
                    .all(|&param| self.is_concrete(param))
                    && struct_node
                        .fields
                        .values()
                        .all(|field| self.is_concrete(field.ty))
            }

            Type::Array(array) => {
                self.is_concrete(array.inner_ty) && self.is_concrete(array.size_ty)
            }

            Type::Tuple(elements) => elements.iter().all(|&elem| self.is_concrete(elem)),

            Type::Function(func) => {
                func.generic_parameters
                    .iter()
                    .all(|&param| self.is_concrete(param))
                    && func
                        .parameters
                        .iter()
                        .all(|parameter| self.is_concrete(parameter.ty))
                    && self.is_concrete(func.return_type)
            }

            Type::LambdaFunction(lambda) => {
                lambda
                    .parameters
                    .iter()
                    .all(|parameter| self.is_concrete(parameter.ty))
                    && self.is_concrete(lambda.return_type)
            }

            Type::FunctionSignature(sig) => {
                sig.parameters.iter().all(|&param| self.is_concrete(param))
                    && self.is_concrete(sig.return_type)
            }

            Type::Enum(enum_node) => enum_node
                .generic_parameters
                .iter()
                .all(|&param| self.is_concrete(param)),

            Type::Const(const_node) => self.is_concrete(const_node.ty),

            Type::Bool | Type::Felt | Type::U32 | Type::Unknown | Type::VOID => true,

            Type::Trait(_) => true,
        }
    }
}
