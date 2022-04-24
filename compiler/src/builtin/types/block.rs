use crate::globals::Globals;
use crate::builtin::types::classes::TypeBuilder;

#[derive(Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Block {
    pub id: u16,
    pub arbitrary: bool,
}

impl Block {

    pub fn new(id: u16) -> Self {
        Block {
            id,
            arbitrary: false,
        }
    }

    pub fn next_free(globals: &mut Globals) -> Self {
        (*globals).closed_groups += 1;

        Block {
            id: (*globals).closed_groups,
            arbitrary: true,
        }
    }
}

impl std::fmt::Debug for Block {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.arbitrary {
            true => f.write_str("?b"),
            false => f.write_str(&format!("{}b", self.id))
        }
    }
}

pub fn init(globals: &mut Globals) -> TypeBuilder<Block> {
    let ty = TypeBuilder::<Block>::name("block")
        .set_constructor(|id, arbitrary| Block { id, arbitrary });
    ty
}