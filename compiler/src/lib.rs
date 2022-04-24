#[cfg(not(debug_assertions))]
#[macro_use(include_dir)]
extern crate include_dir;

pub mod builtins;
pub mod compiler;
pub mod compiler_types;
pub mod context;
pub mod globals;
pub mod leveldata;
pub mod parse_levelstring;
pub mod value;
pub mod value_storage;
pub mod builtin;
mod to_value;
mod from_value;

pub const STD_PATH: &str = "std";
