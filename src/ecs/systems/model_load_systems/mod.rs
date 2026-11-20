mod caches;
mod cleanup;
mod clips;
mod entities;
mod gpu;
mod initial_pose;
mod keep_scene;
mod nodes;
mod replace;
mod source;

#[cfg(feature = "auto-rig")]
pub use keep_scene::load_generated_mesh;
pub use keep_scene::load_model_keeping_scene;
pub(crate) use replace::append_model_to_scene;
#[cfg(test)]
pub(crate) use source::spawn_loaded_model_for_test;
pub use source::{
    find_loaded_model_entity, find_loaded_model_file, has_model_glb, read_model_glb,
    spawn_model_source,
};
