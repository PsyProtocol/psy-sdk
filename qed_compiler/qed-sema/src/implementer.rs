use anyhow::anyhow;
use indexmap::{IndexMap, IndexSet};
use qed_ast::{DefId, IdentId};
use qedlang_core::dpn::ops::context_trait::ContextFelt;

use crate::{
    rewriter::Rewriter, CheckedImplNode, CheckedTraitImplNode, Constraint, Result, TypeChecker,
    TypeCheckerVisitorContext, TypeId, TypeKey,
};

#[derive(Debug)]
pub struct ImplementerCtxt {
    // poly
    impl_ids: IndexMap<TypeId, IndexMap<Constraint, IndexSet<DefId>>>,
    // trait poly -> poly
    trait_impls: IndexMap<TypeId, IndexMap<Constraint, IndexSet<TypeId>>>,
    // poly -> instance
    instances: IndexMap<TypeId, IndexMap<Constraint, DefId>>,
}

impl ImplementerCtxt {
    pub fn new() -> Self {
        Self {
            impl_ids: IndexMap::new(),
            trait_impls: IndexMap::new(),
            instances: IndexMap::new(),
        }
    }

    pub fn impl_ids(&self) -> &IndexMap<TypeId, IndexMap<Constraint, IndexSet<DefId>>> {
        &self.impl_ids
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
    fn register_instance(
        &mut self,
        def_id: DefId,
        ctx: &mut TypeCheckerVisitorContext<F, C>,
    ) -> Result<()>;
    fn find_instance(
        &mut self,
        ty: TypeId,
        generic_parameters: Vec<TypeId>,
        ctx: &mut TypeCheckerVisitorContext<F, C>,
    ) -> Option<DefId>;
    fn find_member(
        &mut self,
        ty: TypeId,
        method: IdentId,
        ctx: &mut TypeCheckerVisitorContext<F, C>,
    ) -> Result<TypeId>;
    fn find_trait_cast_member(
        &mut self,
        ty: TypeId,
        trait_ty: TypeId,
        member: IdentId,
        ctx: &mut TypeCheckerVisitorContext<F, C>,
    ) -> Result<TypeId>;
    fn get_impl_id<R: Copy>(
        &self,
        ty: TypeId,
        f: impl Fn(&CheckedImplNode) -> Option<R>,
        ctx: &mut TypeCheckerVisitorContext<F, C>,
    ) -> Option<(Constraint, DefId, R)>;
    fn get_trait_impl_ids<R: Copy>(
        &self,
        ty: TypeId,
        f: impl Fn(&CheckedTraitImplNode) -> Option<R>,
        ctx: &mut TypeCheckerVisitorContext<F, C>,
    ) -> Option<Vec<(Constraint, Vec<(Constraint, DefId, R)>)>>;
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

        let _ = self
            .implementer
            .impl_ids
            .entry(self.poly_of(impl_node.ty, ctx).unwrap())
            .or_insert_with(IndexMap::new)
            .entry(constraint)
            .or_insert_with(IndexSet::new)
            .insert(impl_id);

        Ok(())
    }

    fn register_trait_impl(
        &mut self,
        impl_id: DefId,
        ctx: &mut TypeCheckerVisitorContext<F, C>,
    ) -> Result<()> {
        let trait_impl_node = self.program[impl_id].as_trait_impl().unwrap();
        let trait_constraint =
            Constraint::new(ctx.symbols[trait_impl_node.trait_ty].generic_parameters());
        let constraint = Constraint::new(ctx.symbols[trait_impl_node.ty].generic_parameters());

        let trait_poly_ty = self.poly_of(trait_impl_node.trait_ty, ctx).unwrap();
        let poly_ty = self.poly_of(trait_impl_node.ty, ctx).unwrap();

        self.implementer
            .trait_impls
            .entry(trait_poly_ty)
            .or_insert_with(IndexMap::new)
            .entry(trait_constraint)
            .or_insert_with(IndexSet::new)
            .insert(poly_ty);

        self.implementer
            .impl_ids
            .entry(poly_ty)
            .or_insert_with(IndexMap::new)
            .entry(constraint)
            .or_insert_with(IndexSet::new)
            .insert(impl_id);

        Ok(())
    }

