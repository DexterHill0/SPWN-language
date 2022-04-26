use crate::globals::Globals;
use crate::builtin::types::classes::TypeBuilder;

#[derive(Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Group {
    pub id: u16,
    pub arbitrary: bool,
}

impl Group {

    pub fn new(id: u16) -> Self {
        Group {
            id,
            arbitrary: false,
        }
    }

    pub fn next_free(globals: &mut Globals) -> Self {
        (*globals).closed_groups += 1;

        Group {
            id: (*globals).closed_groups,
            arbitrary: true,
        }
    }

    pub fn test_fn() -> Group {
        Group::new(10)
    }
}

impl std::fmt::Debug for Group {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.arbitrary {
            true => f.write_str("?g"),
            false => f.write_str(&format!("{}g", self.id))
        }
    }
}

pub fn init() -> TypeBuilder<Group> {
    let ty = TypeBuilder::<Group>::name("group")
        .set_constructor(|id, arbitrary| Group { id, arbitrary })
        .add_static_method("test_fn", || Group::test_fn() );
    ty
}


// pub fn _display_(&self) -> String {
// 	format!("{}{}g", self.id, if self.arbitrary { ".?" } else { "" })
// }

// pub fn _as_<T>(&self, ty: T) -> Result<T, Error> 
// 	where T: TypeName
// {
// 	match ty {
// 		// Number => {
// 		// 	if self.arbitrary {
// 		// 		return Err("This group ID isn't known at this time, and therefore cannot be converted to a number!")
// 		// 	}
// 		// 	Ok(self.id)
// 		// }
// 		_ => Err(format!("Casting `{}` to `{}` is not implemented", Block::type_name, T::type_name))
// 	}
// }
    