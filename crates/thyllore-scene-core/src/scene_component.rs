use serde::de::DeserializeOwned;
use serde::Serialize;

/// A component whose persisted parameters are stored in a scene entity under `TYPE_KEY`;
/// `declare_scene_format!` implements it from the declaration table, so the field list,
/// the serde form and the key never drift apart.
pub trait SceneComponent: Serialize + DeserializeOwned + 'static {
    /// Stable key of the component inside a scene entity's component map.
    const TYPE_KEY: &'static str;
    /// Persisted field names in declaration order; equal to the keys of the serialized form.
    const PERSISTED_FIELDS: &'static [&'static str];
}
