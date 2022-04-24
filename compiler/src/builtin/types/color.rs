use crate::globals::Globals;
use crate::builtin::types::classes::TypeBuilder;

#[derive(Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Color {
    pub id: u16,
    pub arbitrary: bool,
}

impl Color {

    pub fn new(id: u16) -> Self {
        Color {
            id,
            arbitrary: false,
        }
    }

    pub fn next_free(globals: &mut Globals) -> Self {
        (*globals).closed_colors += 1;

        Color {
            id: (*globals).closed_colors,
            arbitrary: true,
        }
    }
}

impl std::fmt::Debug for Color {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.arbitrary {
            true => f.write_str("?c"),
            false => f.write_str(&format!("{}c", self.id))
        }
    }
}

pub fn init(globals: &mut Globals) -> TypeBuilder<Color> {
    let ty = TypeBuilder::<Color>::name("color")
        .set_constructor(|id, arbitrary| Color { id, arbitrary });
    ty
}