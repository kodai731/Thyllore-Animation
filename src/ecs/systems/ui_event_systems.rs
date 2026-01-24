use cgmath::Vector3;

use crate::debugview::RayTracingDebugState;
use crate::ecs::events::{UIEvent, UIEventQueue};
use crate::ecs::systems::camera_systems::{
    camera_move_to_look_at, camera_reset, camera_reset_up,
};
use crate::scene::camera::Camera;
use crate::app::graphics_resource::GraphicsResources;

#[derive(Clone, Debug)]
pub enum DeferredAction {
    LoadModel { path: String },
    TakeScreenshot,
    DebugShadowInfo,
    DebugBillboardDepth,
    DumpDebugInfo,
}

pub fn process_ui_events_system(
    ui_events: &mut UIEventQueue,
    camera: &mut Camera,
    rt_debug_state: &mut RayTracingDebugState,
    graphics_resources: &GraphicsResources,
) -> Vec<DeferredAction> {
    let events: Vec<_> = ui_events.drain().collect();
    process_ui_events_with_events(events, camera, rt_debug_state, graphics_resources)
}

pub fn process_ui_events_with_events(
    events: Vec<UIEvent>,
    camera: &mut Camera,
    rt_debug_state: &mut RayTracingDebugState,
    graphics_resources: &GraphicsResources,
) -> Vec<DeferredAction> {
    let model_bounds = graphics_resources.calculate_model_bounds();
    process_ui_events_with_events_simple(events, camera, rt_debug_state, model_bounds)
}

pub fn process_ui_events_with_events_simple(
    events: Vec<UIEvent>,
    camera: &mut Camera,
    rt_debug_state: &mut RayTracingDebugState,
    model_bounds: Option<(Vector3<f32>, Vector3<f32>, Vector3<f32>)>,
) -> Vec<DeferredAction> {
    let mut deferred = Vec::new();

    for event in events {
        match event {
            UIEvent::ResetCamera => {
                camera_reset(camera);
            }

            UIEvent::ResetCameraUp => {
                camera_reset_up(camera);
            }

            UIEvent::MoveCameraToModel => {
                if let Some((min, max, center)) = model_bounds {
                    let size = max - min;
                    let max_dim = size.x.max(size.y).max(size.z);
                    let distance = max_dim * 2.0;
                    let offset = Vector3::new(0.0, 0.0, distance);
                    camera_move_to_look_at(camera, center, offset);
                    crate::log!(
                        "Moved camera to model: center=({:.2}, {:.2}, {:.2}), distance={:.2}",
                        center.x,
                        center.y,
                        center.z,
                        distance
                    );
                }
            }

            UIEvent::MoveCameraToLightGizmo => {
                let light_pos = rt_debug_state.light_position;
                let offset = Vector3::new(2.0, 2.0, 2.0);
                camera_move_to_look_at(camera, light_pos, offset);
            }

            UIEvent::SetLightPosition(pos) => {
                rt_debug_state.light_position = pos;
            }

            UIEvent::MoveLightToBounds(target) => {
                use crate::app::data::LightMoveTarget;

                if let Some((min, max, _)) = model_bounds {
                    let offset = 2.0;
                    let current = rt_debug_state.light_position;
                    let new_pos = match target {
                        LightMoveTarget::XMin => {
                            Vector3::new(min.x - offset, current.y, current.z)
                        }
                        LightMoveTarget::XMax => {
                            Vector3::new(max.x + offset, current.y, current.z)
                        }
                        LightMoveTarget::YMin => {
                            Vector3::new(current.x, min.y - offset, current.z)
                        }
                        LightMoveTarget::YMax => {
                            Vector3::new(current.x, max.y + offset, current.z)
                        }
                        LightMoveTarget::ZMin => {
                            Vector3::new(current.x, current.y, min.z - offset)
                        }
                        LightMoveTarget::ZMax => {
                            Vector3::new(current.x, current.y, max.z + offset)
                        }
                        LightMoveTarget::None => current,
                    };
                    rt_debug_state.light_position = new_pos;

                    crate::log!(
                        "Light moved to bounds {:?}: ({:.2}, {:.2}, {:.2})",
                        target,
                        new_pos.x,
                        new_pos.y,
                        new_pos.z
                    );
                }
            }

            UIEvent::LoadModel { path } => {
                deferred.push(DeferredAction::LoadModel { path });
            }

            UIEvent::TakeScreenshot => {
                deferred.push(DeferredAction::TakeScreenshot);
            }

            UIEvent::DebugShadowInfo => {
                deferred.push(DeferredAction::DebugShadowInfo);
            }

            UIEvent::DebugBillboardDepth => {
                deferred.push(DeferredAction::DebugBillboardDepth);
            }

            UIEvent::DumpDebugInfo => {
                deferred.push(DeferredAction::DumpDebugInfo);
            }

            UIEvent::SelectEntity(_)
            | UIEvent::DeselectAll
            | UIEvent::ToggleEntitySelection(_)
            | UIEvent::ExpandEntity(_)
            | UIEvent::CollapseEntity(_)
            | UIEvent::SetSearchFilter(_)
            | UIEvent::SetEntityVisible(_, _)
            | UIEvent::SetEntityTranslation(_, _)
            | UIEvent::SetEntityRotation(_, _)
            | UIEvent::SetEntityScale(_, _)
            | UIEvent::RenameEntity(_, _)
            | UIEvent::FocusOnEntity(_) => {}
        }
    }

    deferred
}
