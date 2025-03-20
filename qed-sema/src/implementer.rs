use std::collections::{HashMap, HashSet};

use anyhow::anyhow;
use qed_ast::{DefId, IdentId, ImplTraitNode, Span, VisitorContext};
use qedlang_core::dpn::ops::context_trait::ContextFelt;

use crate::{
    AstVisualizer, Constraint, Inferer, Result, ScopeKind, TypeChecker, TypeCheckerVisitorContext,
    TypeId,
};

#[derive(Debug)]
pub struct ImplementerCtxt {
    // poly
    impl_ids: HashMap<TypeId, HashMap<Constraint, HashSet<DefId>>>,
    // trait poly -> poly
    trait_impls: HashMap<TypeId, HashMap<Constraint, TypeId>>,
}

impl ImplementerCtxt {
    pub fn new() -> Self {
        Self {
            impl_ids: HashMap::new(),
            trait_impls: HashMap::new(),
        }
    }
}

pub trait Implementer<F: Clone + From<u32> + ContextFelt, C> {
    fn register_impl(
        &mut self,
        impl_id: DefId,
        ctx: &mut TypeCheckerVisitorContext<F, C>,
    ) -> Result<()>;
    fn register_trait_impl(
        &mut self,
        impl_id: DefId,
        ctx: &mut TypeCheckerVisitorContext<F, C>,
    ) -> Result<()>;
    fn find_impl(
        &mut self,
        ty: TypeId,
        method: IdentId,
        ctx: &mut TypeCheckerVisitorContext<F, C>,
    ) -> Result<(DefId, TypeId)>;
    fn implements_trait(
        &mut self,
        ty: TypeId,       // mono
        trait_ty: TypeId, // mono OR poly
        ctx: &mut TypeCheckerVisitorContext<F, C>,
    ) -> bool;
    fn satisfies_constraint(
        &mut self,
        gen_ty: TypeId,
        constr_ty: TypeId,
        ctx: &mut TypeCheckerVisitorContext<F, C>,
    ) -> bool;
    fn satisfies_constraints(
        &mut self,
        generics: Vec<TypeId>,
        constraint: &Constraint,
        ctx: &mut TypeCheckerVisitorContext<F, C>,
    ) -> bool;
    fn poly_of(&self, type_id: TypeId, ctx: &mut TypeCheckerVisitorContext<F, C>)
        -> Option<TypeId>;
}

impl<F: Clone + From<u32> + ContextFelt, C> Implementer<F, C> for TypeChecker<F, C> {
    fn register_impl(
        &mut self,
        impl_id: DefId,
        ctx: &mut TypeCheckerVisitorContext<F, C>,
    ) -> Result<()> {
        let impl_node = self.program[impl_id].as_impl().unwrap();
        let constraint = Constraint::new(ctx.symbols[impl_node.ty].generic_parameters());

        let res = self
            .implementer
            .impl_ids
            .entry(impl_node.ty)
            .or_insert_with(HashMap::new)
            .entry(constraint)
            .or_insert_with(HashSet::new)
            .insert(impl_id);

        Ok(())
    }

    fn register_trait_impl(
        &mut self,
        impl_id: DefId,
        ctx: &mut TypeCheckerVisitorContext<F, C>,
    ) -> Result<()> {
        let trait_impl_node = self.program[impl_id].as_impl_trait().unwrap();
        let trait_constraint =
            Constraint::new(ctx.symbols[trait_impl_node.trait_ty].generic_parameters());
        let constraint = Constraint::new(ctx.symbols[trait_impl_node.ty].generic_parameters());

        self.implementer
            .trait_impls
            .entry(trait_impl_node.trait_ty)
            .or_insert_with(HashMap::new)
            .insert(trait_constraint, trait_impl_node.ty);

        self.implementer
            .impl_ids
            .entry(trait_impl_node.ty)
            .or_insert_with(HashMap::new)
            .entry(constraint)
            .or_insert_with(HashSet::new)
            .insert(impl_id);

        Ok(())
    }

    fn find_impl(
        &mut self,
        ty: TypeId, // mono OR poly
        method: IdentId,
        ctx: &mut TypeCheckerVisitorContext<F, C>,
    ) -> Result<(DefId, TypeId)> {
        eprintln!(
            "DEBUGPRINT[20]: implementer.rs:121: ctx.debug_type(ty)={:#?}",
            ctx.debug_type(ty)
        );
        let poly_ty = self.poly_of(ty, ctx).unwrap();

        let get_impl_id = |poly_ty: TypeId,
                           method: IdentId,
                           ctx: &mut TypeCheckerVisitorContext<F, C>|
         -> Option<_> {
            if let Some(impl_map) = self.implementer.impl_ids.get(&poly_ty) {
                for (constraint, impl_set) in impl_map.iter() {
                    for &impl_id in impl_set {
                        if let Some(impl_node) = self.program[impl_id].as_impl() {
                            if impl_node.body.iter().any(|&def_id| {
                                self.program[def_id].as_function().unwrap().name == method
                            }) {
                                return Some((constraint.clone(), impl_node.scope_id, impl_id));
                            }
                        }

                        if let Some(impl_node) = self.program[impl_id].as_impl_trait() {
                            if impl_node.body.iter().any(|&def_id| {
                                self.program[def_id].as_function().unwrap().name == method
                            }) {
                                return Some((constraint.clone(), impl_node.scope_id, impl_id));
                            }
                        }
                    }
                }
            }

            None
        };

        if let Some((constraint, scope_id, impl_id)) = get_impl_id(poly_ty, method, ctx) {
            if self.satisfies_constraints(ctx.symbols[ty].generic_parameters(), &constraint, ctx) {
                for &scope_id in &ctx.symbols[scope_id].children {
                    if ctx.symbols[scope_id].kind == ScopeKind::ImplMethod {
                        if let Some(&type_id) = ctx.symbols[scope_id].types.get(&method.into()) {
                            return Ok((impl_id, type_id));
                        }
                    }
                }
            }
        }

        return Err(anyhow!("method not found").into());
    }

