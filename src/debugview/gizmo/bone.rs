use cgmath::Matrix4;

use crate::animation::SkeletonId;
use crate::ecs::component::{LineMesh, RenderInfo};

#[derive(Clone, Debug)]
pub struct BoneGizmoData {
    pub mesh: LineMesh,
    pub render_info: RenderInfo,
    pub visible: bool,
    pub cached_global_transforms: Vec<Matrix4<f32>>,
    pub cached_skeleton_id: Option<SkeletonId>,
    pub bone_local_offsets: Vec<[f32; 3]>,
}

impl Default for BoneGizmoData {
    fn default() -> Self {
        Self {
            mesh: LineMesh::default(),
            render_info: RenderInfo::default(),
            visible: false,
            cached_global_transforms: Vec::new(),
            cached_skeleton_id: None,
            bone_local_offsets: Vec::new(),
        }
    }
}
