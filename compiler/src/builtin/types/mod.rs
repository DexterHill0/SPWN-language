pub mod classes;
pub mod methods;

use std::collections::HashMap;
use std::sync::Mutex;

use errors::compiler_info::CodeArea;
use super::types::classes::HashId;

use lazy_static::lazy_static;

lazy_static! {
    /// Map of classes that have been globally registered
    ///
    /// These will be used as a fallback, and cached on the host when an unknown instance is seen
    pub static ref DEFAULT_TYPES: Mutex<HashMap<HashId, super::types::classes::Type>> = Default::default();
}

macro_rules! register_type {
    ($(
        $namespace:ident::$strt:ident
    )*) => {
        $(
            pub mod $namespace;
            pub use $namespace::$strt;
        )*

        pub fn initialise(globals: &mut crate::globals::Globals) {
            $(
                let ty = $namespace::init().finish();

                DEFAULT_TYPES
					.lock()
					.unwrap()
                    .insert(ty.type_id.clone(), ty.clone());

                globals.type_ids.insert(ty.name.clone(), (ty.type_id.clone(), CodeArea::new()));
                globals.types.insert(ty.type_id, ty);
            )*
            // something
        }
    }
}

register_type! {
    group::Group
    color::Color
    item::Item
    block::Block
	typeindicator::TypeIndicator
}