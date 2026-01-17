use serde::Serialize;
use std::rc::Rc;
use vulkanalia::prelude::v1_0::*;

use crate::debugview::*;
use crate::platform::ImguiData;
use crate::scene::assets::AssetStorage;
use crate::scene::graphics_resource::GraphicsResources;
use crate::scene::raytracing::RayTracingData;
use crate::scene::world::World;
use crate::scene::Camera;
use crate::vulkanr::command::*;
use crate::vulkanr::pipeline::*;
use crate::vulkanr::render::*;
use crate::vulkanr::swapchain::*;

#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub enum LightMoveTarget {
    None,
    XMin,
    XMax,
    YMin,
    YMax,
    ZMin,
    ZMax,
}

#[derive(Clone, Debug, Default)]
pub struct AppData {
    pub messenger: vk::DebugUtilsMessengerEXT,
    pub surface: vk::SurfaceKHR,
    pub rrswapchain: RRSwapchain,
    pub rrrender: RRRender,
    pub rrcommand_pool: Rc<RRCommandPool>,
    pub rrcommand_buffer: RRCommandBuffer,
    pub model_pipeline: RRPipeline,
    pub graphics_resources: GraphicsResources,
    pub command_pool: vk::CommandPool,
    pub image_available_semaphores: Vec<vk::Semaphore>,
    pub render_finish_semaphores: Vec<vk::Semaphore>,
    pub in_flight_fences: Vec<vk::Fence>,
    pub images_in_flight: Vec<vk::Fence>,
    pub msaa_samples: vk::SampleCountFlags,
    pub camera: Camera,
    pub animation_time: f32,
    pub animation_playing: bool,
    pub current_animation_index: usize,
    pub current_model_path: String,
    pub imgui: ImguiData,
    pub raytracing: RayTracingData,
    pub rt_debug_state: RayTracingDebugState,
    pub debug_view_data: DebugViewData,
    pub ecs_world: World,
    pub ecs_assets: AssetStorage,
}
