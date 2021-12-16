pub mod libraries;
pub mod builtin;

use std::collections::HashMap;
use include_dir::{Dir, include_dir};

#[macro_use]
extern crate lazy_static;


pub const STANDARD_LIBS: Dir = include_dir!("./std");

#[derive(Clone, Debug)]
pub enum Module {
    RustBuiltin(String),
    SpwnBuiltin(String),
    UserLibrary(String),
    Script(String),
}

//recursive macro to count the number of builtins, intialize a constant array with that count
//and fill it it with the names of the builtins
#[macro_export]
macro_rules! add_rust_builtins {

    //once the macro has finished recursing, it creates the array and populates it with names
    ( $count:expr; $($list:ident),* ) => {
        use crate::STANDARD_LIBS;

        lazy_static! {
            pub static ref BUILTIN_NAMES: [Module; $count + STANDARD_LIBS.files.len()] = [
                [$(
                    Module::RustBuiltin($list::__get_lib_info("name")),
                )+],
                
                STANDARD_LIBS.files().to_owned()
                    .iter()
                    .map(|f| Module::SpwnBuiltin(f.path().file_name().unwrap().to_str().unwrap().to_string()))
                    .collect::<Vec<_>>()
                    .try_into()
                    .unwrap()
            ]
            .concat()
            .try_into()
            .unwrap();
        }
    };

    //adds the import and recursively calls the macro adding 1 to count each time
    ( $count:expr; $($const:ident),*; $name:ident $($rest:ident),* ) => {
        use crate::builtin::$name;

        add_rust_builtins!($count + 1; $($const),* $($rest),*);
    };

    //starts the recursive macro
    ( $($list:ident),* ) => {
        add_rust_builtins!(0; $($list),*; $($list),*);
    };
}

#[macro_export] 
macro_rules! setup_lib {
    (   
        info: 
            name=$libname:literal
            $($ikey:ident=$ivalue:literal)*,
        $(opts: 
            $($okey:ident=$ovalue:literal)*
        )?
    ) => {
        lazy_static! {
            static ref __LIB_INFO: Vec<(&'static str, &'static str)> = {
                Vec::from([
                    $(
                        (
                            stringify!($ikey),
                            $ivalue
                        ),
                    )*
                    $($(
                        (
                            stringify!($ikey),
                            $ivalue
                        ),
                    )?)*
                ])
            };
        }

        pub fn __get_lib_info(key: &str) -> String {
            let el = __LIB_INFO.iter().find(|&e| e.0 == key);

            match el {
                Some((_, v)) => v.to_string(),
                None => String::new(),
            }
        }
        
        pub struct Globals {}
    }
}


// pub enum SpwnTypes {
// 	Type,
// 	Number,
// 	String,
// }

// #[macro_export]
// macro_rules! export {
// 	(SpwnTypes::$type:ident, struct $($tt:tt)*) => {
// 		#[Export(SpwnTypes::$type)]
// 		pub struct $($tt)*
// 	};
    
// 	(SpwnTypes::$type:ident, $($tt:tt)*) => {
// 		trait X { $($tt)* }
// 		impl X for Globals {}
// 	}
// }
