mod clip_io;
mod entities;
#[cfg(test)]
pub(crate) use entities::test_support;
mod error;
mod file;
mod motion_path_format;
mod scene_io;
mod transform_format;

pub use clip_io::{load_animation_clip, save_animation_clip};
#[cfg(test)]
pub(crate) use entities::world_with_scene_hooks;
pub use entities::{
    apply_scene_entities, capture_scene_entities, capture_scheduled_clips, SceneEntity,
};
pub use error::{SceneError, SceneResult};
pub use file::{
    AnimationClipFile, AnimationClipRef, ModelReference, SceneFile, SceneMetadata,
    ANIMATION_FORMAT_VERSION, SCENE_FORMAT_VERSION,
};
pub use motion_path_format::{
    motion_path_parameter_snapshot, overwrite_motion_path_persisted_fields,
    MOTION_PATH_SCALAR_PARAMS,
};
pub use scene_io::{
    apply_loaded_scene_to_world, apply_scene_resources, capture_scene_resources,
    find_default_scene, load_scene, save_scene, LoadedScene,
};
pub use transform_format::{
    overwrite_transform_persisted_fields, transform_parameter_snapshot, TRANSFORM_SCALAR_PARAMS,
};
