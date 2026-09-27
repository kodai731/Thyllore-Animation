use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{Data, DeriveInput, Error, Expr, Field, Fields, Ident, LitStr, Meta, Path, Result, Type};

pub fn expand_scene_fields(input: &DeriveInput) -> Result<TokenStream> {
    let Data::Struct(data) = &input.data else {
        return Err(Error::new_spanned(
            &input.ident,
            "SceneFields can only be derived for structs",
        ));
    };

    let attrs = parse_struct_attributes(input)?;
    let fields = parse_fields(&data.fields, &attrs)?;
    let name = &input.ident;

    let record = expand_record(name, &fields);
    let scene_fields = expand_scene_fields_impl(name, &attrs.tag, &fields);
    let component = match &attrs.scene {
        Some(scene) => expand_component_tables(name, &attrs.tag, scene, &fields),
        None => quote!(),
    };

    Ok(quote! {
        const _: () = {
            #record
            #scene_fields
        };
        #component
    })
}

pub struct StructAttributes {
    pub tag: Path,
    pub owner: Option<Ident>,
    pub group: Option<LitStr>,
    pub scene: Option<SceneNames>,
}

pub struct SceneNames {
    pub key: LitStr,
    pub tags: Ident,
    pub snapshot: Ident,
    pub scalars: Ident,
    pub ui: Ident,
    pub overwrite: Ident,
}

pub fn parse_struct_attributes(input: &DeriveInput) -> Result<StructAttributes> {
    let scene_attr = input.attrs.iter().find(|a| a.path().is_ident("scene"));
    let params_attr = input.attrs.iter().find(|a| a.path().is_ident("params"));
    let (attr, is_scene) = match (scene_attr, params_attr) {
        (Some(_), Some(_)) => {
            return Err(Error::new_spanned(
                &input.ident,
                "use either #[scene(...)] (top-level effect) or #[params(...)] (nested struct)",
            ))
        }
        (Some(attr), None) => (attr, true),
        (None, Some(attr)) => (attr, false),
        (None, None) => {
            return Err(Error::new_spanned(
                &input.ident,
                "SceneFields requires #[scene(...)] or #[params(...)] on the struct",
            ))
        }
    };

    let mut tag: Option<Path> = None;
    let mut owner: Option<Ident> = None;
    let mut group: Option<LitStr> = None;
    let mut key: Option<LitStr> = None;
    let mut tags: Option<Ident> = None;
    let mut snapshot: Option<Ident> = None;
    let mut scalars: Option<Ident> = None;
    let mut ui: Option<Ident> = None;
    let mut overwrite: Option<Ident> = None;

    attr.meta.require_list()?.parse_nested_meta(|meta| {
        if meta.path.is_ident("tag") {
            tag = Some(meta.value()?.parse()?);
        } else if meta.path.is_ident("owner") {
            owner = Some(meta.value()?.parse()?);
        } else if meta.path.is_ident("group") {
            group = Some(meta.value()?.parse()?);
        } else if is_scene && meta.path.is_ident("key") {
            key = Some(meta.value()?.parse()?);
        } else if is_scene && meta.path.is_ident("tags") {
            tags = Some(meta.value()?.parse()?);
        } else if is_scene && meta.path.is_ident("snapshot") {
            snapshot = Some(meta.value()?.parse()?);
        } else if is_scene && meta.path.is_ident("scalars") {
            scalars = Some(meta.value()?.parse()?);
        } else if is_scene && meta.path.is_ident("ui") {
            ui = Some(meta.value()?.parse()?);
        } else if is_scene && meta.path.is_ident("overwrite") {
            overwrite = Some(meta.value()?.parse()?);
        } else {
            return Err(meta.error("unknown struct attribute key"));
        }
        Ok(())
    })?;

    let required = |value: Option<Ident>, name: &str| {
        value.ok_or_else(|| Error::new_spanned(attr, format!("scene attribute requires `{name}`")))
    };
    let scene = if is_scene {
        Some(SceneNames {
            key: key.ok_or_else(|| Error::new_spanned(attr, "scene attribute requires `key`"))?,
            tags: required(tags, "tags")?,
            snapshot: required(snapshot, "snapshot")?,
            scalars: required(scalars, "scalars")?,
            ui: required(ui, "ui")?,
            overwrite: required(overwrite, "overwrite")?,
        })
    } else {
        None
    };

    Ok(StructAttributes {
        tag: tag.ok_or_else(|| Error::new_spanned(attr, "struct attribute requires `tag`"))?,
        owner,
        group,
        scene,
    })
}

