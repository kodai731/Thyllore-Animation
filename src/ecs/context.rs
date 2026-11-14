use glam::Vec3;

use crate::asset::AssetStorage;
use crate::ecs::component::{LineMesh, MeshScale};
use crate::ecs::resource::billboard::BillboardData;
use crate::ecs::resource::gizmo::{
    BoneSelectionState, GridGizmoData, LightGizmoData, TransformGizmoData,
};
use crate::ecs::resource::Camera;
use crate::ecs::resource::HierarchyState;
use crate::ecs::resource::LightState;
use crate::ecs::resource::ObjectIdReadback;
use crate::ecs::resource::PointerCapture;
use crate::ecs::resource::PointerState;
use crate::ecs::resource::TransformGizmoState;

use super::world::{ResMut, ResRef, World};

pub struct EcsContext<'a> {
    pub time: f32,
    pub delta_time: f32,
    pub image_index: usize,
    pub swapchain_extent: (u32, u32),
    pub world: &'a mut World,
    pub assets: &'a mut AssetStorage,
}

impl<'a> EcsContext<'a> {
    pub fn camera(&self) -> ResRef<'_, Camera> {
        self.world.resource::<Camera>()
    }

    pub fn camera_mut(&self) -> ResMut<'_, Camera> {
        self.world.resource_mut::<Camera>()
    }

    pub fn light_state(&self) -> ResRef<'_, LightState> {
        self.world.resource::<LightState>()
    }

    pub fn light_state_mut(&self) -> ResMut<'_, LightState> {
        self.world.resource_mut::<LightState>()
    }

    pub fn camera_position(&self) -> Vec3 {
        use crate::ecs::systems::camera_systems::compute_camera_position;
        compute_camera_position(&self.camera())
    }

    pub fn camera_direction(&self) -> Vec3 {
        use crate::ecs::systems::camera_systems::compute_camera_direction;
        compute_camera_direction(&self.camera())
    }

    pub fn camera_up(&self) -> Vec3 {
        use crate::ecs::systems::camera_systems::compute_camera_up;
        compute_camera_up(&self.camera())
    }

    pub fn light_position(&self) -> Vec3 {
        self.light_state().light_position
    }

    pub fn grid_mesh(&self) -> ResRef<'_, LineMesh> {
        self.world.resource::<LineMesh>()
    }

    pub fn grid_mesh_mut(&self) -> ResMut<'_, LineMesh> {
        self.world.resource_mut::<LineMesh>()
    }

    pub fn grid_scale(&self) -> ResRef<'_, MeshScale> {
        self.world.resource::<MeshScale>()
    }

    pub fn grid_scale_mut(&self) -> ResMut<'_, MeshScale> {
        self.world.resource_mut::<MeshScale>()
    }

    pub fn gizmo(&self) -> ResRef<'_, GridGizmoData> {
        self.world.resource::<GridGizmoData>()
    }

    pub fn gizmo_mut(&self) -> ResMut<'_, GridGizmoData> {
        self.world.resource_mut::<GridGizmoData>()
    }

    pub fn light_gizmo(&self) -> ResRef<'_, LightGizmoData> {
        self.world.resource::<LightGizmoData>()
    }

    pub fn light_gizmo_mut(&self) -> ResMut<'_, LightGizmoData> {
        self.world.resource_mut::<LightGizmoData>()
    }

    pub fn billboard(&self) -> ResRef<'_, BillboardData> {
        self.world.resource::<BillboardData>()
    }

    pub fn billboard_mut(&self) -> ResMut<'_, BillboardData> {
        self.world.resource_mut::<BillboardData>()
    }

    pub fn bone_selection(&self) -> ResRef<'_, BoneSelectionState> {
        self.world.resource::<BoneSelectionState>()
    }

    pub fn bone_selection_mut(&self) -> ResMut<'_, BoneSelectionState> {
        self.world.resource_mut::<BoneSelectionState>()
    }

    pub fn transform_gizmo(&self) -> ResRef<'_, TransformGizmoData> {
        self.world.resource::<TransformGizmoData>()
    }

    pub fn transform_gizmo_mut(&self) -> ResMut<'_, TransformGizmoData> {
        self.world.resource_mut::<TransformGizmoData>()
    }

    pub fn transform_gizmo_state(&self) -> ResRef<'_, TransformGizmoState> {
        self.world.resource::<TransformGizmoState>()
    }

    pub fn transform_gizmo_state_mut(&self) -> ResMut<'_, TransformGizmoState> {
        self.world.resource_mut::<TransformGizmoState>()
    }

    pub fn hierarchy_state(&self) -> ResRef<'_, HierarchyState> {
        self.world.resource::<HierarchyState>()
    }

    pub fn hierarchy_state_mut(&self) -> ResMut<'_, HierarchyState> {
        self.world.resource_mut::<HierarchyState>()
    }

    pub fn object_id_readback(&self) -> ResRef<'_, ObjectIdReadback> {
        self.world.resource::<ObjectIdReadback>()
    }

    pub fn object_id_readback_mut(&self) -> ResMut<'_, ObjectIdReadback> {
        self.world.resource_mut::<ObjectIdReadback>()
    }

    pub fn pointer_state(&self) -> ResRef<'_, PointerState> {
        self.world.resource::<PointerState>()
    }

    pub fn pointer_state_mut(&self) -> ResMut<'_, PointerState> {
        self.world.resource_mut::<PointerState>()
    }

    pub fn pointer_capture(&self) -> ResRef<'_, PointerCapture> {
        self.world.resource::<PointerCapture>()
    }

    pub fn pointer_capture_mut(&self) -> ResMut<'_, PointerCapture> {
        self.world.resource_mut::<PointerCapture>()
    }
}
