use cgmath::Matrix4;

use crate::animation::SkeletonId;
use crate::ecs::component::{LineMesh, RenderInfo};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum BoneDisplayStyle {
    #[default]
    Stick,
    Octahedral,
    Box,
    Sphere,
}

#[derive(Clone, Debug)]
pub struct BoneGizmoData {
    pub visible: bool,
    pub display_style: BoneDisplayStyle,
    pub in_front: bool,
    pub distance_scaling_enabled: bool,
    pub distance_scaling_factor: f32,
    pub cached_global_transforms: Vec<Matrix4<f32>>,
    pub cached_skeleton_id: Option<SkeletonId>,
    pub bone_local_offsets: Vec<[f32; 3]>,

    pub stick_mesh: LineMesh,
    pub stick_render_info: RenderInfo,

    pub solid_mesh: LineMesh,
    pub solid_render_info: RenderInfo,

    pub wire_mesh: LineMesh,
    pub wire_render_info: RenderInfo,

    pub solid_depth_render_info: RenderInfo,
    pub wire_depth_render_info: RenderInfo,
    pub solid_occluded_render_info: RenderInfo,
    pub wire_occluded_render_info: RenderInfo,
}

impl Default for BoneGizmoData {
    fn default() -> Self {
        Self {
            visible: false,
            display_style: BoneDisplayStyle::default(),
            in_front: true,
            distance_scaling_enabled: false,
            distance_scaling_factor: 0.03,
            cached_global_transforms: Vec::new(),
            cached_skeleton_id: None,
            bone_local_offsets: Vec::new(),
            stick_mesh: LineMesh::default(),
            stick_render_info: RenderInfo::default(),
            solid_mesh: LineMesh::default(),
            solid_render_info: RenderInfo::default(),
            wire_mesh: LineMesh::default(),
            wire_render_info: RenderInfo::default(),
            solid_depth_render_info: RenderInfo::default(),
            wire_depth_render_info: RenderInfo::default(),
            solid_occluded_render_info: RenderInfo::default(),
            wire_occluded_render_info: RenderInfo::default(),
        }
    }
}
