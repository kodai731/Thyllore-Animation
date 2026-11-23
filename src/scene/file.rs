use std::collections::BTreeMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

use super::entities::SceneEntity;
use crate::hooks::scene::SceneValue;

pub const SCENE_FORMAT_VERSION: u32 = 9;

/// On-disk scene: the clip files, every registered resource and entity component keyed by its
/// type key.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneFile {
    pub version: u32,
    #[serde(default)]
    pub metadata: SceneMetadata,
    #[serde(default)]
    pub animation_clips: Vec<AnimationClipRef>,
    #[serde(default)]
    pub resources: BTreeMap<String, SceneValue>,
    #[serde(default)]
    pub entities: Vec<SceneEntity>,
}

impl SceneFile {
    pub fn new(name: &str) -> Self {
        Self {
            version: SCENE_FORMAT_VERSION,
            metadata: SceneMetadata::new(name),
            animation_clips: Vec::new(),
            resources: BTreeMap::new(),
            entities: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SceneMetadata {
    pub name: String,
    pub created_at: String,
    pub modified_at: String,
}

impl SceneMetadata {
    pub fn new(name: &str) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            name: name.to_string(),
            created_at: now.clone(),
            modified_at: now,
        }
    }

    pub fn update_modified(&mut self) {
        self.modified_at = chrono::Utc::now().to_rfc3339();
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnimationClipRef {
    pub path: String,
}

impl AnimationClipRef {
    pub fn new(path: &str) -> Self {
        Self {
            path: path.to_string(),
        }
    }

    /// A scene names a clip file relative to the assets directory.
    pub fn for_clip_file(clip_file: &Path) -> Option<Self> {
        let file_name = clip_file.file_name()?.to_string_lossy();
        Some(Self::new(&format!("animations/{file_name}")))
    }
}
