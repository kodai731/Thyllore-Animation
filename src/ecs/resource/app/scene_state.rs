use std::path::PathBuf;

use crate::scene::SceneMetadata;

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

    pub fn clear(&mut self) {
        self.current_scene_path = None;
        self.previous_metadata = None;
    }
}
