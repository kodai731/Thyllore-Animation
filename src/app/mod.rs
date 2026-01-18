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

use crate::ecs::{AnimationPlayback, ModelInfo, ResMut, ResRef, Resource};
use crate::scene::graphics_resource::GraphicsResources;
use crate::scene::raytracing::RayTracingData;
use crate::scene::Camera;
use crate::scene::Scene;
use crate::debugview::{RayTracingDebugState, DebugViewData};
use crate::platform::ImguiData;
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
    pub scene: Scene,
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

    pub fn model_info(&self) -> ResRef<ModelInfo> {
        self.resource::<ModelInfo>()
    }

    pub fn model_info_mut(&self) -> ResMut<ModelInfo> {
        self.resource_mut::<ModelInfo>()
    }
}
