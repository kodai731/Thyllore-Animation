use serde::{Deserialize, Serialize};
use thyllore_scene_core::SceneComponent;

pub use thyllore_effect_core::{WaterTemporalAccum, WaterTorusEffect};

crate::scene_owner!(WaterTorusEffect {
    icon: crate::ecs::component::EntityIcon::Effect('W'),
    placement: |e| (e.position, e.rotation),
});

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AppliedWaterPreset {
    pub name: String,
}

impl SceneComponent for AppliedWaterPreset {
    const TYPE_KEY: &'static str = "water_preset";
    const PERSISTED_FIELDS: &'static [&'static str] = &["name"];
}

crate::scene_attachment!(AppliedWaterPreset);
