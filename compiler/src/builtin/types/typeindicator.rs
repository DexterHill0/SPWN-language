use crate::builtin::types::classes::TypeBuilder;

#[derive(Clone, PartialEq, Hash)]
pub struct TypeIndicator (
	pub String,
);

impl TypeIndicator {
	pub fn new(name: String) -> Self {
        TypeIndicator(
            name
		)
    }
}

impl std::fmt::Debug for TypeIndicator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&format!("@{}", self.0))
    }
}

pub fn init() -> TypeBuilder<TypeIndicator> {
    let ty = TypeBuilder::<TypeIndicator>::name("type_indicator")
        .set_constructor(|name| TypeIndicator(name));
    ty
}