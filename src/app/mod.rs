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

use crate::scene::graphics_resource::GraphicsResources;
use crate::scene::world::Resource;
use crate::scene::raytracing::RayTracingData;
use crate::scene::Camera;
use crate::scene::Scene;
use crate::debugview::{RayTracingDebugState, DebugViewData};
use crate::platform::ImguiData;
use crate::vulkanr::context::{
    AnimationPlayback, CameraState, CommandState, DebugState, FrameSync, GpuAssets,
    ImGuiState, PipelineState, RayTracingState, RenderConfig, RenderTargets,
    SurfaceState, SwapchainState,
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
    pub fn resource<R: Resource>(&self) -> &R {
        self.data.ecs_world.resource::<R>()
    }

    pub fn resource_mut<R: Resource>(&mut self) -> &mut R {
        self.data.ecs_world.resource_mut::<R>()
    }

    pub fn get_resource<R: Resource>(&self) -> Option<&R> {
        self.data.ecs_world.get_resource::<R>()
    }

    pub fn frame_sync(&self) -> &FrameSync {
        self.resource::<FrameSync>()
    }

    pub fn frame_sync_mut(&mut self) -> &mut FrameSync {
        self.resource_mut::<FrameSync>()
    }

    pub fn swapchain_state(&self) -> &SwapchainState {
        self.resource::<SwapchainState>()
    }

    pub fn render_targets(&self) -> &RenderTargets {
        self.resource::<RenderTargets>()
    }

    pub fn command_state(&self) -> &CommandState {
        self.resource::<CommandState>()
    }

    pub fn pipeline_state(&self) -> &PipelineState {
        self.resource::<PipelineState>()
    }

    pub fn surface_state(&self) -> &SurfaceState {
        self.resource::<SurfaceState>()
    }

    pub fn graphics_resources(&self) -> &GraphicsResources {
        &self.data.graphics_resources
    }

    pub fn graphics_resources_mut(&mut self) -> &mut GraphicsResources {
        &mut self.data.graphics_resources
    }

    pub fn animation_playback(&self) -> &AnimationPlayback {
        self.resource::<AnimationPlayback>()
    }

    pub fn animation_playback_mut(&mut self) -> &mut AnimationPlayback {
        self.resource_mut::<AnimationPlayback>()
    }

    pub fn camera(&self) -> &Camera {
        &self.data.camera
    }

    pub fn camera_mut(&mut self) -> &mut Camera {
        &mut self.data.camera
    }

    pub fn raytracing(&self) -> &RayTracingData {
        &self.data.raytracing
    }

    pub fn raytracing_mut(&mut self) -> &mut RayTracingData {
        &mut self.data.raytracing
    }

    pub fn rt_debug_state(&self) -> &RayTracingDebugState {
        &self.data.rt_debug_state
    }

    pub fn rt_debug_state_mut(&mut self) -> &mut RayTracingDebugState {
        &mut self.data.rt_debug_state
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
}
