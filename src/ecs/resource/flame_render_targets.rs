use thyllore_vulkan_core::resource::{FlameBuffer, GpuResource};

use crate::gpu_resource;

#[derive(Debug, Default, GpuResource)]
pub struct FlameRenderTargets {
    pub buffer: FlameBuffer,
}

gpu_resource!(FlameRenderTargets);
