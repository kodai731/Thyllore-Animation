use std::path::PathBuf;

use crate::scene::{is_scene_in_assets_tree, scene_assets_dir, SceneMetadata};

const DEFAULT_ASSETS_DIR: &str = "assets";

#[derive(Debug, Clone, Default)]
pub struct SceneState {
    pub current_scene_path: Option<PathBuf>,
    pub previous_metadata: Option<SceneMetadata>,
}

impl SceneState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_from_loaded(&mut self, path: PathBuf, metadata: SceneMetadata) {
        self.current_scene_path = Some(path);
        self.previous_metadata = Some(metadata);
    }

    /// The directory the scene's relative asset paths start from, `assets` without a scene file.
    pub fn assets_dir(&self) -> PathBuf {
        self.current_scene_path
            .as_deref()
            .map(scene_assets_dir)
            .unwrap_or_else(|| PathBuf::from(DEFAULT_ASSETS_DIR))
    }

    /// The assets tree a new path is stored relative to; a scene file outside one keeps paths absolute.
    pub fn assets_tree(&self) -> Option<PathBuf> {
        let is_in_assets_tree = self
            .current_scene_path
            .as_deref()
            .is_none_or(is_scene_in_assets_tree);

        is_in_assets_tree.then(|| self.assets_dir())
    }
}
