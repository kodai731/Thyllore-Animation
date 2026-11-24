mod clip_io;
mod entities;
#[cfg(test)]
pub(crate) use entities::test_support;
mod error;
mod file;
mod motion_path_format;
mod scene_io;
mod transform_format;

pub use clip_io::{load_animation_clip, parse_animation_clip, save_animation_clip};
#[cfg(test)]
pub(crate) use entities::world_with_scene_hooks;
pub use entities::{
    add_scene_entities, apply_scene_entities, capture_scene_entities, capture_scheduled_clips,
    find_scene_owner_entities, SceneEntity,
};
pub use error::{SceneError, SceneResult};
pub use file::{AnimationClipRef, SceneFile, SceneMetadata, SCENE_FORMAT_VERSION};
pub use motion_path_format::{
    motion_path_parameter_snapshot, overwrite_motion_path_persisted_fields,
    MOTION_PATH_SCALAR_PARAMS,
};
pub use scene_io::{
    apply_loaded_scene_to_world, apply_scene_resources, capture_scene_resources,
    default_scene_path, find_default_scene, is_scene_in_assets_tree, load_scene, save_scene,
    scene_assets_dir, LoadedScene,
};
pub use transform_format::{
    overwrite_transform_persisted_fields, transform_parameter_snapshot, TRANSFORM_SCALAR_PARAMS,
};
