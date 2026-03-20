pub mod billboard;
pub mod cleanup;
pub mod color_test_quad;
pub mod data;
pub mod frame_context;
pub mod graphics_resource;
pub mod gui_data;
pub mod init;
pub mod model_loader;
pub mod raytracing;
pub mod render;
pub mod render_context;
pub mod scene_model;
pub mod update;
pub mod util;
pub mod viewport;

pub use frame_context::FrameContext;
pub use render_context::RenderContext;

pub use data::AppData;
pub use gui_data::GUIData;
pub use init::*;

use crate::app::billboard::BillboardData;
use crate::debugview::DebugViewState;
use crate::ecs::resource::gizmo::{GridGizmoData, LightGizmoData, TransformGizmoData};
use crate::ecs::resource::Camera;
use crate::ecs::resource::{GridMeshData, LightState};
use crate::ecs::{ClipLibrary, ModelState, ResMut, ResRef, Resource};
use crate::vulkanr::context::{
    CommandState, FrameSync, PipelineState, RenderTargets, SurfaceState, SwapchainState,
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
    pub last_update_time: f32,
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

    pub fn pipeline_storage(&self) -> &crate::vulkanr::resource::PipelineStorage {
        &self.data.pipeline_storage
    }

    pub fn camera(&self) -> ResRef<Camera> {
        self.resource::<Camera>()
    }

    pub fn light_state(&self) -> ResRef<LightState> {
        self.resource::<LightState>()
    }

    pub fn debug_view_state(&self) -> ResRef<DebugViewState> {
        self.resource::<DebugViewState>()
    }

    pub fn debug_view_state_mut(&self) -> ResMut<DebugViewState> {
        self.resource_mut::<DebugViewState>()
    }

    pub fn clip_library(&self) -> ResRef<ClipLibrary> {
        self.resource::<ClipLibrary>()
    }

    pub fn model_state(&self) -> ResRef<ModelState> {
        self.resource::<ModelState>()
    }

    pub fn grid_mesh(&self) -> ResRef<GridMeshData> {
        self.resource::<GridMeshData>()
    }

    pub fn grid_gizmo(&self) -> ResRef<GridGizmoData> {
        self.resource::<GridGizmoData>()
    }

    pub fn light_gizmo(&self) -> ResRef<LightGizmoData> {
        self.resource::<LightGizmoData>()
    }

    pub fn billboard(&self) -> ResRef<BillboardData> {
        self.resource::<BillboardData>()
    }

    pub fn billboard_mut(&self) -> ResMut<BillboardData> {
        self.resource_mut::<BillboardData>()
    }

    pub fn transform_gizmo(&self) -> ResRef<TransformGizmoData> {
        self.resource::<TransformGizmoData>()
    }
}
