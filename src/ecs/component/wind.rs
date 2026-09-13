use cgmath::{Quaternion, Vector3};
use serde::{Deserialize, Serialize};
use thyllore_scene_core::SceneComponent;

use super::editor::EntityIcon;
use crate::hooks::scene::SceneOwner;

pub use thyllore_effect_core::WindTornadoEffect;

impl SceneOwner for WindTornadoEffect {
    const ICON: EntityIcon = EntityIcon::Wind;

    fn placement(&self) -> (Vector3<f32>, Quaternion<f32>) {
        (self.position, self.rotation)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AppliedWindPreset {
    pub name: String,
}

impl SceneComponent for AppliedWindPreset {
    const TYPE_KEY: &'static str = "wind_preset";
    const PERSISTED_FIELDS: &'static [&'static str] = &["name"];
}
