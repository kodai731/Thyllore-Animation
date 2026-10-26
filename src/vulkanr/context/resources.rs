use std::rc::Rc;
use vulkanalia::prelude::v1_0::*;

use crate::vulkanr::command::{RRCommandBuffer, RRCommandPool};
use crate::vulkanr::core::RRDevice;
use crate::vulkanr::pipeline::RRPipeline;
use crate::vulkanr::render::RRRender;
use crate::vulkanr::resource::graphics_resource::GraphicsResources;
use crate::vulkanr::resource::{GpuResource, RRGBuffer};
use crate::vulkanr::swapchain::RRSwapchain;

pub struct FrameSync {
    pub image_available: Vec<Semaphore>,
    pub render_finished: Vec<Semaphore>,
    pub in_flight: Vec<Fence>,
    pub current_frame: usize,
}

impl FrameSync {
    pub fn new(
        image_available: Vec<Semaphore>,
        render_finished: Vec<Semaphore>,
        in_flight: Vec<Fence>,
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

    pub fn current_image_available(&self) -> Semaphore {
        self.image_available[self.current_frame]
    }

    pub fn current_render_finished(&self) -> Semaphore {
        self.render_finished[self.current_frame]
    }

    pub fn current_fence(&self) -> Fence {
        self.in_flight[self.current_frame]
    }
}

impl GpuResource for FrameSync {
    unsafe fn destroy_gpu(&mut self, rrdevice: &RRDevice) {
        let device = &rrdevice.device;
        for semaphore in self
            .image_available
            .drain(..)
            .chain(self.render_finished.drain(..))
        {
            device.destroy_semaphore(semaphore, None);
        }
        for fence in self.in_flight.drain(..) {
            device.destroy_fence(fence, None);
        }
    }

    fn is_destroyed(&self) -> bool {
        self.image_available.is_empty()
            && self.render_finished.is_empty()
            && self.in_flight.is_empty()
    }
}

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
            images_in_flight: vec![Fence::null(); image_count],
        }
    }

    pub fn extent(&self) -> Extent2D {
        self.swapchain.swapchain_extent
    }

    pub fn format(&self) -> vk::Format {
        self.swapchain.swapchain_format
    }

    pub fn image_count(&self) -> usize {
        self.swapchain.swapchain_images.len()
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

pub struct CommandState {
    pub pool: Rc<RRCommandPool>,
    pub buffers: RRCommandBuffer,
}

impl CommandState {
    pub fn new(pool: Rc<RRCommandPool>, buffers: RRCommandBuffer) -> Self {
        Self { pool, buffers }
    }
}

impl GpuResource for CommandState {
    unsafe fn destroy_gpu(&mut self, rrdevice: &RRDevice) {
        let device = &rrdevice.device;
        if !self.buffers.command_buffers.is_empty() {
            device.free_command_buffers(self.pool.command_pool, &self.buffers.command_buffers);
            self.buffers.command_buffers.clear();
        }
        if self.pool.command_pool != CommandPool::null() {
            device.destroy_command_pool(self.pool.command_pool, None);
        }
    }

    fn is_destroyed(&self) -> bool {
        self.buffers.command_buffers.is_empty()
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

use crate::ecs::resource::Camera;
use crate::ecs::resource::DebugViewState;
use crate::ecs::resource::LightState;
use crate::ecs::systems::camera_systems::{
    compute_camera_direction, compute_camera_position, compute_camera_up,
};
use crate::platform::ImguiData;
use thyllore_vulkan_core::pipeline::SampleCount;
use thyllore_vulkan_core::resource::raytracing_data::RayTracingData;
use thyllore_vulkan_core::types::{CommandPool, Extent2D, Fence, SampleCountFlags, Semaphore};

pub struct CameraState {
    pub camera: Camera,
}

impl CameraState {
    pub fn new(camera: Camera) -> Self {
        Self { camera }
    }

    pub fn position(&self) -> cgmath::Vector3<f32> {
        compute_camera_position(&self.camera)
    }

    pub fn direction(&self) -> cgmath::Vector3<f32> {
        compute_camera_direction(&self.camera)
    }

    pub fn up(&self) -> cgmath::Vector3<f32> {
        compute_camera_up(&self.camera)
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
