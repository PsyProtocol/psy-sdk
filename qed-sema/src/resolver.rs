use std::collections::{HashMap, HashSet};

use qed_ast::{IdentId, ImplTraitNode, Location, ModuleId, PathNode, UncheckedType, UseNode};
use qedlang_core::dpn::ops::context_trait::ContextFelt;

use crate::{
    AstVisualizer, CheckedPathNode, Error, Implementer, Result, TypeChecker,
    TypeCheckerVisitorContext, TypeId, TypeKey,
};

#[derive(Debug)]
pub struct ResolverCtxt {}

impl ResolverCtxt {
    pub fn new() -> Self {
        Self {}
    }
}

pub trait Resolver<F: Clone + From<u32> + ContextFelt, C> {
    fn resolve_path(
        &mut self,
        path: &PathNode,
        ctx: &mut TypeCheckerVisitorContext<F, C>,
    ) -> Result<CheckedPathNode>;

    fn resolve_use(
        &self,
        use_path: &UseNode,
        ctx: &mut TypeCheckerVisitorContext<F, C>,
    ) -> Result<Vec<(TypeKey, TypeId)>>;

    fn resolve_member_type(
        &mut self,
        path: &PathNode,
        root: TypeId,
        target: IdentId,
        ctx: &mut TypeCheckerVisitorContext<F, C>,
    ) -> Result<TypeId>;

    fn resolve_module_type(
        &mut self,
        path: &PathNode,
        module: ModuleId,
        ty: IdentId,
        ctx: &mut TypeCheckerVisitorContext<F, C>,
    ) -> Result<TypeId>;
}

impl<F: Clone + From<u32> + ContextFelt, C> Resolver<F, C> for TypeChecker<F, C> {
    fn resolve_path(
        &mut self,
        path: &PathNode,
        ctx: &mut TypeCheckerVisitorContext<F, C>,
    ) -> Result<CheckedPathNode> {
        let current_module_id = ctx.symbols.current_module_id().unwrap();

        let mut src_module = match path.root.as_ref() {
            Some(ty) => match ty {
                UncheckedType::Basic(name, _) => match name.id {
                    IdentId::SELF => current_module_id,
                    IdentId::CRATE => {
                        let mut module_id = current_module_id;
                        while let Some(parent) = ctx.symbols[module_id].parent {
                            module_id = parent;
                        }
                        module_id
                    }
                    IdentId::SUPER => {
                        ctx.symbols[current_module_id]
                            .parent
                            .ok_or(Error::NoParentModule {
                                location: path.location,
                            })?
                    }
                    _ => {
                        if let Some(&module_id) = ctx.symbols[current_module_id]
                            .children
                            .iter()
                            .find(|&x| ctx.symbols[*x].name == name.id)
                        {
                            module_id
                        } else {
                            if !path.segments.is_empty() {
                                return Err(Error::InvalidPathSegment {
                                    location: path.location,
                                    segment: path.segments[0].id,
                                });
                            }
                            let root_type_id = self.typecheck(ty, ctx)?;
                            return Ok(CheckedPathNode::new(
                                None,
                                Some(root_type_id),
                                self.resolve_member_type(path, root_type_id, path.target.id, ctx)?,
                                path.location,
                            ));
                        }
                    }
                },
                _ => {
                    if !path.segments.is_empty() {
                        return Err(Error::InvalidPathSegment {
                            location: path.location,
                            segment: path.segments[0].id,
                        });
                    }
                    let root_type_id = self.typecheck(ty, ctx)?;
                    return Ok(CheckedPathNode::new(
                        None,
                        Some(root_type_id),
                        self.resolve_member_type(path, root_type_id, path.target.id, ctx)?,
                        path.location,
                    ));
                }
            },
            None => {
                if !path.segments.is_empty() {
                    return Err(Error::InvalidPathSegment {
                        location: path.location,
                        segment: path.segments[0].id,
                    });
                }
                if let Some(var_id) = ctx.symbols.get_variable(None, &path.target.id) {
                    return Ok(CheckedPathNode::new(
                        Some(var_id),
                        None,
                        ctx.symbols[var_id].ty,
                        path.location,
                    ));
                }
                let type_id = ctx
                    .symbols
                    .get_type_id(None, path.target.id)
                    .ok_or_else(|| Error::UnresolvedType {
                        location: path.location,
                        resolved_type: path.target.id,
                    })?;
                return Ok(CheckedPathNode::new(None, None, type_id, path.location));
            }
        };

        for (i, segment) in path.segments.iter().enumerate() {
            if let Some(&target_module_id) = ctx.symbols[src_module]
                .children
                .iter()
                .find(|&&id| ctx.symbols[id].name == segment.id)
            {
                if !ctx.symbols[target_module_id].visibility.is_public() {
                    return Err(Error::ModuleNotPublic {
                        location: path.location,
                        module: ctx.symbols[target_module_id].name,
                    });
                }
                src_module = target_module_id;
            } else {
                if i != path.segments.len() - 1 {
                    return Err(Error::InvalidPathSegment {
                        location: path.location,
                        segment: segment.id,
                    });
                }
                let root_type_id = self.resolve_module_type(path, src_module, segment.id, ctx)?;
                return Ok(CheckedPathNode::new(
                    None,
                    Some(root_type_id),
                    self.resolve_member_type(path, root_type_id, path.target.id, ctx)?,
                    path.location,
                ));
            }
        }

        let type_id = self.resolve_module_type(path, src_module, path.target.id, ctx)?;
        return Ok(CheckedPathNode::new(None, None, type_id, path.location));
    }

