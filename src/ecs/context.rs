use cgmath::Vector3;

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
    pub mesh_positions: Vec<Vector3<f32>>,
}

impl<'a> EcsContext<'a> {
    pub fn camera(&self) -> ResRef<Camera> {
        self.world.resource::<Camera>()
    }

    pub fn camera_mut(&self) -> ResMut<Camera> {
        self.world.resource_mut::<Camera>()
    }

    pub fn light_state(&self) -> ResRef<LightState> {
        self.world.resource::<LightState>()
    }

    pub fn light_state_mut(&self) -> ResMut<LightState> {
        self.world.resource_mut::<LightState>()
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
        self.light_state().light_position
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

    pub fn transform_gizmo(&self) -> ResRef<TransformGizmoData> {
        self.world.resource::<TransformGizmoData>()
    }

    pub fn transform_gizmo_mut(&self) -> ResMut<TransformGizmoData> {
        self.world.resource_mut::<TransformGizmoData>()
    }

    pub fn transform_gizmo_state(&self) -> ResRef<TransformGizmoState> {
        self.world.resource::<TransformGizmoState>()
    }

    pub fn transform_gizmo_state_mut(&self) -> ResMut<TransformGizmoState> {
        self.world.resource_mut::<TransformGizmoState>()
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

    pub fn pointer_state(&self) -> ResRef<PointerState> {
        self.world.resource::<PointerState>()
    }

    pub fn pointer_state_mut(&self) -> ResMut<PointerState> {
        self.world.resource_mut::<PointerState>()
    }

    pub fn pointer_capture(&self) -> ResRef<PointerCapture> {
        self.world.resource::<PointerCapture>()
    }

    pub fn pointer_capture_mut(&self) -> ResMut<PointerCapture> {
        self.world.resource_mut::<PointerCapture>()
    }
}
