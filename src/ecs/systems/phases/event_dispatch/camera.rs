use cgmath::Vector3;

use crate::asset::AssetStorage;
use crate::ecs::events::UIEvent;
use crate::ecs::resource::Camera;
use crate::ecs::resource::LightState;
use crate::ecs::systems::{camera_move_to_look_at, camera_reset};
use crate::ecs::world::World;

use crate::ecs::resource::AppCommand;

pub fn dispatch_camera_light_debug_events(
    events: &[UIEvent],
    world: &mut World,
    model_bounds: Option<(Vector3<f32>, Vector3<f32>, Vector3<f32>)>,
) -> Vec<AppCommand> {
    let mut camera = world.resource_mut::<Camera>();
    let mut rt_debug = world.resource_mut::<LightState>();
    let mut commands = Vec::new();

    for event in events {
        match event {
            UIEvent::ResetCamera | UIEvent::ResetCameraUp => {
                camera_reset(&mut camera);
            }

            UIEvent::MoveCameraToModel => {
                if let Some((min, max, center)) = model_bounds {
                    let size = max - min;
                    let max_dim = size.x.max(size.y).max(size.z);
                    let distance = max_dim * 2.0;
                    let offset = Vector3::new(0.0, 0.0, distance);
                    camera_move_to_look_at(&mut camera, center, offset);
                    log!(
                        "Moved camera to model: center=({:.2}, {:.2}, {:.2}), distance={:.2}",
                        center.x,
                        center.y,
                        center.z,
                        distance
                    );
                }
            }

            UIEvent::MoveCameraToLightGizmo => {
                let light_pos = rt_debug.light_position;
                let offset = Vector3::new(2.0, 2.0, 2.0);
                camera_move_to_look_at(&mut camera, light_pos, offset);
            }

            UIEvent::SetLightPosition(pos) => {
                rt_debug.light_position = *pos;
            }

            UIEvent::MoveLightToBounds(target) => {
                use crate::ecs::events::light_move_target::LightMoveTarget;

                if let Some((min, max, _)) = model_bounds {
                    let offset = 2.0;
                    let current = rt_debug.light_position;
                    let new_pos = match target {
                        LightMoveTarget::XMin => Vector3::new(min.x - offset, current.y, current.z),
                        LightMoveTarget::XMax => Vector3::new(max.x + offset, current.y, current.z),
                        LightMoveTarget::YMin => Vector3::new(current.x, min.y - offset, current.z),
                        LightMoveTarget::YMax => Vector3::new(current.x, max.y + offset, current.z),
                        LightMoveTarget::ZMin => Vector3::new(current.x, current.y, min.z - offset),
                        LightMoveTarget::ZMax => Vector3::new(current.x, current.y, max.z + offset),
                        LightMoveTarget::None => current,
                    };
                    rt_debug.light_position = new_pos;

                    log!(
                        "Light moved to bounds {:?}: ({:.2}, {:.2}, {:.2})",
                        target,
                        new_pos.x,
                        new_pos.y,
                        new_pos.z
                    );
                }
            }

            UIEvent::LoadModel { path } => {
                commands.push(AppCommand::LoadModel { path: path.clone() });
            }

            UIEvent::LoadModelAdditive { path } => {
                commands.push(AppCommand::LoadModelAdditive { path: path.clone() });
            }

            UIEvent::SpawnDebugPrimitive { kind } => {
                commands.push(AppCommand::SpawnDebugPrimitive { kind: *kind });
            }

            UIEvent::TakeScreenshot => {
                commands.push(AppCommand::TakeScreenshot);
            }

            #[cfg(debug_assertions)]
            UIEvent::DebugShadowInfo => {
                commands.push(AppCommand::DebugShadowInfo);
            }

            #[cfg(debug_assertions)]
            UIEvent::DebugBillboardDepth => {
                commands.push(AppCommand::DebugBillboardDepth);
            }

            UIEvent::DumpDebugInfo => {
                commands.push(AppCommand::DumpDebugInfo);
            }

            UIEvent::DumpAnimationDebug => {
                commands.push(AppCommand::DumpAnimationDebug);
            }

            UIEvent::CaptureNow(capture) => {
                commands.push(AppCommand::CaptureNow(capture.clone()));
            }

            _ => {}
        }
    }

    commands
}
