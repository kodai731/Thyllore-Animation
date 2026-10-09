use serde::{Deserialize, Serialize};
use thyllore_scene_core::SceneComponent;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct LightningPath {
    pub waypoints: Vec<String>,
}

impl SceneComponent for LightningPath {
    const TYPE_KEY: &'static str = "lightning_path";
    const PERSISTED_FIELDS: &'static [&'static str] = &["waypoints"];
}

crate::scene_attachment!(LightningPath);