    fn register_instance(
        &mut self,
        func_id: DefId,
        ctx: &mut TypeCheckerVisitorContext<F, C>,
    ) -> Result<()> {
        let type_id = self.program[func_id].type_id();
        let poly_ty = self.poly_of(type_id, ctx).unwrap();

        let constraint = Constraint::new(ctx.symbols[type_id].generic_parameters());

        self.implementer
            .instances
            .entry(poly_ty)
            .or_insert_with(IndexMap::new)
            .entry(constraint)
            .or_insert(func_id);

        Ok(())
    }

    fn find_instance(
        &mut self,
        ty: TypeId,
        generic_parameters: Vec<TypeId>,
        ctx: &mut TypeCheckerVisitorContext<F, C>,
    ) -> Option<DefId> {
        if let Some(instance_map) = self.implementer.instances.get(&ty) {
            if let Some(&instance) = instance_map.get(&Constraint::new(generic_parameters.clone()))
            {
                return Some(instance);
            }
        }
        None
    }

    fn find_trait_cast_member(
        &mut self,
        ty: TypeId,
        trait_ty: TypeId,
        member: IdentId,
        ctx: &mut TypeCheckerVisitorContext<F, C>,
    ) -> Result<TypeId> {
        let poly_ty = self.poly_of(ty, ctx).unwrap();
        let generic_parameters = ctx.symbols[ty].generic_parameters();

        let get_trait_impl_member_id = |poly_ty: TypeId,
                                        trait_ty: TypeId,
                                        ctx: &mut TypeCheckerVisitorContext<F, C>|
         -> Option<_> {
            let impl_map = self.implementer.impl_ids.get(&poly_ty)?;
            let constraint = Constraint::new(generic_parameters.clone());

            let mut get_result = |impl_set: &IndexSet<DefId>| {
                for &impl_id in impl_set {
                    if let Some(impl_node) = self.program[impl_id].as_trait_impl() {
                        if self.poly_of(impl_node.trait_ty, ctx) != self.poly_of(trait_ty, ctx) {
                            continue;
                        }

                        if let Some(function_idx) = impl_node.body.iter().position(|&function_id| {
                            self.program[function_id].as_function().unwrap().name == member
                        }) {
                            if ctx
                                .symbols
                                .get_type_id(None, ctx.symbols[trait_ty].name())
                                .is_none()
                            {
                                return None;
                            } else {
                                return Some(
                                    self.program[impl_node.body[function_idx]]
                                        .as_function()
                                        .unwrap()
                                        .type_id,
                                );
                            }
                        }

                        for (ty_name, ty_val) in impl_node.associated_types.iter() {
                            if ty_name.id == member {
                                return Some(ty_val.type_id);
                            }
                        }
                    }
                }
                None
            };

            if let Some(impl_set) = impl_map.get(&constraint) {
                if let Some(member_type_id) = get_result(impl_set) {
                    return Some(member_type_id);
                }
            }

            for (_constraint, impl_set) in impl_map.iter() {
                if let Some(member_type_id) = get_result(impl_set) {
                    return Some(member_type_id);
                }
            }
            None
        };

        if let Some(member_ty_id) = get_trait_impl_member_id(poly_ty, trait_ty, ctx) {
            return Ok(member_ty_id);
        }

        return Err(anyhow!("type as trait member not found").into());
    }

