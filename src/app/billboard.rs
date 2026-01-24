use cgmath::{Matrix4, Vector3};

use crate::ecs::component::{DynamicMesh, RenderInfo};
use crate::vulkanr::descriptor::RRBillboardDescriptorSet;
use crate::vulkanr::image::RRImage;

#[repr(C)]
#[derive(Clone, Debug, Copy, Default)]
pub struct BillboardVertex {
    pub pos: [f32; 3],
    pub tex_coord: [f32; 2],
}

pub type BillboardMesh = DynamicMesh<BillboardVertex>;

#[derive(Clone, Debug)]
pub struct BillboardTransform {
    pub position: Vector3<f32>,
    pub model_matrix: Matrix4<f32>,
}

#[derive(Clone, Debug, Default)]
pub struct BillboardRenderState {
    pub descriptor_set: RRBillboardDescriptorSet,
    pub texture: Option<RRImage>,
}

#[derive(Clone, Debug, Default)]
pub struct BillboardData {
    pub mesh: BillboardMesh,
    pub transform: Option<BillboardTransform>,
    pub render_info: RenderInfo,
    pub render_state: BillboardRenderState,
}

impl BillboardData {
    pub fn transform(&self) -> Option<&BillboardTransform> {
        self.transform.as_ref()
    }

    pub fn transform_mut(&mut self) -> &mut Option<BillboardTransform> {
        &mut self.transform
    }

    pub fn vertices(&self) -> &[BillboardVertex] {
        &self.mesh.vertices
    }

    pub fn indices(&self) -> &[u32] {
        &self.mesh.indices
    }
}
