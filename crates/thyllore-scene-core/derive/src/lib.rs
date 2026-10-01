mod scene;

use proc_macro::TokenStream;
use syn::{parse_macro_input, DeriveInput, Error};

#[proc_macro_derive(SceneFields, attributes(scene, params, persist, runtime, nested))]
pub fn derive_scene_fields(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    scene::expand_scene_fields(&input)
        .unwrap_or_else(Error::into_compile_error)
        .into()
}
