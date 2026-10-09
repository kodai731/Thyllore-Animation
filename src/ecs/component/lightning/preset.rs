use serde::{Deserialize, Serialize};
use thyllore_scene_core::SceneComponent;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AppliedLightningPreset {
    pub name: String,
}

impl SceneComponent for AppliedLightningPreset {
    const TYPE_KEY: &'static str = "lightning_preset";
    const PERSISTED_FIELDS: &'static [&'static str] = &["name"];
}

crate::scene_attachment!(AppliedLightningPreset);
