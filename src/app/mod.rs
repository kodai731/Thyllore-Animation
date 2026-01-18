pub mod data;
pub mod init;
pub mod model_loader;
pub mod render;
pub mod scene_model;
pub mod update;
pub mod cleanup;
pub mod util;
pub mod gui_data;

pub use data::AppData;
pub use gui_data::GUIData;
pub use init::*;

use crate::debugview::gizmo::{GridGizmoData, LightGizmoData};
use crate::debugview::{DebugViewData, RayTracingDebugState};
use crate::ecs::{
    AnimationPlayback, AnimationRegistry, GpuDescriptors, MaterialRegistry, MeshAssets, ModelState,
    NodeAssets, ResMut, ResRef, Resource,
};
use crate::platform::ImguiData;
use crate::scene::billboard::BillboardData;
use crate::scene::graphics_resource::GraphicsResources;
use crate::scene::grid::GridData;
use crate::scene::raytracing::RayTracingData;
use crate::scene::Camera;
use crate::vulkanr::context::{
    CameraState, CommandState, DebugState, FrameSync, GpuAssets, ImGuiState, PipelineState,
    RayTracingState, RenderConfig, RenderTargets, SurfaceState, SwapchainState,
};
use crate::vulkanr::device::*;

use std::time::Instant;
use vulkanalia::prelude::v1_0::*;

pub struct App {
    pub entry: Entry,
    pub instance: Instance,
    pub rrdevice: RRDevice,
    pub data: AppData,
    pub frame: usize,
    pub resized: bool,
    pub start: Instant,
}

impl App {
    pub fn resource<R: Resource>(&self) -> ResRef<R> {
        self.data.ecs_world.resource::<R>()
    }

    pub fn resource_mut<R: Resource>(&self) -> ResMut<R> {
        self.data.ecs_world.resource_mut::<R>()
    }

    pub fn get_resource<R: Resource>(&self) -> Option<ResRef<R>> {
        self.data.ecs_world.get_resource::<R>()
    }

    pub fn frame_sync(&self) -> ResRef<FrameSync> {
        self.resource::<FrameSync>()
    }

    pub fn frame_sync_mut(&self) -> ResMut<FrameSync> {
        self.resource_mut::<FrameSync>()
    }

    pub fn swapchain_state(&self) -> ResRef<SwapchainState> {
        self.resource::<SwapchainState>()
    }

    pub fn render_targets(&self) -> ResRef<RenderTargets> {
        self.resource::<RenderTargets>()
    }

    pub fn command_state(&self) -> ResRef<CommandState> {
        self.resource::<CommandState>()
    }

    pub fn pipeline_state(&self) -> ResRef<PipelineState> {
        self.resource::<PipelineState>()
    }

    pub fn surface_state(&self) -> ResRef<SurfaceState> {
        self.resource::<SurfaceState>()
    }

    pub fn graphics_resources(&self) -> &GraphicsResources {
        &self.data.graphics_resources
    }

    pub fn graphics_resources_mut(&mut self) -> &mut GraphicsResources {
        &mut self.data.graphics_resources
    }

    pub fn animation_playback(&self) -> ResRef<AnimationPlayback> {
        self.resource::<AnimationPlayback>()
    }

    pub fn animation_playback_mut(&self) -> ResMut<AnimationPlayback> {
        self.resource_mut::<AnimationPlayback>()
    }

    pub fn camera(&self) -> ResRef<Camera> {
        self.resource::<Camera>()
    }

    pub fn camera_mut(&self) -> ResMut<Camera> {
        self.resource_mut::<Camera>()
    }

    pub fn raytracing(&self) -> &RayTracingData {
        &self.data.raytracing
    }

    pub fn raytracing_mut(&mut self) -> &mut RayTracingData {
        &mut self.data.raytracing
    }

    pub fn rt_debug_state(&self) -> ResRef<RayTracingDebugState> {
        self.resource::<RayTracingDebugState>()
    }

    pub fn rt_debug_state_mut(&self) -> ResMut<RayTracingDebugState> {
        self.resource_mut::<RayTracingDebugState>()
    }

    pub fn debug_view_data(&self) -> &DebugViewData {
        &self.data.debug_view_data
    }

    pub fn debug_view_data_mut(&mut self) -> &mut DebugViewData {
        &mut self.data.debug_view_data
    }

    pub fn imgui_data(&self) -> &ImguiData {
        &self.data.imgui
    }

    pub fn imgui_data_mut(&mut self) -> &mut ImguiData {
        &mut self.data.imgui
    }

    pub fn render_config(&self) -> ResRef<RenderConfig> {
        self.resource::<RenderConfig>()
    }

    pub fn render_config_mut(&self) -> ResMut<RenderConfig> {
        self.resource_mut::<RenderConfig>()
    }

    pub fn gpu_descriptors(&self) -> ResRef<GpuDescriptors> {
        self.resource::<GpuDescriptors>()
    }

    pub fn gpu_descriptors_mut(&self) -> ResMut<GpuDescriptors> {
        self.resource_mut::<GpuDescriptors>()
    }

    pub fn material_registry(&self) -> ResRef<MaterialRegistry> {
        self.resource::<MaterialRegistry>()
    }

    pub fn material_registry_mut(&self) -> ResMut<MaterialRegistry> {
        self.resource_mut::<MaterialRegistry>()
    }

    pub fn animation_registry(&self) -> ResRef<AnimationRegistry> {
        self.resource::<AnimationRegistry>()
    }

    pub fn animation_registry_mut(&self) -> ResMut<AnimationRegistry> {
        self.resource_mut::<AnimationRegistry>()
    }

    pub fn model_state(&self) -> ResRef<ModelState> {
        self.resource::<ModelState>()
    }

    pub fn model_state_mut(&self) -> ResMut<ModelState> {
        self.resource_mut::<ModelState>()
    }

    pub fn mesh_assets(&self) -> ResRef<MeshAssets> {
        self.resource::<MeshAssets>()
    }

    pub fn mesh_assets_mut(&self) -> ResMut<MeshAssets> {
        self.resource_mut::<MeshAssets>()
    }

    pub fn node_assets(&self) -> ResRef<NodeAssets> {
        self.resource::<NodeAssets>()
    }

    pub fn node_assets_mut(&self) -> ResMut<NodeAssets> {
        self.resource_mut::<NodeAssets>()
    }

    pub fn grid(&self) -> ResRef<GridData> {
        self.resource::<GridData>()
    }

    pub fn grid_mut(&self) -> ResMut<GridData> {
        self.resource_mut::<GridData>()
    }

    pub fn grid_gizmo(&self) -> ResRef<GridGizmoData> {
        self.resource::<GridGizmoData>()
    }

    pub fn grid_gizmo_mut(&self) -> ResMut<GridGizmoData> {
        self.resource_mut::<GridGizmoData>()
    }

    pub fn light_gizmo(&self) -> ResRef<LightGizmoData> {
        self.resource::<LightGizmoData>()
    }

    pub fn light_gizmo_mut(&self) -> ResMut<LightGizmoData> {
        self.resource_mut::<LightGizmoData>()
    }

    pub fn billboard(&self) -> ResRef<BillboardData> {
        self.resource::<BillboardData>()
    }

    pub fn billboard_mut(&self) -> ResMut<BillboardData> {
        self.resource_mut::<BillboardData>()
    }
}
