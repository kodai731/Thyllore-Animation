use cgmath::{Quaternion, Vector3};
use serde::{Deserialize, Serialize};
use thyllore_scene_core::SceneComponent;

use super::editor::EntityIcon;
use crate::hooks::scene::SceneOwner;

pub use thyllore_effect_core::{WaterTemporalAccum, WaterTorusEffect};

impl SceneOwner for WaterTorusEffect {
    const ICON: EntityIcon = EntityIcon::Water;

    fn placement(&self) -> (Vector3<f32>, Quaternion<f32>) {
        (self.position, self.rotation)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AppliedWaterPreset {
    pub name: String,
}

impl SceneComponent for AppliedWaterPreset {
    const TYPE_KEY: &'static str = "water_preset";
    const PERSISTED_FIELDS: &'static [&'static str] = &["name"];
}
