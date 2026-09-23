use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, Error, Fields, Meta, Result};

#[proc_macro_derive(PyEffect, attributes(py_effect))]
pub fn derive_py_effect(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    expand_py_effect(&input)
        .unwrap_or_else(Error::into_compile_error)
        .into()
}

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
            component: #name,
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
            } else {
                return Err(meta.error("unknown scene attribute key"));
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
            let persist = parse_persist_attribute(attr)?;
            entries.push(expand_persisted_entry(ident, &field.ty, persist)?);
        }
    }

    Ok(entries)
}

fn parse_persist_attribute(attr: &syn::Attribute) -> Result<PersistAttributes> {
    let mut persist = PersistAttributes::default();

    attr.meta.require_list()?.parse_nested_meta(|meta| {
        if meta.path.is_ident("owner") {
            persist.owner = Some(meta.value()?.parse()?);
        } else if meta.path.is_ident("as") {
            persist.as_type = Some(meta.value()?.parse()?);
        } else if meta.path.is_ident("get") {
            persist.get = Some(meta.value()?.parse()?);
        } else if meta.path.is_ident("set") {
            persist.set = Some(meta.value()?.parse()?);
        } else if meta.path.is_ident("default") {
            persist.default = Some(meta.value()?.parse()?);
        } else if meta.path.is_ident("scalars") {
            persist.scalars = Some(meta.value()?.parse()?);
        } else if meta.path.is_ident("ui") {
            persist.ui = Some(parse_ui_attributes(&meta)?);
        } else {
            return Err(meta.error("unknown persist attribute key"));
        }

        Ok(())
    })?;

    Ok(persist)
}

