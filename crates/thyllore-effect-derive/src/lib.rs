use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, Error, Fields, Meta, Result};

#[proc_macro_derive(SceneFormat, attributes(scene, persist, runtime))]
pub fn derive_scene_format(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    expand_scene_format(&input)
        .unwrap_or_else(Error::into_compile_error)
        .into()
}

fn expand_scene_format(input: &DeriveInput) -> Result<proc_macro2::TokenStream> {
    let Data::Struct(data) = &input.data else {
        return Err(Error::new_spanned(
            &input.ident,
            "SceneFormat can only be derived for structs",
        ));
    };

    let attrs = parse_scene_attributes(input)?;
    let Some(attrs) = attrs else {
        return Err(Error::new_spanned(
            &input.ident,
            "SceneFormat requires #[scene(record = ..., tag = ..., key = \"...\", tags = ...)] on the struct",
        ));
    };

    let name = &input.ident;
    let (_impl_generics, type_generics, where_clause) = input.generics.split_for_impl();

    let record = &attrs.record;
    let tag = &attrs.tag;
    let key: syn::LitStr = attrs.key;
    let tags = &attrs.tags;
    let snapshot = &attrs.snapshot;
    let scalars = &attrs.scalars;
    let ui = &attrs.ui;
    let overwrite = &attrs.overwrite;

    let persisted_entries = collect_persisted_entries(&data.fields)?;
    let runtime_entries = collect_runtime_entries(&data.fields)?;

    Ok(quote! {
        ::thyllore_scene_core::declare_scene_format!(
            component: #name #type_generics #where_clause,
            record: #record,
            tag: #tag,
            items {
                key: #key,
                tags: #tags,
                snapshot: #snapshot,
                scalars: #scalars,
                ui: #ui,
                overwrite: #overwrite,
            },
            persisted {
                #(#persisted_entries),*
            },
            runtime {
                #(#runtime_entries),*
            },
        );
    })
}

fn parse_scene_attributes(input: &DeriveInput) -> Result<Option<SceneAttributes>> {
    for attr in input.attrs.iter().filter(|a| a.path().is_ident("scene")) {
        let Meta::List(list) = &attr.meta else {
            continue;
        };

        let mut record: Option<syn::Path> = None;
        let mut tag: Option<syn::Path> = None;
        let mut key: Option<syn::LitStr> = None;
        let mut tags: Option<syn::Path> = None;
        let mut snapshot: Option<syn::Path> = None;
        let mut scalars: Option<syn::Path> = None;
        let mut ui: Option<syn::Path> = None;
        let mut overwrite: Option<syn::Path> = None;

        list.parse_nested_meta(|meta| {
            if meta.path.is_ident("record") {
                record = Some(meta.value()?.parse()?);
            } else if meta.path.is_ident("tag") {
                tag = Some(meta.value()?.parse()?);
            } else if meta.path.is_ident("key") {
                key = Some(meta.value()?.parse()?);
            } else if meta.path.is_ident("tags") {
                tags = Some(meta.value()?.parse()?);
            } else if meta.path.is_ident("snapshot") {
                snapshot = Some(meta.value()?.parse()?);
            } else if meta.path.is_ident("scalars") {
                scalars = Some(meta.value()?.parse()?);
            } else if meta.path.is_ident("ui") {
                ui = Some(meta.value()?.parse()?);
            } else if meta.path.is_ident("overwrite") {
                overwrite = Some(meta.value()?.parse()?);
            }

            Ok(())
        })?;

        return Ok(Some(SceneAttributes {
            record: record.ok_or_else(|| {
                Error::new_spanned(&input.ident, "scene attribute requires `record`")
            })?,
            tag: tag.ok_or_else(|| {
                Error::new_spanned(&input.ident, "scene attribute requires `tag`")
            })?,
            key: key.ok_or_else(|| {
                Error::new_spanned(&input.ident, "scene attribute requires `key`")
            })?,
            tags: tags.ok_or_else(|| {
                Error::new_spanned(&input.ident, "scene attribute requires `tags`")
            })?,
            snapshot: snapshot.ok_or_else(|| {
                Error::new_spanned(&input.ident, "scene attribute requires `snapshot`")
            })?,
            scalars: scalars.ok_or_else(|| {
                Error::new_spanned(&input.ident, "scene attribute requires `scalars`")
            })?,
            ui: ui
                .ok_or_else(|| Error::new_spanned(&input.ident, "scene attribute requires `ui`"))?,
            overwrite: overwrite.ok_or_else(|| {
                Error::new_spanned(&input.ident, "scene attribute requires `overwrite`")
            })?,
        }));
    }

    Ok(None)
}

