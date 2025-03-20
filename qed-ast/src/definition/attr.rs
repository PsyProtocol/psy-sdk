use crate::{IdentId, Location};

#[derive(Clone, Debug, PartialEq)]
pub struct AttrNode {
    pub name: IdentId,
    pub properties: Vec<IdentId>,
    pub location: Location,
}

impl AttrNode {
    pub fn is_derive(&self) -> bool {
        self.name == IdentId::DERIVE
    }

    pub fn is_test(&self) -> bool {
        self.name == IdentId::TEST
    }
}