pub enum FieldSpec {
    Persist(ParamField),
    Runtime(ParamField),
    Nested(NestedField),
}

pub struct ParamField {
    pub ident: Ident,
    pub field_ty: Type,
    pub value_ty: Type,
    pub conversion: Conversion,
    pub owner: Option<Ident>,
    pub ui: Option<UiAttributes>,
    pub kind: ValueKind,
    pub component_scalars: bool,
    pub doc: String,
}

pub enum Conversion {
    Plain,
    From,
    With(Path),
}

pub struct NestedField {
    pub ident: Ident,
    pub ty: Type,
    pub persisted: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ValueKind {
    F32,
    U32,
    Bool,
    Array(usize),
    Other,
}

#[derive(Default)]
pub struct UiAttributes {
    pub primary: bool,
    pub kind: Option<Ident>,
    pub label: Option<LitStr>,
    pub min: Option<Expr>,
    pub max: Option<Expr>,
    pub format: Option<LitStr>,
    pub group: Option<LitStr>,
}

pub fn parse_fields(fields: &Fields, attrs: &StructAttributes) -> Result<Vec<FieldSpec>> {
    let mut specs = Vec::new();

    for field in fields.iter() {
        if field.ident.is_none() {
            return Err(Error::new_spanned(
                field,
                "SceneFields requires named fields",
            ));
        }

        let persist = field.attrs.iter().find(|a| a.path().is_ident("persist"));
        let runtime = field.attrs.iter().find(|a| a.path().is_ident("runtime"));
        let nested = field.attrs.iter().find(|a| a.path().is_ident("nested"));
        let marked = [persist.is_some(), runtime.is_some(), nested.is_some()]
            .iter()
            .filter(|m| **m)
            .count();
        if marked > 1 {
            return Err(Error::new_spanned(
                field,
                "a field carries at most one of #[persist], #[runtime], #[nested]",
            ));
        }

        if let Some(attr) = persist {
            let mut param = parse_param_field(field, attr, attrs)?;
            if param.owner.is_none() {
                return Err(Error::new_spanned(
                    field,
                    "persist requires `owner` on the field or a struct-level default",
                ));
            }
            param.doc = doc_comment(field);
            specs.push(FieldSpec::Persist(param));
        } else if let Some(attr) = runtime {
            let mut param = parse_param_field(field, attr, attrs)?;
            if !matches!(param.conversion, Conversion::Plain) {
                return Err(Error::new_spanned(
                    field,
                    "runtime fields take no `as` / `with`",
                ));
            }
            param.doc = doc_comment(field);
            specs.push(FieldSpec::Runtime(param));
        } else if let Some(attr) = nested {
            specs.push(FieldSpec::Nested(NestedField {
                ident: field.ident.clone().expect("named"),
                ty: field.ty.clone(),
                persisted: parse_nested_persisted(attr)?,
            }));
        }
    }

    Ok(specs)
}

fn parse_nested_persisted(attr: &syn::Attribute) -> Result<bool> {
    if matches!(attr.meta, Meta::Path(_)) {
        return Ok(true);
    }
    let mut persisted = true;
    attr.meta.require_list()?.parse_nested_meta(|meta| {
        if meta.path.is_ident("runtime") {
            persisted = false;
            return Ok(());
        }
        Err(meta.error("nested accepts only `runtime`"))
    })?;
    Ok(persisted)
}

pub fn parse_param_field(
    field: &Field,
    attr: &syn::Attribute,
    attrs: &StructAttributes,
) -> Result<ParamField> {
    let mut owner = attrs.owner.clone();
    let mut as_type: Option<Type> = None;
    let mut with: Option<Path> = None;
    let mut ui: Option<UiAttributes> = None;
    let mut scalars = false;

    if !matches!(attr.meta, Meta::Path(_)) {
        attr.meta.require_list()?.parse_nested_meta(|meta| {
            if meta.path.is_ident("owner") {
                owner = Some(meta.value()?.parse()?);
            } else if meta.path.is_ident("as") {
                as_type = Some(meta.value()?.parse()?);
            } else if meta.path.is_ident("with") {
                with = Some(meta.value()?.parse()?);
            } else if meta.path.is_ident("ui") {
                ui = Some(parse_ui_attributes(&meta)?);
            } else if meta.path.is_ident("scalars") {
                scalars = true;
            } else {
                return Err(meta.error("unknown field attribute key"));
            }
            Ok(())
        })?;
    }

    let conversion = match (&as_type, with) {
        (None, Some(_)) => return Err(Error::new_spanned(attr, "`with` requires `as`")),
        (None, None) => Conversion::Plain,
        (Some(_), None) => Conversion::From,
        (Some(_), Some(with)) => Conversion::With(with),
    };
    let value_ty = as_type.unwrap_or_else(|| field.ty.clone());
    let kind = value_kind(&value_ty);

    if let Some(ui) = &ui {
        if kind == ValueKind::Other {
            return Err(Error::new_spanned(
                field,
                "ui is only available for f32 / u32 / bool / [f32; N] values",
            ));
        }
        if ui.min.is_none() || ui.max.is_none() {
            return Err(Error::new_spanned(field, "ui requires `min` and `max`"));
        }
        let color_kind = ui
            .kind
            .as_ref()
            .is_some_and(|k| k == "Color" || k == "Absorption");
        if color_kind && kind != ValueKind::Array(3) {
            return Err(Error::new_spanned(
                field,
                "Color / Absorption need a [f32; 3] value",
            ));
        }
    }
    if scalars && !matches!(kind, ValueKind::Array(_)) {
        return Err(Error::new_spanned(
            field,
            "`scalars` applies to [f32; N] values",
        ));
    }
    let component_scalars = scalars || (ui.is_some() && matches!(kind, ValueKind::Array(_)));

    Ok(ParamField {
        ident: field.ident.clone().expect("named"),
        field_ty: field.ty.clone(),
        value_ty,
        conversion,
        owner,
        ui: ui.map(|mut ui| {
            if ui.group.is_none() {
                ui.group = attrs.group.clone();
            }
            ui
        }),
        kind,
        component_scalars,
        doc: String::new(),
    })
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
        } else if ui_meta.path.is_ident("group") {
            ui.group = Some(ui_meta.value()?.parse()?);
        } else if ui_meta.path.is_ident("primary") {
            ui.primary = true;
        } else {
            return Err(ui_meta.error("unknown ui key"));
        }
        Ok(())
    })?;

    Ok(ui)
}

