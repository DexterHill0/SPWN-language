use shared::ImportType;

use crate::{Module, add_rust_builtins};

//** Add extra builtins here **//////////////////////////////////

add_rust_builtins!(
    math
);

////////////////////////////////////////////////////////////////

pub fn get_module_type(name: &ImportType) -> Module {
    match name {
        ImportType::Lib(n) => {
            //search for the library to see if it is a builtin
            match BUILTIN_NAMES.iter().find(|&b| match b {
                Module::RustBuiltin(mn) | Module::SpwnBuiltin(mn) => mn == n,
                _ => false	
            }) {
                //if it is a builtin, then leave it
                Some(b) => b.clone(),
                //otherwise assume it's a user added library
                None => Module::UserLibrary(n.clone()),
            }
        },

        ImportType::Script(n) => Module::Script(n.to_str().unwrap().to_string())
    }
}