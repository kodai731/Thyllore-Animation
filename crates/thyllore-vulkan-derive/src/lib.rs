use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, Error, Field, Fields, Index, Result};

#[proc_macro_derive(GpuResource, attributes(gpu_resource))]
pub fn derive_gpu_resource(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    expand_gpu_resource(&input)
        .unwrap_or_else(Error::into_compile_error)
        .into()
}

fn expand_gpu_resource(input: &DeriveInput) -> Result<proc_macro2::TokenStream> {
    let fields = collect_destroyed_fields(input)?;
    let name = &input.ident;
    let (impl_generics, type_generics, where_clause) = input.generics.split_for_impl();

    let destroy_calls = fields.iter().rev().map(|field| {
        quote! {
            ::thyllore_vulkan_core::resource::GpuResource::destroy_gpu(&mut self.#field, rrdevice);
        }
    });

    Ok(quote! {
        impl #impl_generics ::thyllore_vulkan_core::resource::GpuResource for #name #type_generics #where_clause {
            unsafe fn destroy_gpu(&mut self, rrdevice: &::thyllore_vulkan_core::core::device::RRDevice) {
                #(#destroy_calls)*
            }
        }
    })
}

fn collect_destroyed_fields(input: &DeriveInput) -> Result<Vec<proc_macro2::TokenStream>> {
    let Data::Struct(data) = &input.data else {
        return Err(Error::new_spanned(
            &input.ident,
            "GpuResource can only be derived for structs",
        ));
    };

    let fields: Vec<(usize, &Field)> = match &data.fields {
        Fields::Named(named) => named.named.iter().enumerate().collect(),
        Fields::Unnamed(unnamed) => unnamed.unnamed.iter().enumerate().collect(),
        Fields::Unit => Vec::new(),
    };

    let mut destroyed = Vec::new();
    for (position, field) in fields {
        if is_skipped(field)? {
            continue;
        }
        destroyed.push(match &field.ident {
            Some(ident) => quote!(#ident),
            None => {
                let index = Index::from(position);
                quote!(#index)
            }
        });
    }
    Ok(destroyed)
}

fn is_skipped(field: &Field) -> Result<bool> {
    let mut skipped = false;
    for attribute in field
        .attrs
        .iter()
        .filter(|a| a.path().is_ident("gpu_resource"))
    {
        attribute.parse_nested_meta(|meta| {
            if meta.path.is_ident("skip") {
                skipped = true;
                Ok(())
            } else {
                Err(meta.error("unknown gpu_resource attribute, expected `skip`"))
            }
        })?;
    }
    Ok(skipped)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn expand(source: &str) -> String {
        let input: DeriveInput = syn::parse_str(source).expect("valid struct");
        expand_gpu_resource(&input).expect("expands").to_string()
    }

    fn destroy_call_position(expanded: &str, field: &str) -> usize {
        let call = format!("destroy_gpu (& mut self . {field} , rrdevice)");
        expanded
            .find(&call)
            .unwrap_or_else(|| panic!("no destroy call for {field}"))
    }

    #[test]
    fn destroys_fields_in_reverse_declaration_order() {
        let expanded = expand("struct Owner { first: A, second: B, third: C }");

        let first = destroy_call_position(&expanded, "first");
        let second = destroy_call_position(&expanded, "second");
        let third = destroy_call_position(&expanded, "third");
        assert!(third < second && second < first, "{expanded}");
    }

    #[test]
    fn skips_fields_marked_with_gpu_resource_skip() {
        let expanded =
            expand("struct Owner { owned: A, #[gpu_resource(skip)] borrowed: B, last: C }");

        assert!(!expanded.contains("self . borrowed"), "{expanded}");
        destroy_call_position(&expanded, "owned");
        destroy_call_position(&expanded, "last");
    }

    #[test]
    fn supports_tuple_structs() {
        let expanded = expand("struct Owner(A, B);");

        assert!(destroy_call_position(&expanded, "1") < destroy_call_position(&expanded, "0"));
    }

    #[test]
    fn rejects_enums_and_unknown_attributes() {
        let input: DeriveInput = syn::parse_str("enum Owner { A }").expect("valid enum");
        assert!(expand_gpu_resource(&input).is_err());

        let input: DeriveInput =
            syn::parse_str("struct Owner { #[gpu_resource(other)] a: A }").expect("valid struct");
        assert!(expand_gpu_resource(&input).is_err());
    }
}
