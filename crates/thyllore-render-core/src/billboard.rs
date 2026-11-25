use glam::{Mat4, Vec3};

use crate::StaticMesh;

#[repr(C)]
#[derive(Clone, Debug, Copy, Default)]
pub struct BillboardVertex {
    pub pos: [f32; 3],
    pub tex_coord: [f32; 2],
}

pub type BillboardMesh = StaticMesh<BillboardVertex>;

#[derive(Clone, Debug)]
pub struct BillboardTransform {
    pub position: Vec3,
    pub model_matrix: Mat4,
}
