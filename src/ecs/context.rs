use cgmath::Vector3;

use crate::app::GUIData;
use crate::asset::AssetStorage;
use crate::debugview::gizmo::{
    BoneSelectionState, GridGizmoData, LightGizmoData,
};
use crate::debugview::RayTracingDebugState;
use crate::app::billboard::BillboardData;
use crate::ecs::component::{LineMesh, MeshScale};
use crate::ecs::resource::HierarchyState;
use crate::ecs::resource::ObjectIdReadback;
use crate::ecs::resource::Camera;

use super::world::{ResMut, ResRef, World};

pub struct EcsContext<'a> {
    pub time: f32,
    pub delta_time: f32,
    pub image_index: usize,
    pub swapchain_extent: (u32, u32),
    pub world: &'a mut World,
    pub assets: &'a AssetStorage,
    pub gui_data: &'a mut GUIData,
    pub mesh_positions: Vec<Vector3<f32>>,
}

impl<'a> EcsContext<'a> {
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

    pub fn grid_mesh(&self) -> ResRef<LineMesh> {
        self.world.resource::<LineMesh>()
    }

    pub fn grid_mesh_mut(&self) -> ResMut<LineMesh> {
        self.world.resource_mut::<LineMesh>()
    }

    pub fn grid_scale(&self) -> ResRef<MeshScale> {
        self.world.resource::<MeshScale>()
    }

    pub fn grid_scale_mut(&self) -> ResMut<MeshScale> {
        self.world.resource_mut::<MeshScale>()
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

    pub fn bone_selection(&self) -> ResRef<BoneSelectionState> {
        self.world.resource::<BoneSelectionState>()
    }

    pub fn bone_selection_mut(&self) -> ResMut<BoneSelectionState> {
        self.world.resource_mut::<BoneSelectionState>()
    }

    pub fn hierarchy_state(&self) -> ResRef<HierarchyState> {
        self.world.resource::<HierarchyState>()
    }

    pub fn hierarchy_state_mut(&self) -> ResMut<HierarchyState> {
        self.world.resource_mut::<HierarchyState>()
    }

    pub fn object_id_readback(&self) -> ResRef<ObjectIdReadback> {
        self.world.resource::<ObjectIdReadback>()
    }

    pub fn object_id_readback_mut(&self) -> ResMut<ObjectIdReadback> {
        self.world.resource_mut::<ObjectIdReadback>()
    }
}
