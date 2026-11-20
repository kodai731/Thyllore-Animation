use std::path::{Path, PathBuf};

use crate::scene::SceneMetadata;

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

    /// The directory the scene's relative asset paths start from: a scene file sits in `<assets>/scenes/`.
    pub fn assets_dir(&self) -> PathBuf {
        self.current_scene_path
            .as_deref()
            .and_then(Path::parent)
            .and_then(Path::parent)
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from(DEFAULT_ASSETS_DIR))
    }
}