    fn find_member(
        &mut self,
        ty: TypeId, // mono OR poly
        member: IdentId,
        ctx: &mut TypeCheckerVisitorContext<F, C>,
    ) -> Result<TypeId> {
        let poly_ty = self.poly_of(ty, ctx).unwrap();
        let generic_parameters = ctx.symbols[ty].generic_parameters();

        let get_trait_member =
            |trait_type_id: TypeId, ctx: &mut TypeCheckerVisitorContext<F, C>| -> Option<_> {
                let trait_scope_id = ctx.symbols[trait_type_id].scope_id();
                if let Some(&method_type_id) = ctx.symbols[trait_scope_id]
                    .types
                    .get::<TypeKey>(&member.into())
                {
                    return Some(method_type_id);
                }
                None
            };

        if let Some((constraint, impl_id, function_idx)) = self.get_impl_id(
            ty,
            |impl_node: &CheckedImplNode| {
                impl_node.body.iter().position(|&function_id| {
                    self.program[function_id].as_function().unwrap().name == member
                })
            },
            ctx,
        ) {
            if generic_parameters == constraint.constraints {
                let function_id = self.program[impl_id].as_impl().unwrap().body[function_idx];
                return Ok(self.program[function_id].as_function().unwrap().type_id);
            }

            if self.satisfies_constraints(generic_parameters.clone(), &constraint, ctx) {
                let instance = self.instantiate_impl(impl_id, generic_parameters.clone(), ctx)?;
                let function_id = self.program[instance].as_impl().unwrap().body[function_idx];
                return Ok(self.program[function_id].as_function().unwrap().type_id);
            }
        } else if let Some(results) = self.get_trait_impl_ids(
            ty,
            |trait_impl_node: &CheckedTraitImplNode| {
                trait_impl_node.body.iter().position(|&function_id| {
                    self.program[function_id].as_function().unwrap().name == member
                })
            },
            ctx,
        ) {
            for (constraint, result) in results {
                for (trait_constraint, impl_id, function_idx) in result {
                    if generic_parameters == constraint.constraints {
                        let function_id =
                            self.program[impl_id].as_trait_impl().unwrap().body[function_idx];
                        return Ok(self.program[function_id].as_function().unwrap().type_id);
                    }

                    if self.satisfies_constraints(generic_parameters.clone(), &constraint, ctx) {
                        let instance = self.instantiate_trait_impl(
                            impl_id,
                            trait_constraint.constraints,
                            generic_parameters.clone(),
                            ctx,
                        )?;
                        let function_id =
                            self.program[instance].as_trait_impl().unwrap().body[function_idx];
                        return Ok(self.program[function_id].as_function().unwrap().type_id);
                    }
                }
            }
        } else if let Some(constraints) = ctx.symbols[ty]
            .as_type_variable()
            .map(|x| x.constraints.clone())
        {
            for trait_type_id in constraints.into_iter() {
                if let Some(method_type_id) = get_trait_member(trait_type_id, ctx) {
                    return Ok(method_type_id);
                }
            }
        } else if let Some((constraint, impl_id, name)) = self.get_impl_id(
            ty,
            |impl_node: &CheckedImplNode| {
                impl_node
                    .associated_types
                    .iter()
                    .find(|(name, ty)| name.id == member)
                    .map(|(name, _)| name.clone())
            },
            ctx,
        ) {
            if generic_parameters == constraint.constraints {
                return Ok(self.program[impl_id]
                    .as_impl()
                    .and_then(|x| x.associated_types.get(&name))
                    .unwrap()
                    .type_id);
            }

            if self.satisfies_constraints(generic_parameters.clone(), &constraint, ctx) {
                let instance = self.instantiate_impl(impl_id, generic_parameters.clone(), ctx)?;
                return Ok(self.program[instance]
                    .as_impl()
                    .and_then(|x| x.associated_types.get(&name))
                    .unwrap()
                    .type_id);
            }
        } else if let Some(results) = self.get_trait_impl_ids(
            ty,
            |impl_node: &CheckedTraitImplNode| {
                impl_node
                    .associated_types
                    .iter()
                    .find(|(name, ty)| name.id == member)
                    .map(|(name, _)| name.clone())
            },
            ctx,
        ) {
            for (constraint, result) in results {
                for (trait_constraint, impl_id, name) in result {
                    if generic_parameters == constraint.constraints {
                        return Ok(self.program[impl_id]
                            .as_trait_impl()
                            .and_then(|x| x.associated_types.get(&name))
                            .unwrap()
                            .type_id);
                    }

                    if self.satisfies_constraints(generic_parameters.clone(), &constraint, ctx) {
                        let instance = self.instantiate_trait_impl(
                            impl_id,
                            trait_constraint.constraints,
                            generic_parameters.clone(),
                            ctx,
                        )?;
                        return Ok(self.program[instance]
                            .as_trait_impl()
                            .and_then(|x| x.associated_types.get(&name))
                            .unwrap()
                            .type_id);
                    }
                }
            }
        }

        return Err(anyhow!("member not found").into());
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

        let find_constraints = |trait_poly_ty: TypeId,
                                poly_ty: TypeId,
                                _trait_ty: TypeId,
                                _ty: TypeId,
                                ctx: &mut TypeCheckerVisitorContext<F, C>|
         -> Vec<(Constraint, Constraint)> {
            let mut result = Vec::new();
            if let Some(impl_map) = self.implementer.impl_ids.get(&poly_ty) {
                for (constraint, impl_set) in impl_map.iter() {
                    for &impl_id in impl_set {
                        if let Some(impl_node) = self.program[impl_id].as_trait_impl() {
                            if self.poly_of(impl_node.trait_ty, ctx).unwrap() == trait_poly_ty {
                                if let Some(trait_map) =
                                    self.implementer.trait_impls.get(&trait_poly_ty)
                                {
                                    for (trait_constraint, trait_impl_set) in trait_map.iter() {
                                        if trait_impl_set.contains(&poly_ty) {
                                            result.push((
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
            result
        };

        for (constraint, trait_constraint) in
            find_constraints(trait_poly_ty, poly_ty, trait_ty, ty, ctx)
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

    // rhs_ty will be substituted by lhs_ty if they are both type variables
    // so lhs_ty should be the stricter one
    fn satisfies_constraint(
        &mut self,
        lhs_ty: TypeId,
        rhs_ty: TypeId,
        ctx: &mut TypeCheckerVisitorContext<F, C>,
    ) -> bool {
        let lhs_ty = self.substitute_all(lhs_ty, ctx).unwrap();
        let rhs_ty = self.substitute_all(rhs_ty, ctx).unwrap();

        let is_lhs_var = ctx.symbols[lhs_ty].is_type_variable();
        let is_rhs_var = ctx.symbols[rhs_ty].is_type_variable();

        self.infcx.enter_scope();
        let satisfied = match (is_rhs_var, is_lhs_var) {
            (false, false) => self.unify(rhs_ty, lhs_ty, ctx),

            (true, false) => {
                let rhs_traits = ctx.symbols[rhs_ty]
                    .as_type_variable()
                    .unwrap()
                    .constraints
                    .clone();
                rhs_traits.is_empty()
                    || rhs_traits.iter().all(|&type_id| {
                        if ctx.symbols[type_id].is_trait() {
                            return self.implements_trait(lhs_ty, type_id, ctx);
                        }
                        self.unify(lhs_ty, type_id, ctx)
                    })
            }

            (false, true) => {
                let lhs_traits = ctx.symbols[lhs_ty]
                    .as_type_variable()
                    .unwrap()
                    .constraints
                    .clone();
                lhs_traits.is_empty()
                    || lhs_traits.iter().all(|&type_id| {
                        if ctx.symbols[type_id].is_trait() {
                            return self.implements_trait(rhs_ty, type_id, ctx);
                        }
                        self.unify(rhs_ty, type_id, ctx)
                    })
            }

            (true, true) => {
                let rhs_traits = ctx.symbols[rhs_ty]
                    .as_type_variable()
                    .unwrap()
                    .constraints
                    .clone();
                let lhs_traits = ctx.symbols[lhs_ty]
                    .as_type_variable()
                    .unwrap()
                    .constraints
                    .clone();
                rhs_traits
                    .iter()
                    .all(|r_trait| lhs_traits.contains(&r_trait))
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
        ctx.symbols
            .get_type_id(Some(ctx.symbols[type_id].scope_id()), name)
    }

    fn get_impl_id<R: Copy>(
        &self,
        ty: TypeId,
        f: impl Fn(&CheckedImplNode) -> Option<R>,
        ctx: &mut TypeCheckerVisitorContext<F, C>,
    ) -> Option<(Constraint, DefId, R)> {
        let poly_ty = self.poly_of(ty, ctx).unwrap();
        let generic_parameters = ctx.symbols[ty].generic_parameters();

        let get_result = |impl_set: &IndexSet<DefId>| {
            for &impl_id in impl_set {
                if let Some(impl_node) = self.program[impl_id].as_impl() {
                    if let Some(function_idx) = f(impl_node) {
                        return Some((impl_id, function_idx));
                    }
                }
            }
            None
        };

        let impl_map = self.implementer.impl_ids.get(&poly_ty)?;
        let constraint = Constraint::new(generic_parameters.clone());
        if let Some(impl_set) = impl_map.get(&constraint) {
            if let Some((impl_id, function_idx)) = get_result(impl_set) {
                return Some((constraint, impl_id, function_idx));
            }
        }

        for (constraint, impl_set) in impl_map.iter() {
            if let Some((impl_id, function_idx)) = get_result(impl_set) {
                return Some((constraint.clone(), impl_id, function_idx));
            }
        }

        None
    }

    fn get_trait_impl_ids<R: Copy>(
        &self,
        ty: TypeId,
        f: impl Fn(&CheckedTraitImplNode) -> Option<R>,
        ctx: &mut TypeCheckerVisitorContext<F, C>,
    ) -> Option<Vec<(Constraint, Vec<(Constraint, DefId, R)>)>> {
        let poly_ty = self.poly_of(ty, ctx).unwrap();
        let generic_parameters = ctx.symbols[ty].generic_parameters();

        let mut get_result = |impl_set: &IndexSet<DefId>| {
            let mut result = Vec::new();
            for &impl_id in impl_set {
                if let Some(impl_node) = self.program[impl_id].as_trait_impl() {
                    if let Some(function_idx) = f(impl_node) {
                        let trait_poly_ty = self.poly_of(impl_node.trait_ty, ctx).unwrap();
                        if ctx
                            .symbols
                            .get_type_id(None, ctx.symbols[trait_poly_ty].name())
                            .is_none()
                        {
                            continue;
                        }
                        if let Some(trait_map) = self.implementer.trait_impls.get(&trait_poly_ty) {
                            for (trait_constraint, trait_impl_set) in trait_map.iter() {
                                if trait_impl_set.contains(&poly_ty) {
                                    result.push((trait_constraint.clone(), impl_id, function_idx));
                                }
                            }
                        }
                    }
                }
            }
            result
        };

        let mut results = Vec::new();
        let impl_map = self.implementer.impl_ids.get(&poly_ty)?;
        let constraint = Constraint::new(generic_parameters.clone());
        if let Some(impl_set) = impl_map.get(&constraint) {
            let result = get_result(impl_set);
            if !result.is_empty() {
                return Some(vec![(constraint.clone(), result)]);
            }
        }

        for (constraint, impl_set) in impl_map.iter() {
            let result = get_result(impl_set);
            if !result.is_empty() {
                results.push((constraint.clone(), result));
            }
        }

        if !results.is_empty() {
            return Some(results);
        }

        None
    }
}
