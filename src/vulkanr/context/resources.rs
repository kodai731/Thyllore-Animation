use crate::vulkanr::pipeline::RRPipeline;
use crate::vulkanr::render::RRRender;
use crate::vulkanr::resource::graphics_resource::GraphicsResources;
use crate::vulkanr::resource::{GpuResource, RRGBuffer};
use crate::vulkanr::swapchain::RRSwapchain;

#[must_use]
#[derive(GpuResource)]
pub struct SwapchainState {
    pub swapchain: RRSwapchain,
    #[gpu_resource(skip)]
    pub images_in_flight: Vec<Fence>,
}

impl SwapchainState {
    pub fn new(swapchain: RRSwapchain, image_count: usize) -> Self {
        Self {
            swapchain,
            images_in_flight: vec![Fence::default(); image_count],
        }
    }

    pub fn extent(&self) -> Extent2D {
        self.swapchain.swapchain_extent()
    }

    pub fn image_count(&self) -> usize {
        self.swapchain.swapchain_images().len()
    }
}

#[must_use]
#[derive(GpuResource)]
pub struct RenderTargets {
    pub render: RRRender,
    pub gbuffer: Option<RRGBuffer>,
}

impl RenderTargets {
    pub fn new(render: RRRender) -> Self {
        Self {
            render,
            gbuffer: None,
        }
    }

    pub fn set_gbuffer(&mut self, gbuffer: RRGBuffer) {
        self.gbuffer = Some(gbuffer);
    }
}

pub struct PipelineState {
    pub model_pipeline: RRPipeline,
}

impl PipelineState {
    pub fn new(model_pipeline: RRPipeline) -> Self {
        Self { model_pipeline }
    }
}

pub struct GpuAssets {
    pub resources: GraphicsResources,
}

impl GpuAssets {
    pub fn new(resources: GraphicsResources) -> Self {
        Self { resources }
    }
}

use crate::ecs::resource::DebugViewState;
use crate::ecs::resource::LightState;
use crate::platform::ImguiData;
use thyllore_vulkan_core::pipeline::SampleCount;
use thyllore_vulkan_core::resource::raytracing_data::RayTracingData;
use thyllore_vulkan_core::types::{Extent2D, Fence, SampleCountFlags};

pub struct RayTracingState {
    pub data: RayTracingData,
}

impl RayTracingState {
    pub fn new(data: RayTracingData) -> Self {
        Self { data }
    }

    pub fn is_available(&self) -> bool {
        self.data.is_available()
    }
}

impl Default for RayTracingState {
    fn default() -> Self {
        Self {
            data: RayTracingData::default(),
        }
    }
}

pub struct DebugState {
    pub light: LightState,
    pub view: DebugViewState,
}

impl DebugState {
    pub fn new(light: LightState, view: DebugViewState) -> Self {
        Self { light, view }
    }
}

impl Default for DebugState {
    fn default() -> Self {
        Self {
            light: LightState::default(),
            view: DebugViewState::default(),
        }
    }
}

pub struct ImGuiState {
    pub data: ImguiData,
}

impl ImGuiState {
    pub fn new(data: ImguiData) -> Self {
        Self { data }
    }
}

impl Default for ImGuiState {
    fn default() -> Self {
        Self {
            data: ImguiData::default(),
        }
    }
}

pub struct RenderConfig {
    pub msaa_samples: SampleCountFlags,
}

impl RenderConfig {
    pub fn new(msaa_samples: SampleCountFlags) -> Self {
        Self { msaa_samples }
    }
}

impl Default for RenderConfig {
    fn default() -> Self {
        Self {
            msaa_samples: SampleCount::One.into(),
        }
    }
}
