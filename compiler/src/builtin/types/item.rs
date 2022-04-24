use crate::globals::Globals;
use crate::builtin::types::classes::TypeBuilder;

#[derive(Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Item {
    pub id: u16,
    pub arbitrary: bool,
}

impl Item {

    pub fn new(id: u16) -> Self {
        Item {
            id,
            arbitrary: false,
        }
    }

    pub fn next_free(globals: &mut Globals) -> Self {
        (*globals).closed_items += 1;

        Item {
            id: (*globals).closed_items,
            arbitrary: true,
        }
    }
}

impl std::fmt::Debug for Item {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.arbitrary {
            true => f.write_str("?i"),
            false => f.write_str(&format!("{}i", self.id))
        }
    }
}

pub fn init(globals: &mut Globals) -> TypeBuilder<Item> {
    let ty = TypeBuilder::<Item>::name("item")
        .set_constructor(|id, arbitrary| Item { id, arbitrary });
    ty
}