fn expand_persisted_entry(
    ident: &syn::Ident,
    field_ty: &syn::Type,
    persist: PersistAttributes,
) -> Result<proc_macro2::TokenStream> {
    let owner = persist
        .owner
        .ok_or_else(|| Error::new_spanned(ident, "persist attribute requires `owner`"))?;

    let ty = match &persist.as_type {
        Some(as_type) => quote!(#as_type),
        None => quote!(#field_ty),
    };

    let accessors = match (&persist.get, &persist.set) {
        (Some(get), Some(set)) => quote!(get: #get, set: #set),
        (None, None) if persist.as_type.is_none() => {
            quote!(get: |e| e.#ident, set: |e, v| e.#ident = v)
        }
        _ => return Err(Error::new_spanned(
            ident,
            "persist `as` requires both `get` and `set`, and `get` / `set` must be given together",
        )),
    };

    let def = persist.default.map(|def| quote!(, default: #def));
    let scalars = persist.scalars.map(|channels| quote!(, scalars: #channels));
    let ui = persist.ui.map(|ui| expand_ui(ident, ui)).transpose()?;

    Ok(quote! {
        #ident: #ty = #owner {
            #accessors
            #def
            #scalars
            #ui
        }
    })
}

fn collect_runtime_entries(fields: &Fields) -> Result<Vec<proc_macro2::TokenStream>> {
    let mut entries = Vec::new();

    for field in fields.iter() {
        let Some(ident) = &field.ident else {
            continue;
        };

        for attr in field.attrs.iter().filter(|a| a.path().is_ident("runtime")) {
            let ty = &field.ty;
            let ui = parse_runtime_ui(attr)?
                .map(|ui| expand_ui(ident, ui))
                .transpose()?;

            entries.push(quote! {
                #ident: #ty {
                    get: |e| e.#ident,
                    set: |e, v| e.#ident = v
                    #ui
                }
            });
        }
    }

    Ok(entries)
}

fn parse_runtime_ui(attr: &syn::Attribute) -> Result<Option<UiAttributes>> {
    if let Meta::Path(_) = &attr.meta {
        return Ok(None);
    }

    let mut ui = None;
    attr.meta.require_list()?.parse_nested_meta(|meta| {
        if meta.path.is_ident("ui") {
            ui = Some(parse_ui_attributes(&meta)?);
            Ok(())
        } else {
            Err(meta.error("unknown runtime attribute key"))
        }
    })?;

    if let Some(kind) = ui.as_ref().and_then(|ui: &UiAttributes| ui.kind.as_ref()) {
        return Err(Error::new_spanned(
            kind,
            "runtime ui does not accept `kind`",
        ));
    }

    Ok(ui)
}

fn parse_ui_attributes(meta: &syn::meta::ParseNestedMeta) -> Result<UiAttributes> {
    let mut ui = UiAttributes::default();

    meta.parse_nested_meta(|ui_meta| {
        if ui_meta.path.is_ident("kind") {
            ui.kind = Some(ui_meta.value()?.parse()?);
        } else if ui_meta.path.is_ident("label") {
            ui.label = Some(ui_meta.value()?.parse()?);
        } else if ui_meta.path.is_ident("min") {
            ui.min = Some(ui_meta.value()?.parse()?);
        } else if ui_meta.path.is_ident("max") {
            ui.max = Some(ui_meta.value()?.parse()?);
        } else if ui_meta.path.is_ident("format") {
            ui.format = Some(ui_meta.value()?.parse()?);
        } else if ui_meta.path.is_ident("tooltip") {
            ui.tooltip = Some(ui_meta.value()?.parse()?);
        } else if ui_meta.path.is_ident("group") {
            ui.group = Some(ui_meta.value()?.parse()?);
        } else {
            return Err(ui_meta.error("unknown ui key"));
        }

        Ok(())
    })?;

    Ok(ui)
}

fn expand_ui(ident: &syn::Ident, ui: UiAttributes) -> Result<proc_macro2::TokenStream> {
    let min = ui
        .min
        .ok_or_else(|| Error::new_spanned(ident, "ui requires `min`"))?;
    let max = ui
        .max
        .ok_or_else(|| Error::new_spanned(ident, "ui requires `max`"))?;

    let kind = ui.kind.map(|kind| quote!(kind: #kind,));
    let label = ui.label.map(|label| quote!(label: #label,));
    let format = ui.format.map(|format| quote!(, format: #format));
    let tooltip = ui.tooltip.map(|tooltip| quote!(, tooltip: #tooltip));
    let group = ui.group.map(|group| quote!(, group: #group));

    Ok(quote! {
        , ui {
            #kind
            #label
            min: #min,
            max: #max
            #format
            #tooltip
            #group
        }
    })
}

#[derive(Default)]
struct PersistAttributes {
    owner: Option<syn::Ident>,
    as_type: Option<syn::Type>,
    get: Option<syn::Path>,
    set: Option<syn::Path>,
    default: Option<syn::Expr>,
    scalars: Option<syn::Ident>,
    ui: Option<UiAttributes>,
}

#[derive(Default)]
struct UiAttributes {
    kind: Option<syn::Ident>,
    label: Option<syn::LitStr>,
    min: Option<syn::Expr>,
    max: Option<syn::Expr>,
    format: Option<syn::LitStr>,
    tooltip: Option<syn::LitStr>,
    group: Option<syn::LitStr>,
}

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

fn expand_py_effect(input: &DeriveInput) -> Result<proc_macro2::TokenStream> {
    let Data::Struct(data) = &input.data else {
        return Err(Error::new_spanned(
            &input.ident,
            "PyEffect can only be derived for structs",
        ));
    };

    let name = &input.ident;
    let (impl_generics, type_generics, where_clause) = input.generics.split_for_impl();

    let py_effect = parse_py_effect_attributes(input)?;
    let presets = &py_effect.presets;
    let apply_preset = &py_effect.apply_preset;

    let Some(scene) = parse_scene_attributes(input)? else {
        return Err(Error::new_spanned(
            &input.ident,
            "PyEffect requires #[scene(...)] with ui, overwrite and tags on the struct",
        ));
    };
    let ui = &scene.ui;
    let overwrite = &scene.overwrite;
    let tags = &scene.tags;

    let time_field = find_field_by_name(input, &data.fields, "time")?;
    if !time_field
        .attrs
        .iter()
        .any(|a| a.path().is_ident("runtime"))
    {
        return Err(Error::new_spanned(
            time_field,
            "PyEffect requires the `time` field to be #[runtime]",
        ));
    }

    let position_set = find_persist_setter(find_field_by_name(input, &data.fields, "position")?)?;
    let rotation_set = find_persist_setter(find_field_by_name(input, &data.fields, "rotation")?)?;

    Ok(quote! {
        #[cfg(any(feature = "python", feature = "python-test"))]
        impl #impl_generics crate::pybindings::PyEffect for #name #type_generics #where_clause {
            const PRESET_NAMES: &'static [&'static str] = #presets;
            const UI_PARAMS: &'static [crate::UiParam] = #ui;

            fn apply_preset(&mut self, name: &str) -> bool {
                #apply_preset(self, name)
            }

            fn overwrite_persisted_fields(&mut self, source: &Self) {
                #overwrite(self, source);
            }

            fn set_placement(&mut self, time: f32, position: [f32; 3], rotation: [f32; 4]) {
                self.time = time;
                #position_set(self, position);
                #rotation_set(self, rotation);
            }

            fn parameter_owner_name(name: &str) -> &'static str {
                #tags
                    .iter()
                    .find(|(param_name, _)| *param_name == name)
                    .map_or("unknown", |(_, owner)| {
                        crate::pybindings::ParameterOwnerName::owner_name(*owner)
                    })
            }
        }
    })
}

fn parse_py_effect_attributes(input: &DeriveInput) -> Result<PyEffectAttributes> {
    let Some(attr) = input.attrs.iter().find(|a| a.path().is_ident("py_effect")) else {
        return Err(Error::new_spanned(
            &input.ident,
            "PyEffect requires #[py_effect(presets = ..., apply_preset = ...)] on the struct",
        ));
    };

    let mut presets: Option<syn::Path> = None;
    let mut apply_preset: Option<syn::Path> = None;

    attr.meta.require_list()?.parse_nested_meta(|meta| {
        if meta.path.is_ident("presets") {
            presets = Some(meta.value()?.parse()?);
        } else if meta.path.is_ident("apply_preset") {
            apply_preset = Some(meta.value()?.parse()?);
        } else {
            return Err(meta.error("unknown py_effect attribute key"));
        }

        Ok(())
    })?;

    Ok(PyEffectAttributes {
        presets: presets
            .ok_or_else(|| Error::new_spanned(attr, "py_effect attribute requires `presets`"))?,
        apply_preset: apply_preset.ok_or_else(|| {
            Error::new_spanned(attr, "py_effect attribute requires `apply_preset`")
        })?,
    })
}

fn find_field_by_name<'a>(
    input: &DeriveInput,
    fields: &'a Fields,
    name: &str,
) -> Result<&'a syn::Field> {
    fields
        .iter()
        .find(|field| field.ident.as_ref().is_some_and(|ident| ident == name))
        .ok_or_else(|| {
            Error::new_spanned(
                &input.ident,
                format!("PyEffect requires a field named `{name}`"),
            )
        })
}

fn find_persist_setter(field: &syn::Field) -> Result<syn::Path> {
    for attr in field.attrs.iter().filter(|a| a.path().is_ident("persist")) {
        if let Some(set) = parse_persist_attribute(attr)?.set {
            return Ok(set);
        }
    }

    Err(Error::new_spanned(
        field,
        "PyEffect requires #[persist(as = ..., get = ..., set = ...)] on this field",
    ))
}

struct PyEffectAttributes {
    presets: syn::Path,
    apply_preset: syn::Path,
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

    #[test]
    fn scene_persist_ui_all_fields() {
        let expanded = expand_scene(
            "#[scene(record = WindRecord, tag = WindTag, key = \"wind\", tags = WIND_TAGS, snapshot = WIND_SNAPSHOT, scalars = WIND_SCALARS, ui = WIND_UI, overwrite = WIND_OVERWRITE)]
            struct S {
                #[persist(owner = Frame, ui(kind = Color, label = \"Test\", min = 0.0, max = 1.0, format = \"{:.2}\", tooltip = \"tip\", group = \"grp\"))]
                pub value: f32,
            }",
        );
        assert!(
            expanded.contains("kind : Color , label : \"Test\" , min : 0.0 , max : 1.0 , format : \"{:.2}\" , tooltip : \"tip\" , group : \"grp\""),
            "{expanded}"
        );
    }

    #[test]
    fn scene_persist_kind_scalars() {
        let expanded = expand_scene(
            "#[scene(record = WindRecord, tag = WindTag, key = \"wind\", tags = WIND_TAGS, snapshot = WIND_SNAPSHOT, scalars = WIND_SCALARS, ui = WIND_UI, overwrite = WIND_OVERWRITE)]
            struct S {
                #[persist(owner = Frame, scalars = rgb, ui(kind = Color, min = 0.0, max = 1.0))]
                pub color: [f32; 3],
            }",
        );
        assert!(
            expanded.contains("scalars : rgb , ui { kind : Color , min : 0.0 , max : 1.0 }"),
            "{expanded}"
        );
    }

    #[test]
    fn scene_persist_default() {
        let expanded = expand_scene(
            "#[scene(record = WindRecord, tag = WindTag, key = \"wind\", tags = WIND_TAGS, snapshot = WIND_SNAPSHOT, scalars = WIND_SCALARS, ui = WIND_UI, overwrite = WIND_OVERWRITE)]
            struct S {
                #[persist(owner = Frame, default = 1.0)]
                pub value: f32,
            }",
        );
        assert!(
            expanded.contains("e . value = v , default : 1.0 }"),
            "{expanded}"
        );
    }

    #[test]
    fn scene_persist_as_get_set() {
        let expanded = expand_scene(
            "#[scene(record = WindRecord, tag = WindTag, key = \"wind\", tags = WIND_TAGS, snapshot = WIND_SNAPSHOT, scalars = WIND_SCALARS, ui = WIND_UI, overwrite = WIND_OVERWRITE)]
            struct S {
                #[persist(owner = Frame, as = [f32; 3], get = read_color, set = write_color)]
                pub color: Rgb,
            }",
        );
        assert!(
            expanded.contains("color : [f32 ; 3] = Frame { get : read_color , set : write_color }"),
            "{expanded}"
        );
    }

    #[test]
    fn scene_runtime_ui() {
        let expanded = expand_scene(
            "#[scene(record = WindRecord, tag = WindTag, key = \"wind\", tags = WIND_TAGS, snapshot = WIND_SNAPSHOT, scalars = WIND_SCALARS, ui = WIND_UI, overwrite = WIND_OVERWRITE)]
            struct S {
                #[runtime(ui(min = 0.0, max = 1.0, format = \"{:.2}\"))]
                pub time: f32,
            }",
        );
        assert!(
            expanded.contains("ui { min : 0.0 , max : 1.0 , format : \"{:.2}\" }"),
            "{expanded}"
        );
    }

    #[test]
    fn scene_unknown_key_error() {
        let input: DeriveInput = syn::parse_str(
            "#[scene(record = WindRecord, tag = WindTag, key = \"wind\", tags = WIND_TAGS, snapshot = WIND_SNAPSHOT, scalars = WIND_SCALARS, ui = WIND_UI, overwrite = WIND_OVERWRITE, unknown = true)]
            struct S {
                #[persist(owner = Frame, ui(min = 0.0, max = 1.0))]
                pub a: f32,
            }",
        )
        .expect("valid struct");
        let err = expand_scene_format(&input).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("unknown scene attribute key"),
            "expected unknown key error, got: {msg}"
        );
    }

    #[test]
    fn scene_owner_missing_error() {
        let input: DeriveInput = syn::parse_str(
            "#[scene(record = WindRecord, tag = WindTag, key = \"wind\", tags = WIND_TAGS, snapshot = WIND_SNAPSHOT, scalars = WIND_SCALARS, ui = WIND_UI, overwrite = WIND_OVERWRITE)]
            struct S {
                #[persist(ui(min = 0.0, max = 1.0))]
                pub a: f32,
            }",
        )
        .expect("valid struct");
        let err = expand_scene_format(&input).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("persist attribute requires `owner`"),
            "expected owner missing error, got: {msg}"
        );
    }

    const PY_EFFECT_SCENE: &str = "#[py_effect(presets = WIND_PRESETS, apply_preset = apply_wind)]
        #[scene(record = WindRecord, tag = WindTag, key = \"wind\", tags = WIND_TAGS, snapshot = WIND_SNAPSHOT, scalars = WIND_SCALARS, ui = WIND_UI, overwrite = WIND_OVERWRITE)]";

    #[test]
    fn py_effect_expands_impl() {
        let input: DeriveInput = syn::parse_str(&format!(
            "{PY_EFFECT_SCENE}
            struct S {{
                #[runtime]
                pub time: f32,
                #[persist(owner = Frame, as = [f32; 3], get = get_position, set = set_position)]
                pub position: Vector3<f32>,
                #[persist(owner = Frame, as = [f32; 4], get = get_rotation, set = set_rotation)]
                pub rotation: Quaternion<f32>,
            }}"
        ))
        .expect("valid struct");
        let expanded = expand_py_effect(&input).expect("expands").to_string();

        for expected in [
            "impl crate :: pybindings :: PyEffect for S",
            "const PRESET_NAMES : & 'static [& 'static str] = WIND_PRESETS ;",
            "const UI_PARAMS : & 'static [crate :: UiParam] = WIND_UI ;",
            "apply_wind (self , name)",
            "WIND_OVERWRITE (self , source) ;",
            "self . time = time ; set_position (self , position) ; set_rotation (self , rotation) ;",
            "WIND_TAGS . iter ()",
            "map_or (\"unknown\"",
            "crate :: pybindings :: ParameterOwnerName :: owner_name (* owner)",
        ] {
            assert!(expanded.contains(expected), "missing `{expected}` in {expanded}");
        }
    }

    #[test]
    fn py_effect_missing_time_error() {
        let input: DeriveInput = syn::parse_str(&format!(
            "{PY_EFFECT_SCENE}
            struct S {{
                #[persist(owner = Frame, as = [f32; 3], get = get_position, set = set_position)]
                pub position: Vector3<f32>,
                #[persist(owner = Frame, as = [f32; 4], get = get_rotation, set = set_rotation)]
                pub rotation: Quaternion<f32>,
            }}"
        ))
        .expect("valid struct");
        let msg = expand_py_effect(&input).unwrap_err().to_string();
        assert!(
            msg.contains("PyEffect requires a field named `time`"),
            "expected missing time error, got: {msg}"
        );
    }
}
