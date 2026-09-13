use cgmath::{Quaternion, Vector3};
use serde::{Deserialize, Serialize};
use thyllore_scene_core::SceneComponent;

use super::editor::EntityIcon;
use crate::hooks::scene::SceneOwner;

pub use thyllore_effect_core::flame_plume::HeatPlume;
pub use thyllore_effect_core::{FlameBaked, FlameEffect, FlameTemporalAccum};

impl SceneOwner for FlameEffect {
    const ICON: EntityIcon = EntityIcon::Flame;

    fn placement(&self) -> (Vector3<f32>, Quaternion<f32>) {
        (self.position, self.rotation)
    }

    fn prepare_loaded(&mut self) {
        thyllore_effect_core::refresh_flame_coefficients(self, &FlameBaked::default());
    }
}

/// Provenance of the last style applied to this flame. Values are baked into
/// FlameEffect on apply; this records only which style they came from, so a
/// saved scene names its look without depending on the style file.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppliedFlameStyle {
    pub name: String,
    pub version: u32,
}

impl SceneComponent for AppliedFlameStyle {
    const TYPE_KEY: &'static str = "flame_style";
    const PERSISTED_FIELDS: &'static [&'static str] = &["name", "version"];
}
