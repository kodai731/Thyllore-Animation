use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// Base colour texture per material name, as a `/` separated path relative to the model's directory.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct MaterialTextureRemap {
    pub textures: BTreeMap<String, String>,
}
