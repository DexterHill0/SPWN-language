pub mod classes;
pub mod methods;

use std::collections::HashMap;
use std::sync::{Arc};

use lazy_static::lazy_static;

lazy_static! {
    /// Map of classes that have been globally registered
    ///
    /// These will be used as a fallback, and cached on the host when an unknown instance is seen
    pub static ref DEFAULT_TYPES: Arc<HashMap<std::any::TypeId, super::types::classes::Type>> = Default::default();
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
                let ty = $namespace::init(globals).finish();

                DEFAULT_TYPES
                    .insert(ty.type_id, ty);

                globals.n_type_ids.insert(ty.name.clone(), ty.type_id.clone());
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
}