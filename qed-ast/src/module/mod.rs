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

#[derive(Clone, Debug)]
pub enum ModuleItemNode {
    ModuleDecl((IdentId, Visibility)),
    InlineModule(ModuleNode),
    ModuleUse(UsePath),
    Definition(DefinitionNode),
}