    fn resolve_use(
        &self,
        use_path: &UseNode,
        ctx: &mut TypeCheckerVisitorContext<F, C>,
    ) -> Result<Vec<(TypeKey, TypeId)>> {
        let symbols = &ctx.symbols;
        let current_module_id = symbols.current_module_id().unwrap();

        let mut module_id = match use_path.kind.id {
            IdentId::SELF => current_module_id,
            IdentId::CRATE => {
                let mut root_id = current_module_id;
                while let Some(parent) = symbols[root_id].parent {
                    root_id = parent;
                }
                root_id
            }
            IdentId::SUPER => symbols[current_module_id]
                .parent
                .ok_or(Error::NoParentModule {
                    location: use_path.location,
                })?,
            name => symbols
                .modules()
                .iter()
                .position(|x| x.name == name)
                .map(ModuleId)
                .filter(|&id| symbols[current_module_id].children.contains(&id))
                .ok_or(Error::ModuleNotFound {
                    location: use_path.location,
                    module: name,
                })?,
        };

        module_id = use_path
            .segments
            .iter()
            .try_fold(module_id, |current, &segment| {
                if let Some(module) = symbols[current]
                    .children
                    .iter()
                    .find(|&&id| symbols[id].name == segment.id)
                    .copied()
                {
                    if ctx.symbols[module].visibility.is_public() {
                        Ok(module)
                    } else {
                        Err(Error::ModuleNotPublic {
                            location: use_path.location,
                            module: segment.id,
                        })
                    }
                } else {
                    Err(Error::ModuleNotFound {
                        location: use_path.location,
                        module: segment.id,
                    })
                }
            })?;

        let scope = &symbols[symbols[module_id].scope_id].types;
        match use_path.target {
            Some(target) => {
                if let Some((key, &type_id)) = scope.get_key_value(&target.id.into()) {
                    if !key.visibility.is_public() || !symbols[type_id].visibility().is_public() {
                        return Err(Error::TypeNotPublic {
                            location: use_path.location,
                            ty: type_id,
                        });
                    }
                    Ok(vec![(key.clone(), type_id)])
                } else {
                    return Err(Error::UnresolvedType {
                        location: use_path.location,
                        resolved_type: target.id,
                    });
                }
            }
            None => Ok(scope
                .iter()
                .filter_map(|(k, &id)| {
                    (k.visibility.is_public() && symbols[id].visibility().is_public())
                        .then_some((k.clone(), id))
                })
                .collect()),
        }
    }

    fn resolve_member_type(
        &mut self,
        path: &PathNode,
        root: TypeId,
        target: IdentId,
        ctx: &mut TypeCheckerVisitorContext<F, C>,
    ) -> Result<TypeId> {
        let scope_id = ctx.symbols[root].scope_id();
        if let Some(&type_id) = ctx.symbols[scope_id].types.get(&target.into()) {
            if !ctx.symbols[type_id].visibility().is_public() {
                return Err(Error::TypeNotPublic {
                    location: path.location,
                    ty: type_id,
                });
            }
            return Ok(type_id);
        }

        let method_type_id = self.find_impl(root, target, ctx)?.1;
        if !ctx.symbols[method_type_id].visibility().is_public() {
            return Err(Error::MemberNotPublic {
                location: path.location,
                ty: root,
                field: target,
            });
        }
        return Ok(method_type_id);
    }

    fn resolve_module_type(
        &mut self,
        path: &PathNode,
        module: ModuleId,
        ty: IdentId,
        ctx: &mut TypeCheckerVisitorContext<F, C>,
    ) -> Result<TypeId> {
        let scope_id = ctx.symbols[module].scope_id;
        let type_id = ctx.symbols[scope_id]
            .types
            .get(&ty.into())
            .ok_or_else(|| Error::UnresolvedType {
                location: path.location,
                resolved_type: ty,
            })?
            .clone();
        if !ctx.symbols[type_id].visibility().is_public() {
            return Err(Error::TypeNotPublic {
                location: path.location,
                ty: type_id,
            });
        }
        return Ok(type_id);
    }
}
