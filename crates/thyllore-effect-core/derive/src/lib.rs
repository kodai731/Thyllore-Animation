mod ubo;

use proc_macro::TokenStream;
use syn::{parse_macro_input, DeriveInput, Error};

#[proc_macro_derive(UboPack, attributes(ubo))]
pub fn derive_ubo_pack(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    ubo::expand_ubo_pack(&input)
        .unwrap_or_else(Error::into_compile_error)
        .into()
}
