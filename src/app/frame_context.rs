use std::rc::Rc;

use anyhow::Result;
use cgmath::{Matrix4, Vector3};

use crate::app::billboard::BillboardData;
use crate::app::graphics_resource::GraphicsResources;
use crate::app::raytracing::RayTracingData;
use crate::asset::AssetStorage;
use crate::debugview::gizmo::{BoneGizmoData, GridGizmoData, LightGizmoData};
use crate::debugview::{GridMeshData, RayTracingDebugState};
use crate::ecs::resource::Camera;
use crate::ecs::world::{ResMut, ResRef, World};
use crate::render::RenderBackend;
use crate::vulkanr::command::RRCommandPool;
use crate::vulkanr::device::RRDevice;
use crate::vulkanr::resource::GpuBufferRegistry;
use crate::vulkanr::vulkan::Instance;
use crate::vulkanr::VulkanBackend;

use super::GUIData;

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
    pub buffer_registry: &'a mut GpuBufferRegistry,

    pub world: &'a mut World,
    pub assets: &'a mut AssetStorage,

    pub gui_data: &'a mut GUIData,
}

impl<'a> FrameContext<'a> {
    pub fn create_backend(&mut self) -> VulkanBackend<'_> {
        VulkanBackend::new(
            self.instance,
            self.device,
            self.command_pool.clone(),
            self.graphics,
            self.raytracing,
            self.buffer_registry,
        )
    }

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
        use crate::ecs::systems::camera_systems::compute_camera_position;
        compute_camera_position(&self.camera())
    }

    pub fn camera_direction(&self) -> Vector3<f32> {
        use crate::ecs::systems::camera_systems::compute_camera_direction;
        compute_camera_direction(&self.camera())
    }

    pub fn camera_up(&self) -> Vector3<f32> {
        use crate::ecs::systems::camera_systems::compute_camera_up;
        compute_camera_up(&self.camera())
    }

    pub fn light_position(&self) -> Vector3<f32> {
        self.rt_debug().light_position
    }

    pub fn grid_mesh(&self) -> ResRef<GridMeshData> {
        self.world.resource::<GridMeshData>()
    }

    pub fn grid_mesh_mut(&self) -> ResMut<GridMeshData> {
        self.world.resource_mut::<GridMeshData>()
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

    pub fn bone_gizmo(&self) -> ResRef<BoneGizmoData> {
        self.world.resource::<BoneGizmoData>()
    }

    pub fn bone_gizmo_mut(&self) -> ResMut<BoneGizmoData> {
        self.world.resource_mut::<BoneGizmoData>()
    }

    pub fn billboard(&self) -> ResRef<BillboardData> {
        self.world.resource::<BillboardData>()
    }

    pub fn billboard_mut(&self) -> ResMut<BillboardData> {
        self.world.resource_mut::<BillboardData>()
    }

    pub unsafe fn update_billboard_ubo_internal(
        &mut self,
        model: Matrix4<f32>,
        view: Matrix4<f32>,
        proj: Matrix4<f32>,
        image_index: usize,
    ) -> Result<()> {
        let mut billboard = self.world.resource_mut::<BillboardData>();
        let mut backend = VulkanBackend::new(
            self.instance,
            self.device,
            self.command_pool.clone(),
            self.graphics,
            self.raytracing,
            self.buffer_registry,
        );
        backend.update_billboard_ubo(&mut billboard, model, view, proj, image_index)
    }
}
