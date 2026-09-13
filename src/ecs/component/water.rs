use serde::{Deserialize, Serialize};
use thyllore_scene_core::SceneComponent;

pub use thyllore_effect_core::{WaterTemporalAccum, WaterTorusEffect};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AppliedWaterPreset {
    pub name: String,
}

impl SceneComponent for AppliedWaterPreset {
    const TYPE_KEY: &'static str = "water_preset";
    const PERSISTED_FIELDS: &'static [&'static str] = &["name"];
}
