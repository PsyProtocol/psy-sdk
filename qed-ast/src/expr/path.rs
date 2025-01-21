use crate::{
    AstVisitor, {ExprId, IdentId},
};
use std::fmt::Display;
#[derive(Clone, Debug, PartialEq)]
pub enum PathType {
    Basic,
    Nested,
}
//tyree 2
#[derive(Clone, Debug, PartialEq)]
pub struct PathNode {
    pub path_type: PathType,
    pub root: Option<IdentId>,
    pub segments: Vec<IdentId>,
    pub target: IdentId,
}
impl PathNode {
    pub fn new(
        root: Option<IdentId>,
        target: IdentId,
        segments: Vec<IdentId>,
        path_type: PathType,
    ) -> Self {
        Self {
            root,
            segments,
            target,
            path_type,
        }
    }
}
// impl PathNode {
//     pub fn accept_visitor<F: Clone, C, V: AstVisitor<F, C>>(
//         &self,
//         visitor: &mut V,
//     ) -> V::ExprResult {
//         visitor.visit_path(self)
//     }
// }

impl Display for PathNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "PathNode: ")?;
        match &self.root {
            Some(root) => write!(f, "IdentId({})", root),
            None => write!(f, "NO_ROOT"),
        }?;
        for seg in &self.segments {
            write!(f, "::{}", seg)?;
        }
        writeln!(f, "::{}", self.target)?;
        write!(f, "PathType: {:?}", self.path_type)
    }
}
