use serde::{Deserialize, Serialize};
use thyllore_scene_core::SceneComponent;

/// Names the entity whose Transform the bolt ends at; while attached it drives `end_offset`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LightningTarget {
    pub entity_name: String,
}

impl SceneComponent for LightningTarget {
    const TYPE_KEY: &'static str = "lightning_target";
    const PERSISTED_FIELDS: &'static [&'static str] = &["entity_name"];
}

crate::scene_attachment!(LightningTarget);
