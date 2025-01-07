use std::fmt::Display;

use qed_common::{define_arena_id, FileId};

use crate::{AstVisitor, DefinitionNode, IdentId};

define_arena_id!(ModuleId);

impl ModuleId {
    pub const fn root() -> Self {
        Self(0)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ModuleKind {
    File { file_id: FileId, is_dir: bool },
}

#[derive(Debug, Clone, PartialEq)]
pub struct UsePath {
    pub kind: UseKind,
    pub segments: Vec<IdentId>,
    pub target: Option<IdentId>,
}

impl UsePath {
    pub fn accept_visitor<F: Clone, C, V: AstVisitor<F, C>>(&mut self, visitor: &mut V) {
        visitor.visit_use(self)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum UseKind {
    MODULE(IdentId),
    CRATE,
    SELF,
    SUPER,
}

impl From<IdentId> for UseKind {
    fn from(value: IdentId) -> Self {
        match value {
            IdentId::CRATE => UseKind::CRATE,
            IdentId::SELF => UseKind::SELF,
            IdentId::SUPER => UseKind::SUPER,
            v => UseKind::MODULE(v),
        }
    }
}

impl From<UseKind> for IdentId {
    fn from(value: UseKind) -> Self {
        match value {
            UseKind::MODULE(k) => k,
            UseKind::CRATE => IdentId::CRATE,
            UseKind::SELF => IdentId::SELF,
            UseKind::SUPER => IdentId::SUPER,
        }
    }
}

#[derive(Clone, Debug)]
pub struct RawModule {
    pub name: IdentId,
    pub parent_file_id: Option<FileId>,
    pub modules: Vec<IdentId>,
    pub uses: Vec<UsePath>,
    pub definitions: Vec<DefinitionNode>,
    pub is_std: bool,
    pub is_self_std: bool,
    pub is_self_prelude: bool,
}

impl RawModule {
    pub fn accept_visitor<F: Clone, C, V: AstVisitor<F, C>>(&self, visitor: &mut V) {
        visitor.visit_module(self)
    }
}
