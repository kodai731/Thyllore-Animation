use serde::{Deserialize, Serialize};
use thyllore_scene_core::SceneComponent;

pub use thyllore_effect_core::WindTornadoEffect;

crate::scene_owner!(WindTornadoEffect {
    icon: Wind,
    placement: |e| (e.position, e.rotation),
});

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AppliedWindPreset {
    pub name: String,
}

impl SceneComponent for AppliedWindPreset {
    const TYPE_KEY: &'static str = "wind_preset";
    const PERSISTED_FIELDS: &'static [&'static str] = &["name"];
}

crate::scene_attachment!(AppliedWindPreset);
