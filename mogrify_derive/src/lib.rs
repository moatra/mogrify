mod attrs;
mod derive;
mod fields;

use crate::derive::derive_inner;
use proc_macro::TokenStream;
use syn::parse_macro_input;

#[proc_macro_derive(Mogrify, attributes(mogrify))]
pub fn derive(input: TokenStream) -> TokenStream {
    let result = derive_inner(parse_macro_input!(input));
    match result {
        Ok(inner) => inner.into(),
        Err(err) => err.into_compile_error().into(),
    }
}