fn value_kind(ty: &Type) -> ValueKind {
    match ty {
        Type::Path(path) if path.qself.is_none() && path.path.segments.len() == 1 => {
            match path.path.segments[0].ident.to_string().as_str() {
                "f32" => ValueKind::F32,
                "u32" => ValueKind::U32,
                "bool" => ValueKind::Bool,
                _ => ValueKind::Other,
            }
        }
        Type::Array(array) => {
            let is_f32 = matches!(&*array.elem, Type::Path(p) if p.path.is_ident("f32"));
            let len = match &array.len {
                Expr::Lit(syn::ExprLit {
                    lit: syn::Lit::Int(int),
                    ..
                }) => int.base10_parse::<usize>().ok(),
                _ => None,
            };
            match (is_f32, len) {
                (true, Some(len)) => ValueKind::Array(len),
                _ => ValueKind::Other,
            }
        }
        _ => ValueKind::Other,
    }
}

fn doc_comment(field: &Field) -> String {
    let lines: Vec<String> = field
        .attrs
        .iter()
        .filter(|a| a.path().is_ident("doc"))
        .filter_map(|a| match &a.meta {
            Meta::NameValue(nv) => match &nv.value {
                Expr::Lit(syn::ExprLit {
                    lit: syn::Lit::Str(text),
                    ..
                }) => Some(text.value().trim().to_string()),
                _ => None,
            },
            _ => None,
        })
        .collect();
    lines.join(" ")
}

