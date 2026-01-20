use std::rc::Rc;
use vulkanalia::prelude::v1_0::*;

use crate::scene::graphics_resource::GraphicsResources;
use crate::vulkanr::command::{RRCommandBuffer, RRCommandPool};
use crate::vulkanr::pipeline::RRPipeline;
use crate::vulkanr::render::RRRender;
use crate::vulkanr::resource::RRGBuffer;
use crate::vulkanr::swapchain::RRSwapchain;

pub struct FrameSync {
    pub image_available: Vec<vk::Semaphore>,
    pub render_finished: Vec<vk::Semaphore>,
    pub in_flight: Vec<vk::Fence>,
    pub current_frame: usize,
}

impl FrameSync {
    pub fn new(
        image_available: Vec<vk::Semaphore>,
        render_finished: Vec<vk::Semaphore>,
        in_flight: Vec<vk::Fence>,
    ) -> Self {
        Self {
            image_available,
            render_finished,
            in_flight,
            current_frame: 0,
        }
    }

    pub fn advance(&mut self, max_frames: usize) {
        self.current_frame = (self.current_frame + 1) % max_frames;
    }

    pub fn current_image_available(&self) -> vk::Semaphore {
        self.image_available[self.current_frame]
    }

    pub fn current_render_finished(&self) -> vk::Semaphore {
        self.render_finished[self.current_frame]
    }

    pub fn current_fence(&self) -> vk::Fence {
        self.in_flight[self.current_frame]
    }
}

pub struct SwapchainState {
    pub swapchain: RRSwapchain,
    pub images_in_flight: Vec<vk::Fence>,
}

impl SwapchainState {
    pub fn new(swapchain: RRSwapchain, image_count: usize) -> Self {
        Self {
            swapchain,
            images_in_flight: vec![vk::Fence::null(); image_count],
        }
    }

    pub fn extent(&self) -> vk::Extent2D {
        self.swapchain.swapchain_extent
    }

    pub fn format(&self) -> vk::Format {
        self.swapchain.swapchain_format
    }

    pub fn image_count(&self) -> usize {
        self.swapchain.swapchain_images.len()
    }
}

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

pub struct CommandState {
    pub pool: Rc<RRCommandPool>,
    pub buffers: RRCommandBuffer,
}

impl CommandState {
    pub fn new(pool: Rc<RRCommandPool>, buffers: RRCommandBuffer) -> Self {
        Self { pool, buffers }
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

pub struct SurfaceState {
    pub surface: vk::SurfaceKHR,
    pub messenger: vk::DebugUtilsMessengerEXT,
}

impl SurfaceState {
    pub fn new(surface: vk::SurfaceKHR, messenger: vk::DebugUtilsMessengerEXT) -> Self {
        Self { surface, messenger }
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

use crate::debugview::{DebugViewData, RayTracingDebugState};
use crate::platform::ImguiData;
use crate::scene::raytracing::RayTracingData;
use crate::scene::Camera;

pub struct CameraState {
    pub camera: Camera,
}

impl CameraState {
    pub fn new(camera: Camera) -> Self {
        Self { camera }
    }

    pub fn position(&self) -> cgmath::Vector3<f32> {
        self.camera.position
    }

    pub fn direction(&self) -> cgmath::Vector3<f32> {
        self.camera.direction
    }

    pub fn up(&self) -> cgmath::Vector3<f32> {
        self.camera.up
    }
}

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
    pub rt_debug: RayTracingDebugState,
    pub view_data: DebugViewData,
}

impl DebugState {
    pub fn new(rt_debug: RayTracingDebugState, view_data: DebugViewData) -> Self {
        Self {
            rt_debug,
            view_data,
        }
    }
}

impl Default for DebugState {
    fn default() -> Self {
        Self {
            rt_debug: RayTracingDebugState::default(),
            view_data: DebugViewData::default(),
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
    pub msaa_samples: vk::SampleCountFlags,
}

impl RenderConfig {
    pub fn new(msaa_samples: vk::SampleCountFlags) -> Self {
        Self { msaa_samples }
    }
}

impl Default for RenderConfig {
    fn default() -> Self {
        Self {
            msaa_samples: vk::SampleCountFlags::_1,
        }
    }
}
