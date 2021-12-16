use proc_macro::TokenStream;
use syn::{parse_macro_input, DeriveInput};

#[allow(non_snake_case)]
#[proc_macro_attribute]
pub fn Export(attr: TokenStream, tokens: TokenStream) -> TokenStream {
    let DeriveInput { ident, .. } = parse_macro_input!(tokens);

    TokenStream::new()
}