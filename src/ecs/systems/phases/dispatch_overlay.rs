use crate::debugview::gizmo::BoneGizmoData;
use crate::ecs::events::UIEvent;
use crate::ecs::resource::{
    AutoExposure, DepthOfField, GridMeshData, MessageLog, OnionSkinningConfig,
    PhysicalCameraParameters, TransformGizmoState,
};
use crate::ecs::world::World;

pub fn dispatch_overlay_events(events: &[UIEvent], world: &mut World) {
    for event in events {
        match event {
            UIEvent::SetBoneGizmoVisible(visible) => {
                if let Some(mut gizmo) = world.get_resource_mut::<BoneGizmoData>() {
                    gizmo.visible = *visible;
                }
            }
            UIEvent::SetTransformGizmoMode(mode) => {
                if let Some(mut state) = world.get_resource_mut::<TransformGizmoState>() {
                    state.mode = *mode;
                }
            }
            UIEvent::SetTransformGizmoSpace(space) => {
                if let Some(mut state) = world.get_resource_mut::<TransformGizmoState>() {
                    state.coordinate_space = *space;
                }
            }
            UIEvent::UpdateTransformGizmoState(new_state) => {
                if let Some(mut state) = world.get_resource_mut::<TransformGizmoState>() {
                    *state = *new_state.clone();
                }
            }
            UIEvent::UpdateDepthOfField(new_dof) => {
                if let Some(mut dof) = world.get_resource_mut::<DepthOfField>() {
                    *dof = new_dof.clone();
                }
            }
            UIEvent::UpdatePhysicalCamera(new_params) => {
                if let Some(mut params) = world.get_resource_mut::<PhysicalCameraParameters>() {
                    *params = new_params.clone();
                }
            }
            UIEvent::UpdateAutoExposure(new_ae) => {
                if let Some(mut ae) = world.get_resource_mut::<AutoExposure>() {
                    *ae = new_ae.clone();
                }
            }
            UIEvent::UpdateOnionSkinning(new_config) => {
                if let Some(mut config) = world.get_resource_mut::<OnionSkinningConfig>() {
                    *config = new_config.clone();
                }
            }
            UIEvent::SetGridShowYAxis(show) => {
                if let Some(mut grid) = world.get_resource_mut::<GridMeshData>() {
                    grid.show_y_axis_grid = *show;
                }
            }
            UIEvent::ClearMessageLog => {
                if let Some(mut log) = world.get_resource_mut::<MessageLog>() {
                    crate::ecs::systems::message_log_clear_buffer(&mut log);
                }
            }
            _ => {}
        }
    }
}