fn collect_persisted_entries(fields: &Fields) -> Result<Vec<proc_macro2::TokenStream>> {
    let mut entries = Vec::new();

    for field in fields.iter() {
        let Some(ident) = &field.ident else {
            continue;
        };

        for attr in field.attrs.iter().filter(|a| a.path().is_ident("persist")) {
            let Meta::List(list) = &attr.meta else {
                continue;
            };

            let mut owner: Option<syn::Path> = None;
            list.parse_nested_meta(|meta| {
                if meta.path.is_ident("owner") {
                    owner = Some(meta.value()?.parse()?);
                }
                Ok(())
            })?;

            let owner = owner.unwrap_or_else(|| syn::parse_quote!(Frame));
            let field_name = ident.clone();
            let ty = &field.ty;

            entries.push(quote! {
                #field_name: #ty = #owner {
                    get: |e| e.#field_name,
                    set: |e, v| e.#field_name = v
                }
            });
        }
    }

    Ok(entries)
}

fn collect_runtime_entries(fields: &Fields) -> Result<Vec<proc_macro2::TokenStream>> {
    let mut entries = Vec::new();

    for field in fields.iter() {
        let Some(ident) = &field.ident else {
            continue;
        };

        if !field.attrs.iter().any(|a| a.path().is_ident("runtime")) {
            continue;
        }

        let field_name = ident.clone();
        let ty = &field.ty;

        entries.push(quote! {
            #field_name: #ty {
                get: |e| e.#field_name,
                set: |e, v| e.#field_name = v
            }
        });
    }

    Ok(entries)
}

#[derive(Clone)]
struct SceneAttributes {
    record: syn::Path,
    tag: syn::Path,
    key: syn::LitStr,
    tags: syn::Path,
    snapshot: syn::Path,
    scalars: syn::Path,
    ui: syn::Path,
    overwrite: syn::Path,
}

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

    fn expand_scene(source: &str) -> String {
        let input: DeriveInput = syn::parse_str(source).expect("valid struct");
        expand_scene_format(&input).expect("expands").to_string()
    }

    #[test]
    fn scene_persist_field() {
        let expanded = expand_scene(
            "#[scene(record = WindRecord, tag = WindTag, key = \"wind\", tags = WIND_TAGS, snapshot = WIND_SNAPSHOT, scalars = WIND_SCALARS, ui = WIND_UI, overwrite = WIND_OVERWRITE)]
            struct S {
                #[persist(owner = Frame)]
                pub intensity: f32,
            }",
        );
        assert!(
            expanded.contains("intensity : f32 = Frame { get : | e | e . intensity , set : | e , v | e . intensity = v }"),
            "{expanded}"
        );
    }

    #[test]
    fn scene_runtime_field() {
        let expanded = expand_scene(
            "#[scene(record = WindRecord, tag = WindTag, key = \"wind\", tags = WIND_TAGS, snapshot = WIND_SNAPSHOT, scalars = WIND_SCALARS, ui = WIND_UI, overwrite = WIND_OVERWRITE)]
            struct S {
                #[runtime]
                pub time: f32,
            }",
        );
        assert!(
            expanded.contains("time : f32 { get : | e | e . time , set : | e , v | e . time = v }"),
            "{expanded}"
        );
    }

    #[test]
    fn scene_missing_attribute_error() {
        let input: DeriveInput = syn::parse_str(
            "#[scene(record = WindRecord, tag = WindTag, key = \"wind\", tags = WIND_TAGS, scalars = WIND_SCALARS, ui = WIND_UI, overwrite = WIND_OVERWRITE)]
            struct S {
                pub a: f32,
            }",
        )
        .expect("valid struct");
        let err = expand_scene_format(&input).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("scene attribute requires `snapshot`"),
            "expected snapshot error, got: {msg}"
        );
    }
}
