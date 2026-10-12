mod gpu_resource;

use proc_macro::TokenStream;
use syn::{parse_macro_input, DeriveInput, Error};

// rustc requires proc-macro entry points at the crate root; the expansion lives in gpu_resource.rs.
#[proc_macro_derive(GpuResource, attributes(gpu_resource))]
pub fn derive_gpu_resource(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    gpu_resource::expand_gpu_resource(&input)
        .unwrap_or_else(Error::into_compile_error)
        .into()
}
