use std::rc::Rc;

use cgmath::Vector3;

use crate::app::GUIData;
use crate::asset::AssetStorage;
use crate::debugview::gizmo::{GridGizmoData, LightGizmoData};
use crate::debugview::RayTracingDebugState;
use crate::scene::billboard::BillboardData;
use crate::scene::camera::Camera;
use crate::scene::graphics_resource::GraphicsResources;
use crate::scene::grid::GridData;
use crate::scene::raytracing::RayTracingData;
use crate::vulkanr::command::RRCommandPool;
use crate::vulkanr::device::RRDevice;
use crate::vulkanr::vulkan::Instance;

use super::world::{ResMut, ResRef, World};

pub struct FrameContext<'a> {
    pub instance: &'a Instance,
    pub device: &'a RRDevice,
    pub command_pool: Rc<RRCommandPool>,

    pub time: f32,
    pub delta_time: f32,
    pub image_index: usize,
    pub swapchain_extent: (u32, u32),

    pub graphics: &'a mut GraphicsResources,
    pub raytracing: &'a mut RayTracingData,

    pub world: &'a mut World,
    pub assets: &'a AssetStorage,

    pub gui_data: &'a mut GUIData,
}

impl<'a> FrameContext<'a> {
    pub fn camera(&self) -> ResRef<Camera> {
        self.world.resource::<Camera>()
    }

    pub fn camera_mut(&self) -> ResMut<Camera> {
        self.world.resource_mut::<Camera>()
    }

    pub fn rt_debug(&self) -> ResRef<RayTracingDebugState> {
        self.world.resource::<RayTracingDebugState>()
    }

    pub fn rt_debug_mut(&self) -> ResMut<RayTracingDebugState> {
        self.world.resource_mut::<RayTracingDebugState>()
    }

    pub fn camera_position(&self) -> Vector3<f32> {
        self.camera().position
    }

    pub fn camera_direction(&self) -> Vector3<f32> {
        self.camera().direction
    }

    pub fn camera_up(&self) -> Vector3<f32> {
        self.camera().up
    }

    pub fn light_position(&self) -> Vector3<f32> {
        self.rt_debug().light_position
    }

    pub fn grid(&self) -> ResRef<GridData> {
        self.world.resource::<GridData>()
    }

    pub fn grid_mut(&self) -> ResMut<GridData> {
        self.world.resource_mut::<GridData>()
    }

    pub fn gizmo(&self) -> ResRef<GridGizmoData> {
        self.world.resource::<GridGizmoData>()
    }

    pub fn gizmo_mut(&self) -> ResMut<GridGizmoData> {
        self.world.resource_mut::<GridGizmoData>()
    }

    pub fn light_gizmo(&self) -> ResRef<LightGizmoData> {
        self.world.resource::<LightGizmoData>()
    }

    pub fn light_gizmo_mut(&self) -> ResMut<LightGizmoData> {
        self.world.resource_mut::<LightGizmoData>()
    }

    pub fn billboard(&self) -> ResRef<BillboardData> {
        self.world.resource::<BillboardData>()
    }

    pub fn billboard_mut(&self) -> ResMut<BillboardData> {
        self.world.resource_mut::<BillboardData>()
    }
}
