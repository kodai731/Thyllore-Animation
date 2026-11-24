use std::collections::BTreeMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

use super::entities::SceneEntity;
use crate::hooks::scene::SceneValue;
use thyllore_anim_core::editable::EditableAnimationClip;

pub const SCENE_FORMAT_VERSION: u32 = 9;
pub(super) const ANIMATIONS_DIR_NAME: &str = "animations";

/// On-disk scene: the clip files, every registered resource and entity component keyed by its type key.
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

    /// A file outside the scene's animations directory keeps its own path; `None` for a clip without a file.
    pub fn for_clip(clip: &EditableAnimationClip, assets_dir: &Path) -> Option<Self> {
        let clip_file = Path::new(clip.source_path.as_deref()?);
        if is_file_in_directory(clip_file, &assets_dir.join(ANIMATIONS_DIR_NAME)) {
            return Self::for_clip_file(clip_file);
        }

        Some(Self::new(&clip_file.to_string_lossy()))
    }

    /// A scene names a file of its animations directory relative to the assets directory.
    pub fn for_clip_file(clip_file: &Path) -> Option<Self> {
        let file_name = clip_file.file_name()?.to_string_lossy();
        Some(Self::new(&format!("{ANIMATIONS_DIR_NAME}/{file_name}")))
    }
}

/// Compared through the links when both exist, by name otherwise.
pub(super) fn is_file_in_directory(file: &Path, directory: &Path) -> bool {
    let Some(file_directory) = file.parent() else {
        return false;
    };
    if let (Ok(linked_file_directory), Ok(linked_directory)) =
        (file_directory.canonicalize(), directory.canonicalize())
    {
        return linked_file_directory == linked_directory;
    }

    match (
        std::path::absolute(file_directory),
        std::path::absolute(directory),
    ) {
        (Ok(named_file_directory), Ok(named_directory)) => named_file_directory == named_directory,
        _ => false,
    }
}
