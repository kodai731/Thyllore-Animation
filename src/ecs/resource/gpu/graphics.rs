use thyllore_avatar_core::humanoid::components::mapping::HumanoidMapping;

use crate::vulkanr::resource::graphics_resource::{MeshBuffer, NodeData};

pub use thyllore_anim_core::AnimationType;

#[derive(Clone, Debug)]
pub struct ModelState {
    pub has_skinned_meshes: bool,
    pub model_path: String,
    pub load_status: String,
    pub imported_humanoid: Option<HumanoidMapping>,
}

impl Default for ModelState {
    fn default() -> Self {
        Self {
            has_skinned_meshes: false,
            model_path: String::new(),
            load_status: String::from("No model loaded"),
            imported_humanoid: None,
        }
    }
}

#[derive(Debug, Default)]
pub struct MeshAssets {
    pub meshes: Vec<MeshBuffer>,
}

impl MeshAssets {
    pub fn new() -> Self {
        Self { meshes: Vec::new() }
    }

    pub fn mesh_count(&self) -> usize {
        self.meshes.len()
    }
}

#[derive(Clone, Debug, Default)]
pub struct NodeAssets {
    pub nodes: Vec<NodeData>,
}

impl NodeAssets {
    pub fn new() -> Self {
        Self { nodes: Vec::new() }
    }
}
