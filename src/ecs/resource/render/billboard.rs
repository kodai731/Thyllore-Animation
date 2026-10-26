use crate::ecs::component::RenderInfo;
use crate::gpu_resource;
use crate::vulkanr::resource::GpuResource;

pub use thyllore_render_core::{BillboardMesh, BillboardTransform, BillboardVertex};
pub use thyllore_vulkan_core::resource::BillboardRenderState;

#[must_use]
#[derive(Clone, Debug, Default, GpuResource)]
pub struct BillboardData {
    #[gpu_resource(skip)]
    pub mesh: BillboardMesh,
    #[gpu_resource(skip)]
    pub transform: Option<BillboardTransform>,
    #[gpu_resource(skip)]
    pub render_info: RenderInfo,
    pub render_state: BillboardRenderState,
}

gpu_resource!(BillboardData);

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