pub fn read_value(param: &ParamField, component: &TokenStream) -> TokenStream {
    let field = &param.ident;
    let value_ty = &param.value_ty;
    match &param.conversion {
        Conversion::Plain => quote!(#component.#field.clone()),
        Conversion::From => {
            quote!(::core::convert::Into::<#value_ty>::into(#component.#field.clone()))
        }
        Conversion::With(with) => quote!(#with::get(&#component.#field)),
    }
}

pub fn write_value(
    param: &ParamField,
    component: &TokenStream,
    value: &TokenStream,
) -> TokenStream {
    let field = &param.ident;
    let field_ty = &param.field_ty;
    match &param.conversion {
        Conversion::Plain => quote!(#component.#field = #value;),
        Conversion::From => quote!(#component.#field = <#field_ty>::from(#value);),
        Conversion::With(with) => quote!(#with::set(&mut #component.#field, #value);),
    }
}

fn expand_record(name: &Ident, fields: &[FieldSpec]) -> TokenStream {
    let mut record_fields = Vec::new();
    let mut captures = Vec::new();
    let mut applies = Vec::new();
    let component = quote!(component);

    for spec in fields {
        match spec {
            FieldSpec::Persist(param) => {
                let field = &param.ident;
                let value_ty = &param.value_ty;
                let read = read_value(param, &component);
                let write = write_value(param, &component, &quote!(self.#field));
                record_fields.push(quote!(#field: #value_ty));
                captures.push(quote!(#field: #read));
                applies.push(write);
            }
            FieldSpec::Nested(nested) if !nested.persisted => {}
            FieldSpec::Nested(nested) => {
                let field = &nested.ident;
                let ty = &nested.ty;
                record_fields.push(quote!(#field: #ty));
                captures.push(quote!(#field: #component.#field.clone()));
                applies.push(quote! {
                    ::thyllore_scene_core::SceneFields::overwrite_persisted(
                        &mut #component.#field,
                        &self.#field,
                    );
                });
            }
            FieldSpec::Runtime(_) => {}
        }
    }

    quote! {
        #[derive(::serde::Serialize, ::serde::Deserialize)]
        #[serde(default)]
        struct Record {
            #(#record_fields,)*
        }

        impl Default for Record {
            fn default() -> Self {
                Record::capture(&<#name as Default>::default())
            }
        }

        impl Record {
            #[allow(unused_variables)]
            fn capture(component: &#name) -> Self {
                Record { #(#captures,)* }
            }

            #[allow(unused_variables)]
            fn apply(self, component: &mut #name) {
                #(#applies)*
            }
        }

        impl ::serde::Serialize for #name {
            fn serialize<S: ::serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                Record::capture(self).serialize(serializer)
            }
        }

        impl<'de> ::serde::Deserialize<'de> for #name {
            fn deserialize<D: ::serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                let record = Record::deserialize(deserializer)?;
                let mut component = <#name as Default>::default();
                record.apply(&mut component);
                Ok(component)
            }
        }
    }
}

fn marker_ident(field: &Ident) -> Ident {
    let camel: String = field
        .to_string()
        .split('_')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().chain(chars).collect::<String>(),
                None => String::new(),
            }
        })
        .collect();
    format_ident!("{camel}FieldPath")
}

fn expand_scene_fields_impl(name: &Ident, tag: &Path, fields: &[FieldSpec]) -> TokenStream {
    let mut markers = Vec::new();
    let mut scalars = Vec::new();
    let mut uis = Vec::new();
    let mut ownerships = Vec::new();
    let mut paths = Vec::new();
    let mut snapshots = Vec::new();

    for spec in fields {
        match spec {
            FieldSpec::Persist(param) => {
                let field_name = param.ident.to_string();
                let read = read_value(param, &quote!(self));
                let owner = param.owner.as_ref().expect("checked at parse");
                ownerships.push(quote! {
                    out.push((::thyllore_scene_core::intern_name(prefix, #field_name), <#tag>::#owner));
                });
                paths.push(quote! {
                    out.push((
                        ::thyllore_scene_core::intern_name(prefix, #field_name),
                        ::thyllore_scene_core::intern_name(path_prefix, #field_name),
                    ));
                });
                snapshots.push(quote! {
                    out.push(::thyllore_scene_core::SnapshotValues::snapshot_values(&#read));
                });
                scalars.extend(expand_scalars(param));
                uis.extend(expand_ui(param, true));
            }
            FieldSpec::Runtime(param) => {
                scalars.extend(expand_scalars(param));
                uis.extend(expand_ui(param, false));
            }
            FieldSpec::Nested(nested) => {
                let field = &nested.ident;
                let field_name = field.to_string();
                let ty = &nested.ty;
                let marker = marker_ident(field);
                markers.push(quote! {
                    struct #marker;
                    impl ::thyllore_scene_core::FieldPath<#name> for #marker {
                        type Field = #ty;
                        fn get(root: &#name) -> &#ty {
                            &root.#field
                        }
                        fn get_mut(root: &mut #name) -> &mut #ty {
                            &mut root.#field
                        }
                    }
                });
                scalars.push(quote! {
                    <#ty as ::thyllore_scene_core::SceneFields>::collect_scalars::<
                        R,
                        ::thyllore_scene_core::Then<P, #marker>,
                    >(&::thyllore_scene_core::nested_prefix(prefix, #field_name, "_"), out);
                });
                uis.push(quote! {
                    <#ty as ::thyllore_scene_core::SceneFields>::collect_ui(
                        &::thyllore_scene_core::nested_prefix(prefix, #field_name, "_"),
                        &::thyllore_scene_core::nested_prefix(path_prefix, #field_name, "."),
                        out,
                    );
                });
                if nested.persisted {
                    ownerships.push(quote! {
                        <#ty as ::thyllore_scene_core::SceneFields>::collect_ownership(
                            &::thyllore_scene_core::nested_prefix(prefix, #field_name, "_"),
                            out,
                        );
                    });
                    paths.push(quote! {
                        <#ty as ::thyllore_scene_core::SceneFields>::collect_paths(
                            &::thyllore_scene_core::nested_prefix(prefix, #field_name, "_"),
                            &::thyllore_scene_core::nested_prefix(path_prefix, #field_name, "."),
                            out,
                        );
                    });
                    snapshots.push(quote! {
                        ::thyllore_scene_core::SceneFields::collect_snapshot(&self.#field, out);
                    });
                }
            }
        }
    }

    quote! {
        #(#markers)*

        impl ::thyllore_scene_core::SceneFields for #name {
            type Tag = #tag;

            fn overwrite_persisted(&mut self, loaded: &Self) {
                Record::capture(loaded).apply(self);
            }

            #[allow(unused_variables)]
            fn collect_scalars<R: 'static, P: ::thyllore_scene_core::FieldPath<R, Field = Self>>(
                prefix: &str,
                out: &mut Vec<::thyllore_scene_core::ScalarParam<R>>,
            ) {
                #(#scalars)*
            }

            #[allow(unused_variables)]
            fn collect_ui(
                prefix: &str,
                path_prefix: &str,
                out: &mut Vec<::thyllore_scene_core::UiParam>,
            ) {
                #(#uis)*
            }

            #[allow(unused_variables)]
            fn collect_ownership(prefix: &str, out: &mut Vec<(&'static str, Self::Tag)>) {
                #(#ownerships)*
            }

            #[allow(unused_variables)]
            fn collect_paths(
                prefix: &str,
                path_prefix: &str,
                out: &mut Vec<(&'static str, &'static str)>,
            ) {
                #(#paths)*
            }

            #[allow(unused_variables)]
            fn collect_snapshot(&self, out: &mut Vec<Vec<f32>>) {
                #(#snapshots)*
            }
        }
    }
}

fn expand_scalars(param: &ParamField) -> Vec<TokenStream> {
    let field_name = param.ident.to_string();
    let component = quote!(component);
    let read = read_value(param, &component);
    let write_stored = write_value(param, &component, &quote!(stored));

    let push = |suffix: &str, get_body: TokenStream, set_body: TokenStream| {
        let name = format!("{field_name}{suffix}");
        quote! {
            out.push(::thyllore_scene_core::ScalarParam {
                name: ::thyllore_scene_core::intern_name(prefix, #name),
                get: |root: &R| {
                    let component: &Self = P::get(root);
                    #get_body
                },
                set: |root: &mut R, value: f32| {
                    let component: &mut Self = P::get_mut(root);
                    #set_body
                },
            });
        }
    };

    match param.kind {
        ValueKind::F32 => vec![push(
            "",
            quote!(#read),
            quote!(let stored = value; #write_stored),
        )],
        ValueKind::U32 => vec![push(
            "",
            quote!(#read as f32),
            quote!(let stored = value.round() as u32; #write_stored),
        )],
        ValueKind::Bool => vec![push(
            "",
            quote!(u8::from(#read) as f32),
            quote!(let stored = value != 0.0; #write_stored),
        )],
        ValueKind::Array(_) if !param.component_scalars => Vec::new(),
        ValueKind::Array(len) => component_suffixes(param, len)
            .into_iter()
            .enumerate()
            .map(|(index, suffix)| {
                push(
                    suffix,
                    quote!(#read[#index]),
                    quote! {
                        let mut stored = #read;
                        stored[#index] = value;
                        #write_stored
                    },
                )
            })
            .collect(),
        ValueKind::Other => Vec::new(),
    }
}

fn component_suffixes(param: &ParamField, len: usize) -> Vec<&'static str> {
    let color = param
        .ui
        .as_ref()
        .and_then(|ui| ui.kind.as_ref())
        .is_some_and(|k| k == "Color" || k == "Absorption");
    let suffixes: &[&'static str] = if color {
        &["_r", "_g", "_b"]
    } else {
        &["_x", "_y", "_z", "_w"]
    };
    suffixes.iter().copied().take(len).collect()
}

fn expand_ui(param: &ParamField, persisted: bool) -> Option<TokenStream> {
    let ui = param.ui.as_ref()?;
    let field_name = param.ident.to_string();
    let min = ui.min.as_ref().expect("checked at parse");
    let max = ui.max.as_ref().expect("checked at parse");
    let kind = ui
        .kind
        .clone()
        .unwrap_or_else(|| Ident::new("Scalar", proc_macro2::Span::call_site()));
    let label = match &ui.label {
        Some(label) => quote!(Some(#label)),
        None => quote!(None),
    };
    let format = ui
        .format
        .as_ref()
        .map_or_else(|| "%.2f".to_string(), LitStr::value);
    let group = ui.group.as_ref().map_or_else(String::new, LitStr::value);
    let tooltip = &param.doc;
    let primary = ui.primary;

    Some(quote! {
        out.push(::thyllore_scene_core::UiParam {
            name: ::thyllore_scene_core::intern_name(prefix, #field_name),
            path: ::thyllore_scene_core::intern_name(path_prefix, #field_name),
            group: #group,
            label: #label,
            kind: ::thyllore_scene_core::UiKind::#kind,
            min: #min,
            max: #max,
            format: #format,
            tooltip: #tooltip,
            persisted: #persisted,
            primary: #primary,
        });
    })
}

fn expand_component_tables(
    name: &Ident,
    tag: &Path,
    scene: &SceneNames,
    fields: &[FieldSpec],
) -> TokenStream {
    let SceneNames {
        key,
        tags,
        snapshot,
        scalars,
        ui,
        overwrite,
    } = scene;
    let persisted_names: Vec<String> = fields
        .iter()
        .filter_map(|spec| match spec {
            FieldSpec::Persist(param) => Some(param.ident.to_string()),
            FieldSpec::Nested(nested) if nested.persisted => Some(nested.ident.to_string()),
            FieldSpec::Nested(_) | FieldSpec::Runtime(_) => None,
        })
        .collect();

    quote! {
        pub static #tags: ::std::sync::LazyLock<Vec<(&'static str, #tag)>> =
            ::std::sync::LazyLock::new(|| {
                let mut out = Vec::new();
                <#name as ::thyllore_scene_core::SceneFields>::collect_ownership("", &mut out);
                out
            });

        pub static #scalars: ::std::sync::LazyLock<Vec<::thyllore_scene_core::ScalarParam<#name>>> =
            ::std::sync::LazyLock::new(|| {
                let mut out = Vec::new();
                <#name as ::thyllore_scene_core::SceneFields>::collect_scalars::<
                    #name,
                    ::thyllore_scene_core::RootPath<#name>,
                >("", &mut out);
                out
            });

        pub static #ui: ::std::sync::LazyLock<Vec<::thyllore_scene_core::UiParam>> =
            ::std::sync::LazyLock::new(|| {
                let mut out = Vec::new();
                <#name as ::thyllore_scene_core::SceneFields>::collect_ui("", "", &mut out);
                out
            });

        /// Bit-exact snapshot of every persisted parameter; diffing two yields what a writer touched.
        pub fn #snapshot(component: &#name) -> Vec<(&'static str, Vec<f32>)> {
            let mut values = Vec::new();
            ::thyllore_scene_core::SceneFields::collect_snapshot(component, &mut values);
            #tags.iter().map(|(name, _)| *name).zip(values).collect()
        }

        /// Writes every persisted parameter of `loaded` onto `target`, keeping runtime state.
        pub fn #overwrite(target: &mut #name, loaded: &#name) {
            ::thyllore_scene_core::SceneFields::overwrite_persisted(target, loaded);
        }

        impl ::thyllore_scene_core::SceneComponent for #name {
            const TYPE_KEY: &'static str = #key;
            const PERSISTED_FIELDS: &'static [&'static str] = &[#(#persisted_names),*];

            fn overwrite_persisted_fields(&mut self, loaded: &Self) {
                ::thyllore_scene_core::SceneFields::overwrite_persisted(self, loaded);
            }
        }

        impl ::thyllore_scene_core::SceneTables for #name {
            fn scalar_params() -> &'static [::thyllore_scene_core::ScalarParam<Self>] {
                &#scalars
            }

            fn ui_params() -> &'static [::thyllore_scene_core::UiParam] {
                &#ui
            }

            fn ownership() -> &'static [(&'static str, #tag)] {
                &#tags
            }

            fn parameter_paths() -> &'static [(&'static str, &'static str)] {
                static PATHS: ::std::sync::LazyLock<Vec<(&'static str, &'static str)>> =
                    ::std::sync::LazyLock::new(|| {
                        let mut out = Vec::new();
                        <#name as ::thyllore_scene_core::SceneFields>::collect_paths("", "", &mut out);
                        out
                    });
                &PATHS
            }

            fn snapshot(&self) -> Vec<(&'static str, Vec<f32>)> {
                #snapshot(self)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn expand(source: &str) -> String {
        let input: DeriveInput = syn::parse_str(source).expect("valid struct");
        expand_scene_fields(&input).expect("expands").to_string()
    }

    fn expand_err(source: &str) -> String {
        let input: DeriveInput = syn::parse_str(source).expect("valid struct");
        expand_scene_fields(&input)
            .expect_err("rejects")
            .to_string()
    }

    const TOP: &str = "#[scene(key = \"k\", tag = Owner, tags = TAGS, snapshot = snap, scalars = SCALARS, ui = UI, overwrite = overwrite, owner = Frame)]";

    #[test]
    fn plain_persisted_field_is_captured_applied_and_registered() {
        let expanded = expand(&format!(
            "{TOP} struct S {{ #[persist(ui(min = 0.0, max = 1.0))] pub height: f32 }}"
        ));
        assert!(
            expanded.contains("height : component . height . clone ()"),
            "{expanded}"
        );
        assert!(
            expanded.contains("component . height = self . height"),
            "{expanded}"
        );
        assert!(
            expanded.contains("intern_name (prefix , \"height\")"),
            "{expanded}"
        );
        assert!(expanded.contains("< Owner > :: Frame"), "{expanded}");
        assert!(
            expanded.contains("const TYPE_KEY : & 'static str = \"k\""),
            "{expanded}"
        );
    }

    #[test]
    fn nested_field_delegates_with_joined_prefixes() {
        let expanded = expand(&format!("{TOP} struct S {{ #[nested] pub noise: Noise }}"));
        assert!(
            expanded.contains("nested_prefix (prefix , \"noise\" , \"_\")"),
            "{expanded}"
        );
        assert!(
            expanded.contains("nested_prefix (path_prefix , \"noise\" , \".\")"),
            "{expanded}"
        );
        assert!(
            expanded.contains("Then < P , NoiseFieldPath >"),
            "{expanded}"
        );
        assert!(
            expanded.contains("PERSISTED_FIELDS : & 'static [& 'static str] = & [\"noise\"]"),
            "{expanded}"
        );
    }

    #[test]
    fn runtime_nested_field_stays_out_of_the_record() {
        let expanded = expand(&format!(
            "{TOP} struct S {{ #[nested(runtime)] pub emitter: Emitter }}"
        ));
        assert!(
            expanded.contains("nested_prefix (prefix , \"emitter\" , \"_\")"),
            "{expanded}"
        );
        assert!(
            !expanded.contains("collect_ownership (& :: thyllore_scene_core :: nested_prefix"),
            "{expanded}"
        );
        assert!(
            expanded.contains("PERSISTED_FIELDS : & 'static [& 'static str] = & []"),
            "{expanded}"
        );
    }

    #[test]
    fn doc_comment_becomes_the_tooltip() {
        let expanded = expand(&format!(
            "{TOP} struct S {{ /// Half width\n /// of the wall\n #[persist(ui(min = 0.0, max = 1.0))] pub w: f32 }}"
        ));
        assert!(
            expanded.contains("tooltip : \"Half width of the wall\""),
            "{expanded}"
        );
    }

    #[test]
    fn color_array_registers_rgb_aliases() {
        let expanded = expand(&format!(
            "{TOP} struct S {{ #[persist(ui(kind = Color, min = 0.0, max = 1.0))] pub albedo: [f32; 3] }}"
        ));
        for suffix in ["albedo_r", "albedo_g", "albedo_b"] {
            assert!(expanded.contains(&format!("\"{suffix}\"")), "{expanded}");
        }
    }

    #[test]
    fn array_registers_xyz_aliases_only_when_asked() {
        let silent = expand(&format!(
            "{TOP} struct S {{ #[persist] pub dir: [f32; 2] }}"
        ));
        assert!(!silent.contains("\"dir_x\""), "{silent}");

        let expanded = expand(&format!(
            "{TOP} struct S {{ #[persist(scalars)] pub dir: [f32; 2] }}"
        ));
        assert!(expanded.contains("\"dir_x\""), "{expanded}");
        assert!(expanded.contains("\"dir_y\""), "{expanded}");
        assert!(!expanded.contains("\"dir_z\""), "{expanded}");
    }

    #[test]
    fn as_converts_through_from_and_with_calls_the_module() {
        let expanded = expand(&format!(
            "{TOP} struct S {{ #[persist(as = [f32; 3])] pub p: V3, #[persist(as = [f32; 4], with = wxyz)] pub r: Q }}"
        ));
        assert!(
            expanded.contains("Into :: < [f32 ; 3] > :: into (component . p . clone ())"),
            "{expanded}"
        );
        assert!(
            expanded.contains("component . p = < V3 > :: from (self . p)"),
            "{expanded}"
        );
        assert!(
            expanded.contains("wxyz :: get (& component . r)"),
            "{expanded}"
        );
        assert!(
            expanded.contains("wxyz :: set (& mut component . r , self . r)"),
            "{expanded}"
        );
    }

    #[test]
    fn nested_struct_uses_params_and_emits_no_tables() {
        let expanded = expand("#[params(tag = Owner, owner = Style, group = \"noise\")] struct N { #[persist(ui(min = 0.0, max = 3.0))] pub amplitude: f32 }");
        assert!(expanded.contains("group : \"noise\""), "{expanded}");
        assert!(expanded.contains("< Owner > :: Style"), "{expanded}");
        assert!(!expanded.contains("LazyLock"), "{expanded}");
    }

    #[test]
    fn runtime_field_is_ui_only() {
        let expanded = expand(&format!(
            "{TOP} struct S {{ #[runtime(ui(min = 0.0, max = 4.0))] pub time_scale: f32 }}"
        ));
        assert!(expanded.contains("persisted : false"), "{expanded}");
        assert!(
            expanded.contains("PERSISTED_FIELDS : & 'static [& 'static str] = & []"),
            "{expanded}"
        );
    }

    #[test]
    fn rejects_persist_without_owner() {
        let message = expand_err("#[params(tag = Owner)] struct N { #[persist] pub a: f32 }");
        assert!(message.contains("owner"), "{message}");
    }

    #[test]
    fn rejects_unknown_keys() {
        assert!(expand_err(&format!(
            "{TOP} struct S {{ #[persist(path = \"a.b\")] pub a: f32 }}"
        ))
        .contains("unknown field attribute key"));
        assert!(expand_err(&format!(
            "{TOP} struct S {{ #[persist(ui(min = 0.0, max = 1.0, tooltip = \"x\"))] pub a: f32 }}"
        ))
        .contains("unknown ui key"));
        assert!(
            expand_err("#[params(tag = Owner, key = \"k\")] struct N { }")
                .contains("unknown struct attribute key")
        );
    }

    #[test]
    fn rejects_ui_on_non_scalar_values() {
        let message = expand_err(&format!(
            "{TOP} struct S {{ #[persist(ui(min = 0.0, max = 1.0))] pub s: Source }}"
        ));
        assert!(message.contains("ui is only available"), "{message}");
    }

    #[test]
    fn rejects_color_kind_on_non_rgb_arrays() {
        let message = expand_err(&format!("{TOP} struct S {{ #[persist(ui(kind = Color, min = 0.0, max = 1.0))] pub c: [f32; 2] }}"));
        assert!(message.contains("[f32; 3]"), "{message}");
    }

    #[test]
    fn rejects_both_scene_and_params() {
        let message = expand_err("#[scene(key = \"k\")] #[params(tag = T)] struct S { }");
        assert!(message.contains("either"), "{message}");
    }

    #[test]
    fn rejects_enum() {
        let input: DeriveInput = syn::parse_str("enum E { A }").expect("valid enum");
        assert!(expand_scene_fields(&input).is_err());
    }
}
