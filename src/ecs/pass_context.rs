use anyhow::Result;

use crate::asset::AssetStorage;
use crate::ecs::World;
use crate::vulkanr::core::RRDevice;
use crate::vulkanr::renderer::onion_skin_buffers::OnionSkinGpuState;
use crate::vulkanr::resource::graphics_resource::GraphicsResources;
use crate::vulkanr::resource::{
    AutoExposureBuffers, BloomChain, DofBuffer, GpuBufferRegistry, HdrBuffer, OffscreenFramebuffer,
    PipelineStorage, RenderTargetTransient, TransientImage,
};
use thyllore_vulkan_core::renderer::{FrameTransients, TransientSlot};
use thyllore_vulkan_core::resource::raytracing_data::RayTracingData;
use thyllore_vulkan_core::FrameRenderContext;

/// What a `RenderPassNode` may read while declaring targets and recording, and what `prepare`
/// may bind: the frame's GPU objects and `World`, never `App`.
pub struct PassContext<'a> {
    pub rrdevice: &'a RRDevice,
    pub graphics: &'a GraphicsResources,
    pub buffers: &'a GpuBufferRegistry,
    pub pipelines: &'a PipelineStorage,
    pub raytracing: &'a RayTracingData,
    pub hdr_buffer: Option<&'a HdrBuffer>,
    pub offscreen: Option<&'a OffscreenFramebuffer>,
    pub bloom_chain: Option<&'a BloomChain>,
    pub dof_buffer: Option<&'a DofBuffer>,
    pub auto_exposure_buffers: Option<&'a AutoExposureBuffers>,
    pub hdr_grid_pipeline_id: Option<usize>,
    pub transient: &'a mut RenderTargetTransient,
    pub frame_transients: &'a FrameTransients,
    pub onion_skin_gpu: Option<&'a OnionSkinGpuState>,
    pub world: &'a World,
    pub assets: &'a AssetStorage,
}

impl<'a> PassContext<'a> {
    pub fn frame_render_context(&self, image_index: usize) -> FrameRenderContext<'_> {
        FrameRenderContext {
            device: self.rrdevice,
            graphics: self.graphics,
            buffers: self.buffers,
            pipelines: self.pipelines,
            image_index,
        }
    }

    pub fn transient_image(&self, slot: TransientSlot) -> Result<TransientImage> {
        self.transient.get(self.frame_transients.handle(slot)?)
    }
}
