use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, Error, Fields, Meta, Result};

#[proc_macro_derive(UboPack, attributes(ubo))]
pub fn derive_ubo_pack(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    expand_ubo_pack(&input)
        .unwrap_or_else(Error::into_compile_error)
        .into()
}

fn expand_ubo_pack(input: &DeriveInput) -> Result<proc_macro2::TokenStream> {
    let Data::Struct(data) = &input.data else {
        return Err(Error::new_spanned(
            &input.ident,
            "UboPack can only be derived for structs",
        ));
    };

    let name = &input.ident;
    let (impl_generics, type_generics, where_clause) = input.generics.split_for_impl();
    let target = parse_target(input)?;
    let assignments = collect_assignments(&data.fields)?;

    Ok(quote! {
        impl #impl_generics ::thyllore_effect_core::gpu_pack::UboPack<#target>
            for #name #type_generics #where_clause
        {
            fn pack(&self, ubo: &mut #target) {
                #(#assignments)*
            }
        }
    })
}

fn parse_target(input: &DeriveInput) -> Result<syn::Path> {
    for attr in input.attrs.iter().filter(|a| a.path().is_ident("ubo")) {
        let Meta::List(list) = &attr.meta else {
            continue;
        };

        let meta: Meta = syn::parse2(list.tokens.clone())?;
        if let Meta::NameValue(name_value) = meta {
            if name_value.path.is_ident("target") {
                if let syn::Expr::Path(path) = &name_value.value {
                    return Ok(path.path.clone());
                }
            }
        }
    }

    Err(Error::new_spanned(
        &input.ident,
        "UboPack requires #[ubo(target = TypeName)] on the struct",
    ))
}

fn collect_assignments(fields: &Fields) -> Result<Vec<proc_macro2::TokenStream>> {
    let mut assignments = Vec::new();

    for (position, field) in fields.iter().enumerate() {
        let field_ref = match &field.ident {
            Some(ident) => quote!(self.#ident),
            None => {
                let index = syn::Index::from(position);
                quote!(self.#index)
            }
        };

        for attr in field.attrs.iter().filter(|a| a.path().is_ident("ubo")) {
            let Meta::List(list) = &attr.meta else {
                continue;
            };

            let slot: syn::LitStr = syn::parse2(list.tokens.clone())?;
            assignments.push(generate_assignment(&slot, &field_ref)?);
        }
    }

    Ok(assignments)
}

fn generate_assignment(
    slot: &syn::LitStr,
    field_ref: &proc_macro2::TokenStream,
) -> Result<proc_macro2::TokenStream> {
    let spec = slot.value();
    let Some((slot_name, component)) = spec.split_once('.') else {
        return Err(Error::new_spanned(
            slot,
            format!("ubo attribute must be \"slot.component\" (e.g. \"shape.x\"), got \"{spec}\""),
        ));
    };

    let slot_ident = syn::Ident::new(slot_name, slot.span());

    if component == "xyz" {
        return Ok(quote! {
            ubo.#slot_ident[0] = #field_ref[0];
            ubo.#slot_ident[1] = #field_ref[1];
            ubo.#slot_ident[2] = #field_ref[2];
        });
    }

    let index: usize = match component {
        "x" => 0,
        "y" => 1,
        "z" => 2,
        "w" => 3,
        _ => {
            return Err(Error::new_spanned(
                slot,
                format!("unknown component \"{component}\", expected x/y/z/w or xyz"),
            ))
        }
    };

    Ok(quote! {
        ubo.#slot_ident[#index] = #field_ref;
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn expand(source: &str) -> String {
        let input: DeriveInput = syn::parse_str(source).expect("valid struct");
        expand_ubo_pack(&input).expect("expands").to_string()
    }

    #[test]
    fn packs_single_field_to_component() {
        let expanded = expand(
            "#[ubo(target = WindUBO)]
            struct S {
                #[ubo(\"shape.x\")]
                pub height: f32,
            }",
        );
        assert!(
            expanded.contains("ubo . shape [0usize] = self . height"),
            "{expanded}"
        );
    }

    #[test]
    fn packs_multiple_fields() {
        let expanded = expand(
            "#[ubo(target = WindUBO)]
            struct S {
                #[ubo(\"shape.x\")]
                pub a: f32,
                #[ubo(\"shape.y\")]
                pub b: f32,
            }",
        );
        assert!(
            expanded.contains("ubo . shape [0usize] = self . a"),
            "{expanded}"
        );
        assert!(
            expanded.contains("ubo . shape [1usize] = self . b"),
            "{expanded}"
        );
    }

    #[test]
    fn packs_xyz_array_field() {
        let expanded = expand(
            "#[ubo(target = WindUBO)]
            struct S {
                #[ubo(\"albedo.xyz\")]
                pub albedo: [f32; 3],
            }",
        );
        assert!(
            expanded.contains("ubo . albedo [0] = self . albedo [0]"),
            "{expanded}"
        );
        assert!(
            expanded.contains("ubo . albedo [1] = self . albedo [1]"),
            "{expanded}"
        );
        assert!(
            expanded.contains("ubo . albedo [2] = self . albedo [2]"),
            "{expanded}"
        );
    }

    #[test]
    fn skips_fields_without_ubo_attribute() {
        let expanded = expand(
            "#[ubo(target = WindUBO)]
            struct S {
                #[ubo(\"shape.x\")]
                pub a: f32,
                pub b: f32,
            }",
        );
        assert!(!expanded.contains("self . b"), "{expanded}");
    }

    #[test]
    fn rejects_missing_target() {
        let input: DeriveInput = syn::parse_str(
            "struct S {
                #[ubo(\"shape.x\")]
                pub a: f32,
            }",
        )
        .expect("valid struct");
        assert!(expand_ubo_pack(&input).is_err());
    }

    #[test]
    fn rejects_unknown_component() {
        let input: DeriveInput = syn::parse_str(
            "#[ubo(target = WindUBO)]
            struct S {
                #[ubo(\"shape.q\")]
                pub a: f32,
            }",
        )
        .expect("valid struct");
        assert!(expand_ubo_pack(&input).is_err());
    }

    #[test]
    fn rejects_enum() {
        let input: DeriveInput = syn::parse_str("enum E { A }").expect("valid enum");
        assert!(expand_ubo_pack(&input).is_err());
    }
}
