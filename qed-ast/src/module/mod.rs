use std::fmt::Display;

use qed_common::{define_arena_id, FileId};

use crate::{AstVisitor, DefId, DefinitionNode, IdentId, Visibility};

define_arena_id!(ModuleId);

impl ModuleId {
    pub const fn root() -> Self {
        Self(0)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ModuleKind {
    File { file_id: FileId },
}

#[derive(Debug, Clone, PartialEq)]
pub struct UsePath {
    pub visibility: Visibility,
    pub kind: IdentId,
    pub segments: Vec<IdentId>,
    pub target: Option<IdentId>,
}

#[derive(Clone, Debug)]
pub struct ModuleNode {
    pub name: IdentId,
    pub file_id: FileId,
    pub modules: Vec<(IdentId, Visibility)>,
    pub inline_modules: Vec<ModuleNode>,
    pub uses: Vec<UsePath>,
    pub definitions: Vec<DefId>,
    pub visibility: Visibility,

    pub is_std: bool,
    pub is_self_std: bool,
    pub is_self_prelude: bool,
    pub is_self_primitive: bool,
}

impl ModuleNode {
    pub fn new(
        name: IdentId,
        file_id: FileId,
        visibility: Visibility,
        module_items: Vec<ModuleItemNode>,
        is_std: bool,
        is_self_std: bool,
        is_self_prelude: bool,
        is_self_primitive: bool,
    ) -> Self {
        let mut inline_modules = vec![];
        let mut modules = vec![];
        let mut uses = vec![];
        let mut definitions = vec![];
        for item in module_items.into_iter() {
            match item {
                ModuleItemNode::InlineModule(m) => inline_modules.push(m),
                ModuleItemNode::ModuleDecl(m) => modules.push(m),
                ModuleItemNode::ModuleUse(m) => uses.push(m),
                ModuleItemNode::Definition(d) => definitions.push(d),
            }
        }
        let mut module = Self {
            name,
            file_id,
            modules: {
                if !is_std {
                    modules.insert(0, (IdentId::STD, Visibility::Public));
                }
                modules
            },
            inline_modules,
            uses: {
                if !is_std {
                    uses.insert(
                        0,
                        UsePath {
                            visibility: Visibility::Public,
                            kind: IdentId::STD,
                            segments: vec![IdentId::PRELUDE],
                            target: None,
                        },
                    );
                }
                uses
            },
            definitions,
            visibility,
            is_std,
            is_self_std,
            is_self_prelude,
            is_self_primitive,
        };
        module
    }
}

#[derive(Clone, Debug)]
pub enum ModuleItemNode {
    ModuleDecl((IdentId, Visibility)),
    InlineModule(ModuleNode),
    ModuleUse(UsePath),
    Definition(DefId),
}