    fn implements_trait(
        &mut self,
        ty: TypeId,       // mono
        trait_ty: TypeId, // mono or poly
        ctx: &mut TypeCheckerVisitorContext<F, C>,
    ) -> bool {
        let poly_ty = match self.poly_of(ty, ctx) {
            Some(ty) => ty,
            None => return false,
        };
        let trait_poly_ty = match self.poly_of(trait_ty, ctx) {
            Some(ty) => ty,
            None => return false,
        };

        let find_constraint = |trait_poly_ty: TypeId,
                               poly_ty: TypeId,
                               trait_ty: TypeId,
                               ty: TypeId,
                               ctx: &mut TypeCheckerVisitorContext<F, C>|
         -> Option<(Constraint, Constraint)> {
            if let Some(impl_map) = self.implementer.impl_ids.get(&poly_ty) {
                for (constraint, impl_set) in impl_map.iter() {
                    for &impl_id in impl_set {
                        if let Some(impl_node) = self.program[impl_id].as_impl_trait() {
                            let impl_trait_ty = impl_node.trait_ty;

                            if impl_trait_ty == trait_poly_ty {
                                if let Some(trait_map) =
                                    self.implementer.trait_impls.get(&trait_poly_ty)
                                {
                                    for (trait_constraint, trait_impl_ty) in trait_map.iter() {
                                        if trait_impl_ty == &poly_ty {
                                            return Some((
                                                constraint.clone(),
                                                trait_constraint.clone(),
                                            ));
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            None
        };

        if let Some((constraint, trait_constraint)) =
            find_constraint(trait_poly_ty, poly_ty, trait_ty, ty, ctx)
        {
            if self.satisfies_constraints(ctx.symbols[ty].generic_parameters(), &constraint, ctx) {
                if self.satisfies_constraints(
                    ctx.symbols[trait_ty].generic_parameters(),
                    &trait_constraint,
                    ctx,
                ) {
                    return true;
                }
            }
        }

        false
    }

    fn satisfies_constraint(
        &mut self,
        gen_ty: TypeId,
        constr_ty: TypeId,
        ctx: &mut TypeCheckerVisitorContext<F, C>,
    ) -> bool {
        let gen_ty = self.substitute_all(gen_ty, ctx).unwrap();
        let constr_ty = self.substitute_all(constr_ty, ctx).unwrap();

        let is_gen_var = ctx.symbols[gen_ty].is_type_variable();
        let is_constr_var = ctx.symbols[constr_ty].is_type_variable();

        self.infcx.enter_scope();
        let satisfied = match (is_constr_var, is_gen_var) {
            (false, false) => self.unify(constr_ty, gen_ty, ctx),

            (true, false) => {
                let constr_traits = ctx.symbols[constr_ty]
                    .as_type_variable()
                    .unwrap()
                    .constraints
                    .clone();
                constr_traits.is_empty()
                    || constr_traits.iter().all(|&type_id| {
                        if ctx.symbols[type_id].is_trait() {
                            return self.implements_trait(gen_ty, type_id, ctx);
                        }
                        self.unify(type_id, gen_ty, ctx)
                    })
            }

            (false, true) => {
                let gen_traits = ctx.symbols[gen_ty]
                    .as_type_variable()
                    .unwrap()
                    .constraints
                    .clone();
                gen_traits.is_empty()
                    || gen_traits.iter().all(|&type_id| {
                        if ctx.symbols[type_id].is_trait() {
                            return self.implements_trait(constr_ty, type_id, ctx);
                        }
                        self.unify(type_id, constr_ty, ctx)
                    })
            }

            (true, true) => {
                let constr_traits = ctx.symbols[constr_ty]
                    .as_type_variable()
                    .unwrap()
                    .constraints
                    .clone();
                let gen_traits = ctx.symbols[gen_ty]
                    .as_type_variable()
                    .unwrap()
                    .constraints
                    .clone();
                if constr_traits.len() != gen_traits.len() {
                    false
                } else {
                    constr_traits
                        .iter()
                        .all(|c_trait| gen_traits.contains(&c_trait))
                }
            }
        };
        self.infcx.exit_scope();

        satisfied
    }

    fn satisfies_constraints(
        &mut self,
        generic_args: Vec<TypeId>,
        constraint: &Constraint,
        ctx: &mut TypeCheckerVisitorContext<F, C>,
    ) -> bool {
        if generic_args.len() != constraint.constraints.len() {
            return false;
        }

        let satisfied = constraint
            .constraints
            .iter()
            .zip(generic_args.iter())
            .all(|(constr_ty, gen_ty)| self.satisfies_constraint(*gen_ty, *constr_ty, ctx));

        satisfied
    }

    fn poly_of(
        &self,
        type_id: TypeId,
        ctx: &mut TypeCheckerVisitorContext<F, C>,
    ) -> Option<TypeId> {
        let name = ctx.symbols[type_id].name();
        ctx.symbols.get_type_id(None, name)
    }
}
