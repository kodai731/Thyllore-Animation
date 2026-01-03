use serde::Serialize;
use std::rc::Rc;
use vulkanalia::prelude::v1_0::*;

use crate::platform::ImguiData;
use crate::renderer::deferred::RRGBuffer;
use rust_rendering::scene::billboard::BillboardTransform;
use rust_rendering::scene::Camera;
use rust_rendering::vulkanr::buffer::*;
use rust_rendering::vulkanr::command::*;
use rust_rendering::vulkanr::data;
use rust_rendering::vulkanr::descriptor::*;
use rust_rendering::vulkanr::image::*;
use rust_rendering::vulkanr::pipeline::*;
use rust_rendering::vulkanr::render::*;
use rust_rendering::vulkanr::swapchain::*;
use rust_rendering::vulkanr::raytracing::acceleration::*;
use rust_rendering::loader::gltf::gltf::*;
use rust_rendering::loader::fbx::fbx::*;
use rust_rendering::debugview::*;

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

#[derive(Clone, Debug, Serialize)]
pub struct GUIData {
    pub is_left_clicked: bool,
    pub is_wheel_clicked: bool,
    pub monitor_value: f32,
    pub mouse_pos: [f32; 2],
    pub mouse_wheel: f32,
    pub file_path: String,
    pub file_changed: bool,
    pub selected_model_path: String,
    pub load_status: String,
    pub take_screenshot: bool,
    pub imgui_wants_mouse: bool,
    pub show_click_debug: bool,
    pub billboard_click_rect: Option<[f32; 4]>,
    pub debug_shadow_info: bool,
    pub debug_billboard_depth: bool,
    pub show_light_ray_to_model: bool,
    pub is_ctrl_pressed: bool,
    pub move_light_to: LightMoveTarget,
    pub load_cube: bool,
}

impl Default for GUIData {
    fn default() -> Self {
        Self {
            is_left_clicked: false,
            is_wheel_clicked: false,
            monitor_value: 0.0,
            mouse_pos: [0.0, 0.0],
            mouse_wheel: 0.0,
            file_path: String::default(),
            file_changed: false,
            selected_model_path: String::default(),
            load_status: String::from("No model loaded"),
            take_screenshot: false,
            imgui_wants_mouse: false,
            show_click_debug: false,
            billboard_click_rect: None,
            debug_shadow_info: false,
            debug_billboard_depth: false,
            show_light_ray_to_model: false,
            is_ctrl_pressed: false,
            move_light_to: LightMoveTarget::None,
            load_cube: false,
        }
    }
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
    pub model_descriptor_set: RRDescriptorSet,
    pub grid_pipeline: RRPipeline,
    pub grid_descriptor_set: RRDescriptorSet,
    pub grid_vertex_buffer: RRVertexBuffer,
    pub grid_index_buffer: RRIndexBuffer,
    pub gizmo_pipeline: RRPipeline,
    pub gizmo_descriptor_set: RRDescriptorSet,
    pub gizmo_data: GridGizmoData,
    pub light_gizmo_data: LightGizmoData,
    pub light_gizmo_selected: bool,
    pub light_drag_axis: LightGizmoAxis,
    pub light_just_selected: bool,
    pub light_initial_position: [f32; 3],
    pub billboard_pipeline: RRPipeline,
    pub billboard_descriptor_set: RRBillboardDescriptorSet,
    pub billboard_transform: Option<BillboardTransform>,
    pub command_pool: vk::CommandPool,
    pub image_available_semaphores: Vec<vk::Semaphore>,
    pub render_finish_semaphores: Vec<vk::Semaphore>,
    pub in_flight_fences: Vec<vk::Fence>,
    pub images_in_flight: Vec<vk::Fence>,
    pub msaa_samples: vk::SampleCountFlags,
    pub color_image: vk::Image,
    pub color_image_memory: vk::DeviceMemory,
    pub color_image_view: vk::ImageView,
    pub camera: Camera,
    pub grid_vertices: Vec<data::Vertex>,
    pub grid_indices: Vec<u32>,
    pub grid_scale: f32,
    pub near_plane: f32,
    pub far_plane: f32,
    pub is_left_clicked: bool,
    pub clicked_mouse_pos: [f32; 2],
    pub is_wheel_clicked: bool,
    pub gltf_model: GltfModel,
    pub fbx_model: FbxModel,
    pub animation_time: f32,
    pub animation_playing: bool,
    pub current_animation_index: usize,
    pub current_model_path: String,
    pub imgui: ImguiData,
    pub gbuffer: Option<RRGBuffer>,
    pub acceleration_structure: Option<RRAccelerationStructure>,
    pub ray_query_pipeline: Option<RRPipeline>,
    pub ray_query_descriptor: Option<RRRayQueryDescriptorSet>,
    pub scene_uniform_buffer: Option<vk::Buffer>,
    pub scene_uniform_buffer_memory: Option<vk::DeviceMemory>,
    pub composite_pipeline: Option<RRPipeline>,
    pub composite_descriptor: Option<RRCompositeDescriptorSet>,
    pub gbuffer_pipeline: Option<RRPipeline>,
    pub gbuffer_descriptor_set: Option<RRDescriptorSet>,
    pub gbuffer_sampler: Option<vk::Sampler>,
    pub rt_debug_state: RayTracingDebugState,
    pub debug_view_data: DebugViewData,
}
