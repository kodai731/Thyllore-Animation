use gltf::json;

/// Sets a root extension, replacing an earlier value of the same name, and lists it in `extensionsUsed` once.
pub(crate) fn merge_extension(root: &mut json::Root, name: &str, value: serde_json::Value) {
    root.extensions
        .get_or_insert_with(Default::default)
        .others
        .insert(name.to_string(), value);

    if !root.extensions_used.iter().any(|used| used == name) {
        root.extensions_used.push(name.to_string());
    }
}
