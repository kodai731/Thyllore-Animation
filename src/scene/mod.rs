pub mod camera;
mod clip_io;
mod error;
mod format;
mod scene_io;

pub use camera::Camera;
pub use clip_io::{load_animation_clip, save_animation_clip};
pub use error::{SceneError, SceneResult};
pub use format::{
    AnimationClipFile, AnimationClipRef, EditorState, ModelReference, SceneFile,
    SceneMetadata, TimelineConfig, TransformData, ANIMATION_FORMAT_VERSION, SCENE_FORMAT_VERSION,
    CameraState as SavedCameraState,
};
pub use scene_io::{
    apply_loaded_scene_to_world, find_default_scene, load_scene, save_scene, LoadedScene,
};
